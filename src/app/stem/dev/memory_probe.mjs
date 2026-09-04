#!/usr/bin/env node
// Memory probe harness for doc/decimation/memory-limits.md.
//
// Measures browser-side memory per byte of geojson string ("k") by loading
// synthetic datasets into headless Chromium and reading JS heaps and renderer
// RSS over CDP. Leak mode reloads the app repeatedly and tracks post-GC
// memory to find the session plateau (ratchets) or a true leak.
//
// Run via bazel: bazel run //src/app/stem:memory_probe -- [flags]
// Needs Chromium or Chrome in /Applications (or STEM_PROBE_BROWSER), and
// node >= 22 on the PATH.

import { spawn, spawnSync, execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';

const HELP = `memory_probe: browser memory per geojson byte, over CDP.
Flags:
  --mode sweep|leak|tiles|churn|scrub   default sweep. tiles = per-tile cost probe:
                        sweeps styles x counts x view zooms and reports
                        per-tile wire bytes, slice+fetch latency, and
                        browser memory per visible feature.
  --counts N,N,...      default 10000,30000,100000,300000
                        (leak: 100000; tiles: 100000,300000)
  --styles s,s,...      of points|lines|cmap, default all three (leak: lines)
  --zooms Z,Z,...       tiles-mode view zooms, default 12,10. Synth data
                        always fills the z12 viewport, so z10 packs all N
                        into ~1 tile (zoom-out worst case).
  --shape walk|scatter|clusters   default walk (tiles: scatter)
  --cycles N            leak/churn cycle count, default 8 (churn: 12)
  --tab NAME            churn tabs kind: the tab to cycle against Map,
                        of Log|Places|Stats, default Log. Isolates which
                        page's remount leaks.
  --churn k,k,...       churn-mode kinds, of tabs|pan, default both.
                        In-page lifecycle leak test on a live page:
                        tabs = cycle Log<->Map (map unmount/remount),
                        pan = pan off the data and back (tile
                        load/evict/reload plus a backend query per
                        move). Catches leaks that about:blank cycling
                        (leak mode) frees wholesale and cannot see.
  --steps N             scrub-mode taps per cycle (half forward, half
                        back on the time stepper), default 40
  --step-ms MS          scrub-mode delay between taps, default 60.
                        Scrub mode tracks the backend server process
                        footprint/RSS under rapid time stepping, the
                        iOS per-process-limit jetsam path.
  --decimation temporal|spatial   scrub-mode decimation mode seeded
                        into the style, default temporal (spatial is
                        the app default on phones)
  --out DIR             results dir, default $TMPDIR/stem-memory-probe
  --browser PATH        browser binary (or STEM_PROBE_BROWSER env); by default
                        finds/installs chrome-headless-shell in ~/.cache
  --dev-bin PATH        dev binary to run (or STEM_PROBE_DEV_BIN env);
                        default bazel-bin/src/app/stem/dev. Point at a
                        copied binary to avoid racing an ibazel dev loop:
                        its --//:autoreload=on flavor overwrites bazel-bin
                        with a runfiles build that panics outside the
                        workspace.
  --no-build            skip 'bazel build //src/app/stem:dev'
  --keep                keep scratch dirs, echo server/browser output
  --headful             run the browser with a visible window (debugging)`;

const WORKSPACE = process.env.BUILD_WORKSPACE_DIRECTORY ?? process.cwd();
const SCOPE = '123'; // the insecure local-dev scope/secret
const CACHE_DIR = path.join(os.homedir(), '.cache', 'stem-probe-browsers');

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const mb = (b) => (b / 1e6).toFixed(1);

function parseArgs(argv) {
  const args = {
    mode: 'sweep',
    counts: null,
    styles: null,
    zooms: null,
    shape: null,
    cycles: null,
    churn: null,
    out: path.join(os.tmpdir(), 'stem-memory-probe'),
    build: true,
    keep: false,
    headful: false,
  };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    const next = () => argv[++i];
    if (a === '--mode') args.mode = next();
    else if (a === '--counts') args.counts = next().split(',').map(Number);
    else if (a === '--styles') args.styles = next().split(',');
    else if (a === '--zooms') args.zooms = next().split(',').map(Number);
    else if (a === '--shape') args.shape = next();
    else if (a === '--cycles') args.cycles = Number(next());
    else if (a === '--churn') args.churn = next().split(',');
    else if (a === '--steps') args.steps = Number(next());
    else if (a === '--step-ms') args.stepMs = Number(next());
    else if (a === '--decimation') args.decimation = next();
    else if (a === '--tab') args.tab = next();
    else if (a === '--out') args.out = path.resolve(next());
    else if (a === '--browser') args.browser = next();
    else if (a === '--dev-bin') args.devBin = next();
    else if (a === '--no-build') args.build = false;
    else if (a === '--keep') args.keep = true;
    else if (a === '--headful') args.headful = false || (args.headful = true);
    else if (a === '--help' || a === '-h') { console.log(HELP); process.exit(0); }
    else throw new Error(`unknown flag ${a} (--help for usage)`);
  }
  if (!args.counts) {
    args.counts = ['leak', 'churn', 'scrub'].includes(args.mode) ? [100_000]
      : args.mode === 'tiles' ? [100_000, 300_000]
        : [10_000, 30_000, 100_000, 300_000];
  }
  if (!args.styles) {
    args.styles = args.mode === 'leak' ? ['lines']
      : args.mode === 'churn' || args.mode === 'scrub' ? ['cmap']
        : ['points', 'lines', 'cmap'];
  }
  if (!args.zooms) args.zooms = [12, 10];
  if (!args.shape) args.shape = args.mode === 'tiles' ? 'scatter' : 'walk';
  if (args.cycles == null) args.cycles = args.mode === 'churn' || args.mode === 'scrub' ? 12 : 8;
  if (args.steps == null) args.steps = 40;
  if (args.stepMs == null) args.stepMs = 60;
  if (!args.churn) args.churn = ['tabs', 'pan'];
  if (!args.tab) args.tab = 'Log';
  return args;
}

// --- CDP client (node's built-in WebSocket) ---

class Cdp {
  constructor(url) {
    this.ws = new WebSocket(url);
    this.nextId = 1;
    this.pending = new Map();
    this.listeners = [];
    this.ready = new Promise((resolve, reject) => {
      this.ws.addEventListener('open', () => resolve(), { once: true });
      this.ws.addEventListener('error', () => reject(new Error('CDP socket error')), { once: true });
    });
    this.ws.addEventListener('message', (ev) => {
      const msg = JSON.parse(ev.data);
      if (msg.id !== undefined) {
        const p = this.pending.get(msg.id);
        if (p) {
          this.pending.delete(msg.id);
          if (msg.error) p.reject(new Error(`${p.method}: ${msg.error.message}`));
          else p.resolve(msg.result);
        }
      } else {
        for (const l of this.listeners) l(msg);
      }
    });
  }
  send(method, params = {}, sessionId = undefined) {
    const id = this.nextId++;
    const payload = { id, method, params };
    if (sessionId) payload.sessionId = sessionId;
    this.ws.send(JSON.stringify(payload));
    return new Promise((resolve, reject) => this.pending.set(id, { resolve, reject, method }));
  }
  on(cb) { this.listeners.push(cb); }
  close() { try { this.ws.close(); } catch { /* already closed */ } }
}

// --- browser + server process management ---

/// Locate a CDP-capable browser: chrome-headless-shell is the reliable
/// choice (the ungoogled-chromium in /Applications hangs in headless mode).
/// Installs one into ~/.cache/stem-probe-browsers if nothing is found.
function findBrowser(explicit) {
  const scanCache = () => {
    const bins = [];
    const root = path.join(CACHE_DIR, 'chrome-headless-shell');
    let versions = [];
    try { versions = fs.readdirSync(root); } catch { return bins; }
    for (const ver of versions) {
      let plats = [];
      try { plats = fs.readdirSync(path.join(root, ver)); } catch { continue; }
      for (const plat of plats) {
        bins.push(path.join(root, ver, plat, 'chrome-headless-shell'));
      }
    }
    return bins;
  };
  const candidates = () => [
    explicit,
    process.env.STEM_PROBE_BROWSER,
    ...scanCache(),
    '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome',
  ].filter(Boolean);
  let found = candidates().find((p) => fs.existsSync(p));
  if (found) return found;
  console.error(`no browser found; installing chrome-headless-shell into ${CACHE_DIR}`);
  const r = spawnSync('npx', [
    '--yes', '@puppeteer/browsers', 'install',
    'chrome-headless-shell@stable', '--path', CACHE_DIR,
  ], { stdio: ['ignore', 'inherit', 'inherit'] });
  if (r.status !== 0) throw new Error('browser install failed; set STEM_PROBE_BROWSER');
  found = candidates().find((p) => fs.existsSync(p));
  if (!found) throw new Error('browser still not found after install');
  return found;
}

async function launchBrowser(chrome, profileDir, headful) {
  fs.mkdirSync(profileDir, { recursive: true });
  const isShell = path.basename(chrome).includes('chrome-headless-shell');
  const flags = [
    `--user-data-dir=${profileDir}`,
    '--remote-debugging-port=0',
    '--no-first-run',
    '--no-default-browser-check',
    '--disable-background-networking',
    '--disable-component-update',
    '--disable-sync',
    '--window-size=1200,800',
    '--hide-scrollbars',
  ];
  // headless shell is inherently headless (and ignores --headful)
  if (!headful && !isShell) flags.unshift('--headless=new');
  flags.push('about:blank');
  const proc = spawn(chrome, flags, { stdio: ['ignore', 'ignore', 'pipe'] });
  let stderr = '';
  proc.stderr.on('data', (d) => { stderr += d.toString(); });
  const portFile = path.join(profileDir, 'DevToolsActivePort');
  const deadline = Date.now() + 20_000;
  let wsUrl;
  while (Date.now() < deadline) {
    // stderr announcement (headless shell) or the port file (full browser)
    const m = stderr.match(/DevTools listening on (ws:\/\/\S+)/);
    if (m) { wsUrl = m[1]; break; }
    try {
      const [p, wp] = fs.readFileSync(portFile, 'utf8').trim().split('\n');
      if (p && wp) { wsUrl = `ws://127.0.0.1:${Number(p)}${wp}`; break; }
    } catch { /* not there yet */ }
    await sleep(100);
  }
  if (!wsUrl) {
    proc.kill('SIGKILL');
    throw new Error(`browser debugger did not start; stderr: ${stderr.slice(-1500)}`);
  }
  const cdp = new Cdp(wsUrl);
  await cdp.ready;
  return { proc, cdp };
}

async function attachPage(cdp) {
  const { targetInfos } = await cdp.send('Target.getTargets');
  const page = targetInfos.find((t) => t.type === 'page');
  if (!page) throw new Error('no page target');
  const { sessionId } = await cdp.send('Target.attachToTarget', {
    targetId: page.targetId, flatten: true,
  });
  return sessionId;
}

function trackWorkers(cdp, workers) {
  cdp.on((msg) => {
    if (msg.method === 'Target.attachedToTarget') {
      const { sessionId, targetInfo } = msg.params;
      if (targetInfo.type === 'worker') {
        workers.set(targetInfo.targetId, sessionId);
        // maplibre fetches geojson from its worker, so watch its network too
        cdp.send('Network.enable', {}, sessionId).catch(() => {});
        cdp.send('Runtime.enable', {}, sessionId).catch(() => {});
      }
    } else if (msg.method === 'Target.detachedFromTarget') {
      for (const [tid, sid] of workers) {
        if (sid === msg.params.sessionId) {
          workers.delete(tid);
          workers.deaths = (workers.deaths ?? 0) + 1;
        }
      }
    }
  });
}

async function startServer(devBin, runDir, { count, shape, style, viewZoom, decimation }, echo) {
  fs.mkdirSync(runDir, { recursive: true });
  const args = ['--synth', String(count), '--shape', shape, '--style', style, '--port', '0'];
  if (viewZoom !== undefined) args.push('--view-zoom', String(viewZoom));
  if (decimation !== undefined) args.push('--decimation', decimation);
  const proc = spawn(devBin, args, { cwd: runDir, stdio: ['ignore', 'pipe', 'pipe'] });
  let buf = '';
  const port = await new Promise((resolve, reject) => {
    const to = setTimeout(() => reject(new Error('server not ready after 180s')), 180_000);
    proc.stdout.on('data', (d) => {
      buf += d.toString();
      if (echo) process.stderr.write(d);
      const m = buf.match(/PROBE_READY port=(\d+)/);
      if (m) { clearTimeout(to); resolve(Number(m[1])); }
    });
    proc.stderr.on('data', (d) => { if (echo) process.stderr.write(d); });
    proc.on('exit', (code) => {
      clearTimeout(to);
      reject(new Error(`server exited early (${code}): ${buf.slice(-2000)}`));
    });
  });
  return { proc, port };
}

// --- measurement ---

/// RSS (bytes) of a single process (the dev server).
function processRss(pid) {
  try {
    const out = execFileSync('ps', ['-o', 'rss=', '-p', String(pid)], { encoding: 'utf8' });
    return Number(out.trim()) * 1024 || 0;
  } catch { return 0; }
}

/// Physical footprint (bytes) of a process: dirty + compressed, the number
/// the iOS per-process jetsam limit is enforced against. ps RSS also counts
/// clean reusable pages the allocator retains, which jetsam ignores. Slow
/// (~1 s), so sampled per cycle, not per tap.
function processFootprint(pid) {
  try {
    const out = execFileSync('vmmap', ['--summary', String(pid)], {
      encoding: 'utf8', maxBuffer: 16e6,
    });
    const m = out.match(/Physical footprint:\s+([\d.]+)([KMG])/);
    if (!m) return 0;
    const scale = { K: 1e3, M: 1e6, G: 1e9 }[m[2]];
    return Number(m[1]) * scale;
  } catch { return 0; }
}

/// Sum of RSS (bytes) of renderer processes descended from the browser.
function rendererRss(browserPid) {
  let out;
  try {
    out = execFileSync('ps', ['-axo', 'pid=,ppid=,rss=,command='], {
      encoding: 'utf8', maxBuffer: 64e6,
    });
  } catch { return 0; }
  const rows = [];
  for (const line of out.split('\n')) {
    const m = line.match(/^\s*(\d+)\s+(\d+)\s+(\d+)\s+(.*)$/);
    if (m) rows.push({ pid: +m[1], ppid: +m[2], rss: +m[3], cmd: m[4] });
  }
  const children = new Map();
  for (const r of rows) {
    if (!children.has(r.ppid)) children.set(r.ppid, []);
    children.get(r.ppid).push(r);
  }
  let sum = 0;
  const stack = [browserPid];
  const seen = new Set();
  while (stack.length) {
    const pid = stack.pop();
    if (seen.has(pid)) continue;
    seen.add(pid);
    for (const c of children.get(pid) ?? []) {
      if (c.cmd.includes('--type=renderer')) sum += c.rss;
      stack.push(c.pid);
    }
  }
  return sum * 1024;
}

async function evalInPage(cdp, sessionId, expression) {
  const { result } = await cdp.send(
    'Runtime.evaluate', { expression, returnByValue: true }, sessionId,
  );
  return result?.value;
}

/// GC every session, then read used JS heap for the page and workers.
async function gcAndHeaps(cdp, pageSession, workers) {
  const sessions = [pageSession, ...workers.values()];
  for (const sid of sessions) {
    try {
      await cdp.send('HeapProfiler.enable', {}, sid);
      await cdp.send('HeapProfiler.collectGarbage', {}, sid);
    } catch { /* session may be gone */ }
  }
  await sleep(500);
  const heaps = { page: 0, workers: 0 };
  for (const sid of sessions) {
    try {
      const { usedSize } = await cdp.send('Runtime.getHeapUsage', {}, sid);
      if (sid === pageSession) heaps.page += usedSize;
      else heaps.workers += usedSize;
    } catch { /* session may be gone */ }
  }
  return heaps;
}

/// Navigate to the app and wait until the data has loaded and the map has
/// gone idle. Returns response sizes in bytes, per-tile detail rows
/// ({z, x, y, bytes, ms} per .mvt request), and firstIdleMs (navigate to
/// first map-idle, a coarse jank/latency proxy).
async function loadApp(cdp, pageSession, port, isAborted = () => false, timeoutMs = 480_000) {
  const sizes = { points: 0, lines: 0, mvt: 0, tiles: [], firstIdleMs: null };
  const reqs = new Map();
  let lastGeojsonAt = 0;
  cdp.on((msg) => {
    // data requests come from the maplibre worker session, so accept
    // network events from every session
    if (msg.method === 'Network.requestWillBeSent') {
      const url = msg.params.request.url;
      const kind = url.endsWith('points.geojson') ? 'points'
        : url.endsWith('lines.geojson') ? 'lines'
        : url.includes('/tiles/') && url.includes('.mvt') ? 'mvt' : null;
      // timestamp is monotonic seconds within the sending process
      if (kind) reqs.set(msg.params.requestId, { kind, url, start: msg.params.timestamp });
    } else if (msg.method === 'Network.loadingFinished') {
      const req = reqs.get(msg.params.requestId);
      if (req) {
        if (req.kind === 'mvt') {
          sizes.mvt += msg.params.encodedDataLength;
          const m = req.url.match(/tiles\/(\d+)\/(\d+)\/(\d+)\.mvt/);
          sizes.tiles.push({
            z: m ? +m[1] : -1,
            x: m ? +m[2] : -1,
            y: m ? +m[3] : -1,
            bytes: msg.params.encodedDataLength,
            ms: (msg.params.timestamp - req.start) * 1000,
          });
        } else {
          sizes[req.kind] = msg.params.encodedDataLength;
        }
        lastGeojsonAt = Date.now();
      }
    }
  });
  const navStart = Date.now();
  await cdp.send('Page.navigate', { url: `http://127.0.0.1:${port}/${SCOPE}/` }, pageSession);
  const deadline = Date.now() + timeoutMs;
  let prevIdle = -1;
  while (Date.now() < deadline) {
    await sleep(500);
    if (isAborted()) {
      console.error('warning: renderer crashed during load');
      return sizes;
    }
    const idle = (await evalInPage(cdp, pageSession, 'window.__stem_idle_count || 0')) ?? 0;
    if (idle > 0 && sizes.firstIdleMs === null) sizes.firstIdleMs = Date.now() - navStart;
    // Empty tiles (~82 B) arrive while a large-N query/tileset build is
    // still running and the data lands later via the expired-reload
    // refresh, so wait for a data-bearing tile, not just any mvt bytes.
    if (!((sizes.points && sizes.lines)
      || sizes.tiles.some((t) => t.bytes > 256))) continue;
    if (Date.now() - lastGeojsonAt < 4_000) continue;
    if (idle > 0 && idle === prevIdle) return sizes;
    prevIdle = idle;
  }
  console.error('warning: load wait timed out, proceeding with what arrived');
  return sizes;
}

/// Track renderer crashes on the page session. Returns a function that
/// reports whether a crash has happened.
async function trackCrashes(cdp, pageSession) {
  let crashed = false;
  await cdp.send('Inspector.enable', {}, pageSession).catch(() => {});
  cdp.on((m) => {
    if (m.method === 'Inspector.targetCrashed' && m.sessionId === pageSession) {
      crashed = true;
    }
  });
  return () => crashed;
}

// --- sweep mode ---

async function sweepRun(ctx, count, style, shape) {
  const runDir = path.join(ctx.scratch, `run-${count}-${style}-${shape}`);
  const server = await startServer(ctx.devBin, runDir, { count, shape, style }, ctx.args.keep);
  const { proc: browser, cdp } = await launchBrowser(
    ctx.browserBin, path.join(runDir, 'chrome-profile'), ctx.args.headful,
  );
  try {
    const pageSession = await attachPage(cdp);
    const workers = new Map();
    trackWorkers(cdp, workers);
    await cdp.send('Network.enable', {}, pageSession);
    await cdp.send('Page.enable', {}, pageSession);
    await cdp.send('Runtime.enable', {}, pageSession);
    await cdp.send('Target.setAutoAttach', {
      autoAttach: true, waitForDebuggerOnStart: false, flatten: true,
    }, pageSession);
    const hasCrashed = await trackCrashes(cdp, pageSession);
    const heapsBaseline = await gcAndHeaps(cdp, pageSession, workers);
    const rssBaseline = rendererRss(browser.pid);
    let rssPeak = rssBaseline;
    const sampler = setInterval(() => {
      rssPeak = Math.max(rssPeak, rendererRss(browser.pid));
    }, 150);
    workers.deaths = 0; // deaths past this point mean memory exhaustion
    const sizes = await loadApp(cdp, pageSession, server.port, hasCrashed);
    // The map can go idle while the worker is still indexing (large N), so
    // keep sampling until RSS is stable for two consecutive 2s intervals.
    let prevRss = rendererRss(browser.pid);
    let stable = 0;
    const settleDeadline = Date.now() + 60_000;
    while (stable < 2 && Date.now() < settleDeadline) {
      await sleep(2_000);
      const rss = rendererRss(browser.pid);
      if (Math.abs(rss - prevRss) < 0.01 * Math.max(rss, 1)) stable += 1;
      else stable = 0;
      prevRss = rss;
    }
    clearInterval(sampler);
    rssPeak = Math.max(rssPeak, rendererRss(browser.pid));
    const heaps = await gcAndHeaps(cdp, pageSession, workers);
    const rssSettled = rendererRss(browser.pid);
    return {
      count, style, shape,
      crashed: hasCrashed(),
      workerDied: (workers.deaths ?? 0) > 0,
      pointsBytes: sizes.points,
      linesBytes: sizes.lines,
      mvtBytes: sizes.mvt,
      totalBytes: sizes.points + sizes.lines + sizes.mvt,
      heapPageBaseline: heapsBaseline.page,
      heapPage: heaps.page,
      heapWorkers: heaps.workers,
      rssBaseline, rssPeak, rssSettled,
    };
  } finally {
    cdp.close();
    browser.kill('SIGKILL');
    server.proc.kill('SIGTERM');
    await sleep(300);
    if (!ctx.args.keep) fs.rmSync(runDir, { recursive: true, force: true });
  }
}

function slope(points) {
  const n = points.length;
  if (n < 2) return NaN;
  const mx = points.reduce((s, p) => s + p[0], 0) / n;
  const my = points.reduce((s, p) => s + p[1], 0) / n;
  let num = 0;
  let den = 0;
  for (const [x, y] of points) {
    num += (x - mx) * (y - my);
    den += (x - mx) ** 2;
  }
  return den === 0 ? NaN : num / den;
}

function sweepReport(rows) {
  let md = '# Memory probe sweep\n\n';
  md += '| style | N | points MB | lines MB | peak RSS d MB | settled RSS d MB | page heap MB | worker heap MB |\n';
  md += '|---|---|---|---|---|---|---|---|\n';
  for (const r of rows) {
    const label = r.crashed ? `${r.style} (CRASHED)`
      : r.workerDied ? `${r.style} (WORKER DIED)` : r.style;
    md += `| ${label} | ${r.count} | ${mb(r.pointsBytes)} | ${mb(r.linesBytes)} `
      + `| ${mb(r.rssPeak - r.rssBaseline)} | ${mb(r.rssSettled - r.rssBaseline)} `
      + `| ${mb(r.heapPage)} | ${mb(r.heapWorkers)} |\n`;
  }
  md += '\n## Per-style rates (regression across N)\n\n';
  md += 'k = bytes of browser memory per byte of geojson string. Runs where\n';
  md += 'the renderer crashed or the maplibre worker died (memory exhaustion;\n';
  md += 'their byte counts and heaps are invalid) are excluded.\n\n';
  md += '| style | k (peak RSS) | k (settled RSS) | k (post-GC heap) | str bytes/pt | str bytes/line |\n';
  md += '|---|---|---|---|---|---|\n';
  const ks = {};
  for (const s of [...new Set(rows.map((r) => r.style))]) {
    const rs = rows.filter((r) => r.style === s && !r.crashed && !r.workerDied);
    if (!rs.length) continue;
    const kPeak = slope(rs.map((r) => [r.totalBytes, r.rssPeak]));
    const kSettled = slope(rs.map((r) => [r.totalBytes, r.rssSettled]));
    const kHeap = slope(rs.map((r) => [r.totalBytes, r.heapPage + r.heapWorkers]));
    const big = rs.reduce((a, b) => (a.count > b.count ? a : b));
    const rPt = big.pointsBytes / big.count;
    const rLine = big.linesBytes > 0 && big.count > 1 ? big.linesBytes / (big.count - 1) : 0;
    ks[s] = { kPeak, kSettled, kHeap, rPt, rLine };
    md += `| ${s} | ${kPeak.toFixed(1)} | ${kSettled.toFixed(1)} | ${kHeap.toFixed(1)} `
      + `| ${rPt.toFixed(0)} | ${rLine.toFixed(0)} |\n`;
  }
  return { md, ks };
}

// --- tiles mode (per-tile cost probe) ---

/// One run at a fixed (N, style, view zoom): load, settle, and report
/// per-tile wire bytes and latency alongside the usual memory signals.
/// Synth data always fills the z12 viewport, so features-per-tile is set
/// by N and zoom together (z10 packs all N into ~1 tile).
async function tilesRun(ctx, count, style, viewZoom) {
  const runDir = path.join(ctx.scratch, `tiles-${count}-${style}-z${viewZoom}`);
  const server = await startServer(
    ctx.devBin, runDir,
    { count, shape: ctx.args.shape, style, viewZoom }, ctx.args.keep,
  );
  const { proc: browser, cdp } = await launchBrowser(
    ctx.browserBin, path.join(runDir, 'chrome-profile'), ctx.args.headful,
  );
  try {
    const pageSession = await attachPage(cdp);
    const workers = new Map();
    trackWorkers(cdp, workers);
    await cdp.send('Network.enable', {}, pageSession);
    await cdp.send('Page.enable', {}, pageSession);
    await cdp.send('Runtime.enable', {}, pageSession);
    await cdp.send('Target.setAutoAttach', {
      autoAttach: true, waitForDebuggerOnStart: false, flatten: true,
    }, pageSession);
    const hasCrashed = await trackCrashes(cdp, pageSession);
    const rssBaseline = rendererRss(browser.pid);
    let rssPeak = rssBaseline;
    const sampler = setInterval(() => {
      rssPeak = Math.max(rssPeak, rendererRss(browser.pid));
    }, 150);
    workers.deaths = 0;
    const sizes = await loadApp(cdp, pageSession, server.port, hasCrashed);
    clearInterval(sampler);
    rssPeak = Math.max(rssPeak, rendererRss(browser.pid));
    const heaps = await gcAndHeaps(cdp, pageSession, workers);
    const rssSettled = rendererRss(browser.pid);
    // nonEmpty: tiles that actually hold data; the empty fringe would
    // drag down the medians without informing the cap
    const nonEmpty = sizes.tiles.filter((t) => t.bytes > 256);
    const bytesArr = nonEmpty.map((t) => t.bytes).sort((a, b) => a - b);
    const msArr = nonEmpty.map((t) => t.ms).sort((a, b) => a - b);
    const med = (arr) => (arr.length ? arr[Math.floor(arr.length / 2)] : 0);
    const maxTileBytes = bytesArr.at(-1) ?? 0;
    // wire bytes are ~proportional to features, so apportion N by bytes
    const estMaxTileFeatures = sizes.mvt > 0
      ? Math.round((count * maxTileBytes) / sizes.mvt) : 0;
    return {
      count, style, viewZoom, shape: ctx.args.shape,
      crashed: hasCrashed(),
      workerDied: (workers.deaths ?? 0) > 0,
      mvtBytes: sizes.mvt,
      tileCount: sizes.tiles.length,
      dataTileCount: nonEmpty.length,
      maxTileBytes,
      medTileBytes: med(bytesArr),
      maxTileMs: msArr.at(-1) ?? 0,
      medTileMs: med(msArr),
      estMaxTileFeatures,
      firstIdleMs: sizes.firstIdleMs,
      heapPage: heaps.page,
      heapWorkers: heaps.workers,
      rssBaseline, rssPeak, rssSettled,
      tiles: nonEmpty,
    };
  } finally {
    cdp.close();
    browser.kill('SIGKILL');
    server.proc.kill('SIGTERM');
    await sleep(300);
    if (!ctx.args.keep) fs.rmSync(runDir, { recursive: true, force: true });
  }
}

function tilesReport(rows) {
  const kb = (b) => (b / 1e3).toFixed(1);
  let md = '# Per-tile cost probe\n\n';
  md += 'Synth data fills the z12 viewport; view zoom repacks the same N\n';
  md += 'into fewer, denser tiles. est max feat/tile apportions N by tile\n';
  md += 'wire bytes. Tile ms is request-to-finish seen from the maplibre\n';
  md += 'worker (backend slice CPU + queue + transfer on localhost).\n\n';
  md += '| style | N | zoom | data tiles | mvt MB | max tile KB | est max feat/tile '
    + '| max tile ms | med tile ms | first idle ms | worker heap MB | page heap MB | settled RSS d MB |\n';
  md += '|---|---|---|---|---|---|---|---|---|---|---|---|\n';
  for (const r of rows) {
    const label = r.crashed ? `${r.style} (CRASHED)`
      : r.workerDied ? `${r.style} (WORKER DIED)` : r.style;
    md += `| ${label} | ${r.count} | ${r.viewZoom} | ${r.dataTileCount} `
      + `| ${mb(r.mvtBytes)} | ${kb(r.maxTileBytes)} | ${r.estMaxTileFeatures} `
      + `| ${r.maxTileMs.toFixed(0)} | ${r.medTileMs.toFixed(0)} `
      + `| ${r.firstIdleMs ?? '?'} | ${mb(r.heapWorkers)} | ${mb(r.heapPage)} `
      + `| ${mb(r.rssSettled - r.rssBaseline)} |\n`;
  }
  md += '\n## Per-style rates (regression across runs)\n\n';
  md += 'wire B/feat from the densest run (in-tile rate, free of cross-tile\n';
  md += 'segment duplication); heap and RSS per visible feature regress on N\n';
  md += '(all N visible in every run; needs >=2 distinct N); tile latency\n';
  md += 'regresses on est max feat/tile across the zoom x N grid.\n\n';
  md += '| style | wire B/feat | worker heap B/feat | settled RSS B/feat | tile ms per 10k feat |\n';
  md += '|---|---|---|---|---|\n';
  const fmt = (v, scale = 1) => (Number.isFinite(v) ? (v * scale).toFixed(1) : 'n/a');
  const rates = {};
  for (const s of [...new Set(rows.map((r) => r.style))]) {
    const rs = rows.filter((r) => r.style === s && !r.crashed && !r.workerDied);
    if (!rs.length) continue;
    const densest = rs.reduce((a, b) => (a.estMaxTileFeatures > b.estMaxTileFeatures ? a : b));
    const wirePerFeat = densest.mvtBytes / densest.count;
    const heapPerFeat = slope(rs.map((r) => [r.count, r.heapWorkers + r.heapPage]));
    const rssPerFeat = slope(rs.map((r) => [r.count, r.rssSettled - r.rssBaseline]));
    const msPerFeat = slope(rs.map((r) => [r.estMaxTileFeatures, r.maxTileMs]));
    rates[s] = { wirePerFeat, heapPerFeat, rssPerFeat, msPerFeat };
    md += `| ${s} | ${fmt(wirePerFeat)} | ${fmt(heapPerFeat)} `
      + `| ${fmt(rssPerFeat)} | ${fmt(msPerFeat, 10_000)} |\n`;
  }
  return { md, rates };
}

// --- churn mode (in-page lifecycle leaks: tab cycling and pan churn) ---

/// Wait until the map-idle counter exceeds `prev`; returns the new count.
async function waitIdleAbove(cdp, pageSession, prev, timeoutMs = 60_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const idle = (await evalInPage(cdp, pageSession, 'window.__stem_idle_count || 0')) ?? 0;
    if (idle > prev) return idle;
    await sleep(300);
  }
  console.error('warning: idle wait timed out');
  return prev;
}

/// Load once, then repeatedly churn the live page: kind 'tabs' cycles
/// Log <-> Map (unmounting and recreating the map component), kind 'pan'
/// pans off the data and back (tile load/evict/reload plus a backend
/// query per move). Measures post-GC heaps and RSS per cycle. Unlike
/// leak mode's about:blank cycles, nothing is freed wholesale, so app-
/// level cleanup leaks accumulate visibly.
async function churnMode(ctx, kind, count, style) {
  const runDir = path.join(ctx.scratch, `churn-${kind}-${count}-${style}`);
  const server = await startServer(ctx.devBin, runDir, { count, shape: ctx.args.shape, style }, ctx.args.keep);
  const { proc: browser, cdp } = await launchBrowser(
    ctx.browserBin, path.join(runDir, 'chrome-profile'), ctx.args.headful,
  );
  const rows = [];
  try {
    const pageSession = await attachPage(cdp);
    const workers = new Map();
    trackWorkers(cdp, workers);
    await cdp.send('Network.enable', {}, pageSession);
    await cdp.send('Page.enable', {}, pageSession);
    await cdp.send('Runtime.enable', {}, pageSession);
    await cdp.send('Target.setAutoAttach', {
      autoAttach: true, waitForDebuggerOnStart: false, flatten: true,
    }, pageSession);
    const hasCrashed = await trackCrashes(cdp, pageSession);
    await loadApp(cdp, pageSession, server.port, hasCrashed);
    const evalJs = (expr) => evalInPage(cdp, pageSession, expr);
    const tab = ctx.args.tab;
    if (kind === 'tabs' && !(await evalJs(`!!document.getElementById("${tab}")`))) {
      throw new Error(`tab button #${tab} not found`);
    }
    if (kind === 'pan' && !(await evalJs('!!window.__stem_map'))) {
      throw new Error('window.__stem_map missing (front built with the probe hook?)');
    }
    for (let cycle = 0; cycle <= ctx.args.cycles; cycle++) {
      if (cycle > 0) { // cycle 0 is the post-load baseline
        let idle = (await evalJs('window.__stem_idle_count || 0')) ?? 0;
        if (kind === 'tabs') {
          await evalJs(`document.getElementById("${tab}").click()`);
          await sleep(1_500); // let the map unmount settle
          await evalJs('document.getElementById("Map").click()');
          idle = await waitIdleAbove(cdp, pageSession, idle);
        } else {
          for (const dx of [1_800, -1_800]) {
            // void: panBy returns the (unserializable) Map object
            await evalJs(`void window.__stem_map.panBy([${dx}, 0], {duration: 0})`);
            idle = await waitIdleAbove(cdp, pageSession, idle);
          }
        }
        await sleep(500);
      }
      const heaps = await gcAndHeaps(cdp, pageSession, workers);
      const rss = rendererRss(browser.pid);
      const row = {
        cycle,
        heapPage: heaps.page,
        heapWorkers: heaps.workers,
        rss,
        crashed: hasCrashed(),
      };
      rows.push(row);
      console.error(`churn(${kind}) cycle ${cycle}: rss ${mb(rss)} MB, `
        + `page heap ${mb(heaps.page)} MB, worker heap ${mb(heaps.workers)} MB`);
      if (row.crashed) {
        console.error('renderer crashed; stopping churn cycles');
        break;
      }
    }
  } finally {
    cdp.close();
    browser.kill('SIGKILL');
    server.proc.kill('SIGTERM');
    await sleep(300);
    if (!ctx.args.keep) fs.rmSync(runDir, { recursive: true, force: true });
  }
  return rows;
}

function churnReport(rows, kind, count, style) {
  let md = `# Memory probe churn mode (${kind}, N=${count}, style=${style})\n\n`;
  md += kind === 'tabs'
    ? 'Each cycle switches Log -> Map on a live page, unmounting and\nrecreating the map component. '
    : 'Each cycle pans off the data and back on a live page, churning\ntile load/evict/reload with a backend query per move. ';
  md += 'Post-GC values; cycle 0 is the\npost-load baseline. A plateau is ratchet behavior; a steady late\nslope is an in-page leak that leak mode (about:blank cycling, which\nfrees the page wholesale) cannot see.\n\n';
  md += '| cycle | RSS MB | page heap MB | worker heap MB |\n';
  md += '|---|---|---|---|\n';
  for (const r of rows) {
    md += `| ${r.cycle}${r.crashed ? ' (CRASHED)' : ''} | ${mb(r.rss)} `
      + `| ${mb(r.heapPage)} | ${mb(r.heapWorkers)} |\n`;
  }
  const later = rows.slice(Math.floor(rows.length / 2));
  const s = (f) => mb(slope(later.map((r) => [r.cycle, f(r)])));
  md += `\nLate-cycle trend: RSS ${s((r) => r.rss)} MB/cycle, `
    + `page heap ${s((r) => r.heapPage)} MB/cycle, `
    + `worker heap ${s((r) => r.heapWorkers)} MB/cycle.\n`;
  return md;
}

// --- scrub mode (backend RSS under rapid time stepping) ---

/// Load once, open the TimeRange settings tab, then per cycle fire a
/// burst of rapid taps on the time stepper's scrub cells (steps/2
/// forward then steps/2 back, one tap per step-ms). Each tap steps the
/// time range through the frontend's real dispatch path, so the backend
/// requeries and rebuilds tilesets under the same load as scrubbing on
/// the phone. Unlike the other modes this tracks the SERVER process RSS,
/// where the iOS per-process-limit jetsam at ~3.5 GB fires.
async function scrubMode(ctx, count, style) {
  const runDir = path.join(ctx.scratch, `scrub-${count}-${style}`);
  const server = await startServer(
    ctx.devBin, runDir,
    { count, shape: ctx.args.shape, style, decimation: ctx.args.decimation },
    ctx.args.keep,
  );
  const { proc: browser, cdp } = await launchBrowser(
    ctx.browserBin, path.join(runDir, 'chrome-profile'), ctx.args.headful,
  );
  const rows = [];
  const tap = (id) => `(() => {
    const el = document.getElementById(${JSON.stringify(id)});
    if (!el) return false;
    const o = { bubbles: true, cancelable: true, pointerId: 1,
      isPrimary: true, clientX: 50, clientY: 5 };
    el.dispatchEvent(new PointerEvent('pointerdown', { ...o, buttons: 1 }));
    el.dispatchEvent(new PointerEvent('pointerup', { ...o, buttons: 0 }));
    return true;
  })()`;
  try {
    const pageSession = await attachPage(cdp);
    const workers = new Map();
    trackWorkers(cdp, workers);
    await cdp.send('Network.enable', {}, pageSession);
    await cdp.send('Page.enable', {}, pageSession);
    await cdp.send('Runtime.enable', {}, pageSession);
    await cdp.send('Target.setAutoAttach', {
      autoAttach: true, waitForDebuggerOnStart: false, flatten: true,
    }, pageSession);
    const hasCrashed = await trackCrashes(cdp, pageSession);
    await loadApp(cdp, pageSession, server.port, hasCrashed);
    const evalJs = (expr) => evalInPage(cdp, pageSession, expr);
    await evalJs('document.getElementById("time_range_btn").click()');
    await sleep(500);
    if (!(await evalJs('!!document.getElementById("time_scrub_fwd")'))) {
      throw new Error('#time_scrub_fwd not found after opening the TimeRange tab');
    }
    for (let cycle = 0; cycle <= ctx.args.cycles; cycle++) {
      let serverPeak = 0;
      if (cycle > 0) { // cycle 0 is the post-load baseline
        let idle = (await evalJs('window.__stem_idle_count || 0')) ?? 0;
        const half = Math.floor(ctx.args.steps / 2);
        for (let i = 0; i < ctx.args.steps; i++) {
          const id = i < half ? 'time_scrub_fwd' : 'time_scrub_back';
          if (!(await evalJs(tap(id)))) throw new Error(`#${id} vanished mid-burst`);
          serverPeak = Math.max(serverPeak, processRss(server.proc.pid));
          await sleep(ctx.args.stepMs);
        }
        idle = await waitIdleAbove(cdp, pageSession, idle);
        await sleep(1_000);
      }
      const heaps = await gcAndHeaps(cdp, pageSession, workers);
      const serverSettled = processRss(server.proc.pid);
      const row = {
        cycle,
        serverSettled,
        serverPeak: Math.max(serverPeak, serverSettled),
        serverFootprint: processFootprint(server.proc.pid),
        rendererRss: rendererRss(browser.pid),
        heapPage: heaps.page,
        heapWorkers: heaps.workers,
        crashed: hasCrashed(),
      };
      rows.push(row);
      console.error(`scrub cycle ${cycle}: server footprint ${mb(row.serverFootprint)} MB, `
        + `rss ${mb(serverSettled)} MB (peak ${mb(row.serverPeak)} MB), `
        + `renderer rss ${mb(row.rendererRss)} MB`);
      if (row.crashed) {
        console.error('renderer crashed; stopping scrub cycles');
        break;
      }
    }
  } finally {
    cdp.close();
    browser.kill('SIGKILL');
    server.proc.kill('SIGTERM');
    await sleep(300);
    if (!ctx.args.keep) fs.rmSync(runDir, { recursive: true, force: true });
  }
  return rows;
}

function scrubReport(rows, args, count, style) {
  let md = `# Memory probe scrub mode (N=${count}, style=${style}, `
    + `${args.steps} steps/cycle @ ${args.stepMs} ms)\n\n`;
  md += 'Each cycle rapidly steps the time range forward then back via the\n';
  md += 'time stepper, then settles. Server footprint (dirty + compressed,\n';
  md += 'what iOS jetsam enforces) is the leak signal; server RSS also\n';
  md += 'counts clean reusable pages the allocator retains, so an RSS\n';
  md += 'plateau over a flat footprint is benign.\n\n';
  md += '| cycle | server footprint MB | server RSS MB | server peak MB | renderer RSS MB | page heap MB | worker heap MB |\n';
  md += '|---|---|---|---|---|---|---|\n';
  for (const r of rows) {
    md += `| ${r.cycle}${r.crashed ? ' (CRASHED)' : ''} | ${mb(r.serverFootprint)} `
      + `| ${mb(r.serverSettled)} | ${mb(r.serverPeak)} | ${mb(r.rendererRss)} `
      + `| ${mb(r.heapPage)} | ${mb(r.heapWorkers)} |\n`;
  }
  const later = rows.slice(Math.floor(rows.length / 2));
  const s = (f) => mb(slope(later.map((r) => [r.cycle, f(r)])));
  md += `\nLate-cycle trend: server footprint ${s((r) => r.serverFootprint)} MB/cycle, `
    + `server RSS ${s((r) => r.serverSettled)} MB/cycle, `
    + `renderer RSS ${s((r) => r.rendererRss)} MB/cycle.\n`;
  return md;
}

// --- leak mode ---

async function leakMode(ctx, count, style) {
  const runDir = path.join(ctx.scratch, `leak-${count}-${style}`);
  const server = await startServer(ctx.devBin, runDir, { count, shape: ctx.args.shape, style }, ctx.args.keep);
  const { proc: browser, cdp } = await launchBrowser(
    ctx.browserBin, path.join(runDir, 'chrome-profile'), ctx.args.headful,
  );
  const rows = [];
  try {
    const pageSession = await attachPage(cdp);
    const workers = new Map();
    trackWorkers(cdp, workers);
    await cdp.send('Network.enable', {}, pageSession);
    await cdp.send('Page.enable', {}, pageSession);
    await cdp.send('Runtime.enable', {}, pageSession);
    await cdp.send('Target.setAutoAttach', {
      autoAttach: true, waitForDebuggerOnStart: false, flatten: true,
    }, pageSession);
    const hasCrashed = await trackCrashes(cdp, pageSession);
    for (let cycle = 0; cycle < ctx.args.cycles; cycle++) {
      const sizes = await loadApp(cdp, pageSession, server.port, hasCrashed);
      await sleep(1_000);
      const loaded = await gcAndHeaps(cdp, pageSession, workers);
      const rssLoaded = rendererRss(browser.pid);
      await cdp.send('Page.navigate', { url: 'about:blank' }, pageSession);
      await sleep(1_500);
      const unloaded = await gcAndHeaps(cdp, pageSession, workers);
      const rssUnloaded = rendererRss(browser.pid);
      const row = {
        cycle,
        totalBytes: sizes.points + sizes.lines + sizes.mvt,
        heapLoadedPage: loaded.page,
        heapLoadedWorkers: loaded.workers,
        rssLoaded,
        heapUnloadedPage: unloaded.page,
        rssUnloaded,
      };
      row.crashed = hasCrashed();
      rows.push(row);
      console.error(`cycle ${cycle}: loaded rss ${mb(rssLoaded)} MB, `
        + `unloaded rss ${mb(rssUnloaded)} MB, unloaded page heap ${mb(unloaded.page)} MB`);
      if (row.crashed) {
        console.error('renderer crashed; stopping leak cycles');
        break;
      }
    }
  } finally {
    cdp.close();
    browser.kill('SIGKILL');
    server.proc.kill('SIGTERM');
    await sleep(300);
    if (!ctx.args.keep) fs.rmSync(runDir, { recursive: true, force: true });
  }
  return rows;
}

function leakReport(rows, count, style) {
  let md = `# Memory probe leak mode (N=${count}, style=${style})\n\n`;
  md += 'Post-GC values per load/unload cycle. A plateau is ratchet behavior\n';
  md += '(expected); a steady slope past the plateau is a true leak.\n\n';
  md += '| cycle | loaded RSS MB | loaded page heap MB | loaded worker heap MB | unloaded RSS MB | unloaded page heap MB |\n';
  md += '|---|---|---|---|---|---|\n';
  for (const r of rows) {
    md += `| ${r.cycle} | ${mb(r.rssLoaded)} | ${mb(r.heapLoadedPage)} | ${mb(r.heapLoadedWorkers)} `
      + `| ${mb(r.rssUnloaded)} | ${mb(r.heapUnloadedPage)} |\n`;
  }
  if (rows.some((r) => r.crashed)) {
    md += '\nRenderer CRASHED during the run; later rows are invalid.\n';
  }
  // slope over the later half of cycles, where the plateau should be reached
  const later = rows.slice(Math.floor(rows.length / 2));
  const rssSlope = slope(later.map((r) => [r.cycle, r.rssUnloaded]));
  const heapSlope = slope(later.map((r) => [r.cycle, r.heapUnloadedPage]));
  md += `\nLate-cycle trend: unloaded RSS ${mb(rssSlope)} MB/cycle, `
    + `unloaded page heap ${mb(heapSlope)} MB/cycle.\n`;
  return md;
}

// --- main ---

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const devBin = args.devBin ?? process.env.STEM_PROBE_DEV_BIN
    ?? path.join(WORKSPACE, 'bazel-bin/src/app/stem/dev');
  if (args.build) {
    console.error('building //src/app/stem:dev ...');
    const r = spawnSync('bazel', ['build', '//src/app/stem:dev'], {
      cwd: WORKSPACE, stdio: ['ignore', 'inherit', 'inherit'],
    });
    if (r.status !== 0) throw new Error('bazel build failed');
  }
  if (!fs.existsSync(devBin)) throw new Error(`dev binary not found at ${devBin}`);
  const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'stem-probe-'));
  const browserBin = findBrowser(args.browser);
  console.error(`browser: ${browserBin}`);
  const ctx = { args, devBin, scratch, browserBin };
  fs.mkdirSync(args.out, { recursive: true });
  const stamp = new Date().toISOString().replace(/[:.]/g, '-').slice(0, 19);
  try {
    if (args.mode === 'sweep') {
      const rows = [];
      for (const style of args.styles) {
        for (const count of args.counts) {
          console.error(`--- run: N=${count} style=${style} shape=${args.shape}`);
          const row = await sweepRun(ctx, count, style, args.shape);
          console.error(JSON.stringify(row));
          rows.push(row);
        }
      }
      const { md, ks } = sweepReport(rows);
      const base = path.join(args.out, `sweep-${stamp}`);
      fs.writeFileSync(`${base}.json`, JSON.stringify({ args, rows, ks }, null, 2));
      fs.writeFileSync(`${base}.md`, md);
      console.log(md);
      console.error(`results: ${base}.json`);
    } else if (args.mode === 'tiles') {
      const rows = [];
      for (const style of args.styles) {
        for (const count of args.counts) {
          for (const zoom of args.zooms) {
            console.error(`--- run: N=${count} style=${style} zoom=${zoom} shape=${args.shape}`);
            const row = await tilesRun(ctx, count, style, zoom);
            console.error(JSON.stringify({ ...row, tiles: undefined }));
            rows.push(row);
          }
        }
      }
      const { md, rates } = tilesReport(rows);
      const base = path.join(args.out, `tiles-${stamp}`);
      fs.writeFileSync(`${base}.json`, JSON.stringify({ args, rows, rates }, null, 2));
      fs.writeFileSync(`${base}.md`, md);
      console.log(md);
      console.error(`results: ${base}.json`);
    } else if (args.mode === 'churn') {
      const count = args.counts[0];
      const style = args.styles[0];
      for (const kind of args.churn) {
        console.error(`--- churn: kind=${kind} N=${count} style=${style}`);
        const rows = await churnMode(ctx, kind, count, style);
        const md = churnReport(rows, kind, count, style);
        const base = path.join(args.out, `churn-${kind}-${stamp}`);
        fs.writeFileSync(`${base}.json`, JSON.stringify({ args, kind, rows }, null, 2));
        fs.writeFileSync(`${base}.md`, md);
        console.log(md);
        console.error(`results: ${base}.json`);
      }
    } else if (args.mode === 'scrub') {
      const count = args.counts[0];
      const style = args.styles[0];
      console.error(`--- scrub: N=${count} style=${style} steps=${args.steps} stepMs=${args.stepMs}`);
      const rows = await scrubMode(ctx, count, style);
      const md = scrubReport(rows, args, count, style);
      const base = path.join(args.out, `scrub-${stamp}`);
      fs.writeFileSync(`${base}.json`, JSON.stringify({ args, rows }, null, 2));
      fs.writeFileSync(`${base}.md`, md);
      console.log(md);
      console.error(`results: ${base}.json`);
    } else if (args.mode === 'leak') {
      const count = args.counts[0];
      const style = args.styles[0];
      const rows = await leakMode(ctx, count, style);
      const md = leakReport(rows, count, style);
      const base = path.join(args.out, `leak-${stamp}`);
      fs.writeFileSync(`${base}.json`, JSON.stringify({ args, rows }, null, 2));
      fs.writeFileSync(`${base}.md`, md);
      console.log(md);
      console.error(`results: ${base}.json`);
    } else {
      throw new Error(`unknown mode ${args.mode}`);
    }
  } finally {
    if (!args.keep) fs.rmSync(scratch, { recursive: true, force: true });
    else console.error(`scratch kept at ${scratch}`);
  }
}

main().catch((e) => { console.error(e); process.exit(1); });
