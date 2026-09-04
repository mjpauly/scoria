import sqlite3, math, os, time, json, sys
S=sys.argv[1]  # scratch copy of the export
LEVELS=[4,6,8,10,12,14,16]; ZOOMS=list(range(6,19)); VW,VH=400,800; M=4294967295
c=sqlite3.connect(S); c.execute("pragma journal_mode=wal")
def merc(lat,lng):
    x=(lng+180)/360; y=(1-math.log(math.tan(math.radians(lat))+1/math.cos(math.radians(lat)))/math.pi)/2
    return min(int(x*2**32),M), min(int(y*2**32),M)
cols=[r[1] for r in c.execute("pragma table_xinfo(location)")]
if 'gx' not in cols:
    c.execute("alter table location add column gx integer"); c.execute("alter table location add column gy integer")
    rows=c.execute("select id,latitude,longitude from location").fetchall()
    c.executemany("update location set gx=?, gy=? where id=?",[(*merc(la,ln),i) for i,la,ln in rows]); c.commit()
    for L in LEVELS:
        s=22-L
        c.execute(f"alter table location add column cy{L} integer generated always as (gy >> {s}) virtual")
        c.execute(f"alter table location add column cx{L} integer generated always as (gx >> {s}) virtual")
        c.execute(f"create index idx{L} on location(cy{L}, cx{L}, timestamp)")
    c.commit(); c.execute("vacuum"); print("schema built")
cy,cx,n=c.execute("select gy>>14, gx>>14, count(*) n from location group by 1,2 order by n desc limit 1").fetchone()
lat,lng=c.execute("select avg(latitude),avg(longitude) from location where gy>>14=? and gx>>14=?",(cy,cx)).fetchone()
def bounds(z):
    dpp=360/(512*2**z); dlng=VW*dpp/2*1.04; dlat=VH*dpp*math.cos(math.radians(lat))/2*1.04
    return lat-dlat,lat+dlat,lng-dlng,lng+dlng
def timeit(fn,reps=3):
    ts=[]
    for _ in range(reps):
        t=time.perf_counter(); r=fn(); ts.append(time.perf_counter()-t)
    return min(ts), r
def fetch_rows(ids):
    c.execute("create temp table if not exists ids(id integer primary key)"); c.execute("delete from ids")
    c.executemany("insert into ids values(?)",[(i,) for i in ids])
    return c.execute("select l.* from ids join location l on l.id=ids.id").fetchall()
def walk_sql(L, cy0, cy1, cx0, cx1, dshift):
    """occupied L-cells in rect, newest row per L-cell, grouped by query cell (L-cell >> dshift)."""
    seek=lambda pos: f"""(select cy{L} * 4294967296 + cx{L} from location indexed by idx{L}
        where (cy{L}, cx{L}) > (({pos}) >> 32, ({pos}) & {M}) and cy{L} <= {cy1} order by cy{L}, cx{L} limit 1)"""
    pos_expr=f"""(case when (w.nxt & {M}) < {cx0} then (w.nxt >> 32 << 32) + {cx0} - 1
        when (w.nxt & {M}) > {cx1} then (((w.nxt >> 32) + 1) << 32) + {cx0} - 1 else w.nxt end)"""
    p0=f"(({cy0} << 32) + {cx0} - 1)"
    latest=f"""(select timestamp * 16777216 + id from location indexed by idx{L}
        where cy{L} = w.pos >> 32 and cx{L} = w.pos & {M} order by timestamp desc limit 1)"""
    return f"""with recursive w(pos, real, nxt) as (
        select {p0}, 0, {seek(p0)}
        union all
        select {pos_expr}, (w.nxt & {M}) between {cx0} and {cx1}, {seek(pos_expr)} from w where w.nxt is not null)
      select max(tsid) & 16777215 from (select {latest} as tsid, (w.pos >> 32) >> {dshift} as qy, (w.pos & {M}) >> {dshift} as qx from w where real)
      group by qy, qx"""
results={}
for z in ZOOMS:
    lat0,lat1,lng0,lng1=bounds(z)
    N=c.execute("select count(*) from location where latitude>=? and latitude<=? and longitude>=? and longitude<=?",(lat0,lat1,lng0,lng1)).fetchone()[0]
    stripe=c.execute("select count(*) from location where longitude>=? and longitude<=?",(lng0,lng1)).fetchone()[0]
    dlng=0.5*360/(512*2**z); dlat=dlng*max(math.cos(math.radians(lat)),0.1)
    qa=f"""select *, max(timestamp) from location where latitude>=? and latitude<=? and longitude>=? and longitude<=?
           group by cast((longitude+180.0)/{dlng} as integer), cast((latitude+90.0)/{dlat} as integer)"""
    ta,ra=timeit(lambda: c.execute(qa,(lat0,lat1,lng0,lng1)).fetchall())
    row={'N':N,'stripe':stripe,'A_ms':ta*1000,'cells':len(ra),'levels':{}}
    sq=22-z
    for L in LEVELS:
        s=22-L
        x0,y0=merc(lat1,lng0); x1,y1=merc(lat0,lng1)
        if L<=z:
            # index cell >= query cell: walk at L, group sub-cells to query cells (dshift = z-L... query coarser? no)
            pass
        if z<=L:
            # query cell coarser or equal: walk occupied L-cells, group by query cell
            cx0,cx1,cy0,cy1=x0>>s,x1>>s,y0>>s,y1>>s
            q=walk_sql(L,cy0,cy1,cx0,cx1,L-z)
            tq,ids=timeit(lambda: [r[0] for r in c.execute(q).fetchall()])
        else:
            # query cell finer than index cell: index as bounds filter, group by row coords
            cx0,cx1,cy0,cy1=x0>>s,x1>>s,y0>>s,y1>>s
            q=f"""select id, max(timestamp) from location indexed by idx{L}
                  where cy{L} between {cy0} and {cy1} and cx{L} between {cx0} and {cx1}
                  and gy >= {y0} and gy <= {y1} and gx >= {x0} and gx <= {x1}
                  group by gy >> {sq}, gx >> {sq}"""
            tq,ids=timeit(lambda: [r[0] for r in c.execute(q).fetchall()])
        tf,_=timeit(lambda: fetch_rows(ids))
        row['levels'][L]={'q_ms':tq*1000,'rows_ms':tf*1000,'cells':len(ids),'speedup':ta/(tq+tf)}
    results[z]=row
    print(f"z{z:<3} N={N:>7} stripe={stripe:>7} A={ta*1000:6.0f}ms cells={len(ra):>6} | "+"  ".join(f"L{L}:{v['speedup']:4.1f}x({v['q_ms']:.0f}+{v['rows_ms']:.0f})" for L,v in row['levels'].items()), flush=True)
json.dump(results,open('bench4.json','w'),indent=1)
