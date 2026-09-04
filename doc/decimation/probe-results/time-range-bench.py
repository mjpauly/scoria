"""Latency of the map query paths for time-constrained views on bench4.db
(2.56M points, grid indexes). Compares today's temporal count/fetch plans
against grid-index and timestamp-index (rowid proxy) alternatives, and the
spatial walk, for one day / one week / all time at several zooms."""
import sqlite3, math, time, sys

S = sys.argv[1]  # scratch copy of the export
c = sqlite3.connect(f'file:{S}?mode=ro', uri=True)
M = 4294967295
VW, VH = 400, 800

def merc(lat, lng):
    x = (lng + 180) / 360
    y = (1 - math.log(math.tan(math.radians(lat)) + 1 / math.cos(math.radians(lat))) / math.pi) / 2
    return min(int(x * 2**32), M), min(int(y * 2**32), M)

cy, cx, n = c.execute("select gy>>14, gx>>14, count(*) n from location group by 1,2 order by n desc limit 1").fetchone()
lat, lng = c.execute("select avg(latitude),avg(longitude) from location where gy>>14=? and gx>>14=?", (cy, cx)).fetchone()

def bounds(z):
    dpp = 360 / (512 * 2**z)
    dlng = VW * dpp / 2 * 1.04
    dlat = VH * dpp * math.cos(math.radians(lat)) / 2 * 1.04
    return lat - dlat, lat + dlat, lng - dlng, lng + dlng

def timeit(fn, reps=3):
    ts = []
    for _ in range(reps):
        t = time.perf_counter(); r = fn(); ts.append(time.perf_counter() - t)
    return min(ts) * 1000, r

def plan(q, args=()):
    return ' | '.join(r[3] for r in c.execute('explain query plan ' + q, args))

# densest day, and the week around it
_b = bounds(10)
day = c.execute("select timestamp/86400 d from location where latitude>=? and latitude<=? and longitude>=? and longitude<=? group by d order by count(*) desc limit 1", _b).fetchone()[0]
RANGES = {
    'day': (day * 86400, (day + 1) * 86400),
    'week': ((day - 3) * 86400, (day + 4) * 86400),
    'month': ((day - 15) * 86400, (day + 15) * 86400),
    'year': ((day - 182) * 86400, (day + 183) * 86400),
    'all': (0, 2**40),
}

def walk_sql(L, cy0, cy1, cx0, cx1, dshift, s, e):
    seek = lambda pos: f"""(select cy{L} * 4294967296 + cx{L} from location indexed by idx{L}
        where (cy{L}, cx{L}) > (({pos}) >> 32, ({pos}) & {M}) and cy{L} <= {cy1} order by cy{L}, cx{L} limit 1)"""
    pos_expr = f"""(case when (w.nxt & {M}) < {cx0} then (w.nxt >> 32 << 32) + {cx0} - 1
        when (w.nxt & {M}) > {cx1} then (((w.nxt >> 32) + 1) << 32) + {cx0} - 1 else w.nxt end)"""
    p0 = f"(({cy0} << 32) + {cx0} - 1)"
    latest = f"""(select timestamp * 16777216 + id from location indexed by idx{L}
        where cy{L} = w.pos >> 32 and cx{L} = w.pos & {M} and timestamp >= {s} and timestamp < {e}
        order by timestamp desc limit 1)"""
    return f"""with recursive w(pos, real, nxt) as (
        select {p0}, 0, {seek(p0)}
        union all
        select {pos_expr}, (w.nxt & {M}) between {cx0} and {cx1}, {seek(pos_expr)} from w where w.nxt is not null)
      select max(tsid) & 16777215 from (select {latest} as tsid, (w.pos >> 32) >> {dshift} as qy, (w.pos & {M}) >> {dshift} as qx from w where real)
      where tsid is not null group by qy, qx"""

def index_for(shift):
    for L in (8, 10, 12):
        if 22 - L <= shift:
            return L
    return None

show_plans = '--plans' in sys.argv
for z in (6, 10, 13, 16):
    lat0, lat1, lng0, lng1 = bounds(z)
    x0, y0 = merc(lat1, lng0); x1, y1 = merc(lat0, lng1)
    s12 = 10
    a, b, cc, d = y0 >> s12, y1 >> s12, x0 >> s12, x1 >> s12
    for name, (s, e) in RANGES.items():
        # rowid proxy for a timestamp index (ids are time ordered)
        id0 = c.execute("select min(id) from location where timestamp >= ?", (s,)).fetchone()[0] or 0
        id1 = c.execute("select max(id) from location where timestamp < ?", (e,)).fetchone()[0] or 0
        in_range = id1 - id0 + 1 if id1 >= id0 else 0
        bnd = "latitude>=? and latitude<=? and longitude>=? and longitude<=?"
        args = (lat0, lat1, lng0, lng1)
        tr = f"timestamp >= {s} and timestamp < {e}"

        # A: today's temporal count (planner's choice)
        qA = f"select count(*) from location where {bnd} and id % 10 = 0 and {tr}"
        tA, nA = timeit(lambda: c.execute(qA, args).fetchone()[0])
        # A': forced table scan
        qA2 = f"select count(*) from location not indexed where {bnd} and id % 10 = 0 and {tr}"
        tA2, _ = timeit(lambda: c.execute(qA2, args).fetchone()[0])
        # B: rowid range (timestamp index proxy) + bounds
        qB = f"select count(*) from location where id between {id0} and {id1} and {bnd} and id % 10 = 0"
        tB, nB = timeit(lambda: c.execute(qB, args).fetchone()[0])
        # C: grid L12 covering count with edge-cell exact check
        qC = f"""select count(*) from location indexed by idx12
            where cy12 between {a} and {b} and cx12 between {cc} and {d}
            and ((cy12 between {a+1} and {b-1} and cx12 between {cc+1} and {d-1})
                 or (gy between {y0} and {y1} and gx between {x0} and {x1}))
            and id % 10 = 0 and {tr}"""
        tC, nC = timeit(lambda: c.execute(qC).fetchone()[0])
        # C2: grid L12 count without the edge check (pure covering)
        qC2 = f"""select count(*) from location indexed by idx12
            where cy12 between {a} and {b} and cx12 between {cc} and {d}
            and id % 10 = 0 and {tr}"""
        tC2, nC2 = timeit(lambda: c.execute(qC2).fetchone()[0])
        # C3: level-matched band count with edge check
        Lc = index_for(math.floor(32 - 9 - z + math.log2(0.5))) or 8
        sc = 22 - Lc
        ca, cb, ccc, cd = y0 >> sc, y1 >> sc, x0 >> sc, x1 >> sc
        qC3 = f"""select count(*) from location indexed by idx{Lc}
            where cy{Lc} between {ca} and {cb} and cx{Lc} between {ccc} and {cd}
            and ((cy{Lc} between {ca+1} and {cb-1} and cx{Lc} between {ccc+1} and {cd-1})
                 or (gy between {y0} and {y1} and gx between {x0} and {x1}))
            and id % 10 = 0 and {tr}"""
        tC3, nC3 = timeit(lambda: c.execute(qC3).fetchone()[0])
        # C4: level-matched per-row seek via CTE
        qC4 = f"""with recursive r(cy) as (select {ca} union all select cy+1 from r where cy < {cb})
            select count(*) from r join location indexed by idx{Lc}
            on cy{Lc} = r.cy and cx{Lc} between {ccc} and {cd}
            where ((cy{Lc} between {ca+1} and {cb-1} and cx{Lc} between {ccc+1} and {cd-1})
                 or (gy between {y0} and {y1} and gx between {x0} and {x1}))
            and id % 10 = 0 and {tr}"""
        tC4, nC4 = timeit(lambda: c.execute(qC4).fetchone()[0])
        # D: temporal fetch, decim to 10k, today's plan vs grid
        N = max(nA, 1) * 10
        decim = (N - 1) // 10000 + 1
        qD = f"select * from location where {bnd} and id % {decim} = 0 and {tr}"
        tD, rD = timeit(lambda: c.execute(qD, args).fetchall())
        qD2 = f"""select * from location indexed by idx12
            where cy12 between {a} and {b} and cx12 between {cc} and {d}
            and gy between {y0} and {y1} and gx between {x0} and {x1}
            and id % {decim} = 0 and {tr}"""
        tD2, rD2 = timeit(lambda: c.execute(qD2).fetchall())
        # E: spatial trigger check (LIMIT-bounded count), today's plan
        qE = f"select count(*) from (select 1 from location where {bnd} and {tr} limit 10001)"
        tE, _ = timeit(lambda: c.execute(qE, args).fetchone()[0])
        # F: spatial walk with the time filter, at the app's level choice
        shift = math.floor(32 - 9 - z + math.log2(0.5))
        L = index_for(shift)
        if L is not None:
            sL = 22 - L
            qF = walk_sql(L, y0 >> sL, y1 >> sL, x0 >> sL, x1 >> sL, shift - sL, s, e)
        else:
            L = 8; sL = 14
            qF = f"""select id, max(timestamp) from location indexed by idx8
                  where cy8 between {y0>>sL} and {y1>>sL} and cx8 between {x0>>sL} and {x1>>sL}
                  and gy between {y0} and {y1} and gx between {x0} and {x1} and {tr}
                  group by gy >> {shift}, gx >> {shift}"""
        tF, rF = timeit(lambda: c.execute(qF).fetchall())
        # G: bucketed via rowid range (timestamp index proxy), group by grid cell
        qG = f"""select id, max(timestamp) from location where id between {id0} and {id1}
                 and gy between {y0} and {y1} and gx between {x0} and {x1}
                 group by gy >> {shift}, gx >> {shift}"""
        tG, rG = timeit(lambda: c.execute(qG).fetchall())

        print(f"z{z:<2} {name:<4} inrange={in_range:>8} N~{N:>8} | count: today {tA:6.1f} scan {tA2:6.1f} rowid {tB:6.1f} grid12 {tC:6.1f} gridL{Lc} band {tC3:6.1f} cte {tC4:6.1f} (n {nA}/{nB}/{nC}/{nC3}/{nC4}) rows={cb-ca+1}"
              f" | fetch decim={decim}: today {tD:6.1f} grid {tD2:6.1f} ({len(rD)}/{len(rD2)})"
              f" | trigger {tE:5.1f} | walk L{L} {tF:7.1f} rowid-bucket {tG:7.1f} ({len(rF)}/{len(rG)})", flush=True)
        if show_plans and name == 'day' and z == 10:
            print('   planA:', plan(qA, args))
            print('   planD:', plan(qD, args))
            print('   planE:', plan(qE, args))
