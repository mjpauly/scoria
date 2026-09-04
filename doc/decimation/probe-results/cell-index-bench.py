import sqlite3, math, os, sys, time, statistics as st
S=sys.argv[1]  # scratch copy of the export
ZOOMS=[9,11,13,15]
VW,VH=400,800  # CSS px viewport
c=sqlite3.connect(S)
c.execute("pragma journal_mode=wal")

def merc(lat,lng):
    x=(lng+180)/360; y=(1-math.log(math.tan(math.radians(lat))+1/math.cos(math.radians(lat)))/math.pi)/2
    return min(int(x*2**32),2**32-1), min(int(y*2**32),2**32-1)

# --- schema: integer grid columns + per-level virtual cells + indexes
cols=[r[1] for r in c.execute("pragma table_info(location)")]
if 'gx' not in cols:
    t=time.time()
    c.execute("alter table location add column gx integer")
    c.execute("alter table location add column gy integer")
    rows=c.execute("select id,latitude,longitude from location").fetchall()
    c.executemany("update location set gx=?, gy=? where id=?",[(*merc(la,ln),i) for i,la,ln in rows])
    c.commit()
    for z in ZOOMS:
        s=22-z  # 0.5px cells at zoom z: 2^(z+10) divisions per axis
        c.execute(f"alter table location add column cy{z} integer generated always as (gy >> {s}) virtual")
        c.execute(f"alter table location add column cx{z} integer generated always as (gx >> {s}) virtual")
        c.execute(f"create index idx{z} on location(cy{z}, cx{z}, timestamp)")
    c.commit(); c.execute("vacuum")
    print(f"schema built in {time.time()-t:.1f}s, db {os.path.getsize(S)/1e6:.0f} MB")

# --- densest 152 m cell as view center
cy,cx,n=c.execute("select gy>>14, gx>>14, count(*) n from location group by 1,2 order by n desc limit 1").fetchone()
lat,lng=c.execute("select avg(latitude),avg(longitude) from location where gy>>14=? and gx>>14=?",(cy,cx)).fetchone()
print(f"center {lat:.4f},{lng:.4f} ({n} pts in its 152 m cell)")

def bounds(z):
    dpp=360/(512*2**z)  # deg lng per css px
    dlng=VW*dpp/2*1.04; dlat=VH*dpp*math.cos(math.radians(lat))/2*1.04  # incl 4% expansion
    return lat-dlat,lat+dlat,lng-dlng,lng+dlng

def timeit(fn,reps=3):
    ts=[]
    for _ in range(reps):
        t=time.perf_counter(); r=fn(); ts.append(time.perf_counter()-t)
    return min(ts), r

for z in ZOOMS:
    lat0,lat1,lng0,lng1=bounds(z)
    N=c.execute("select count(*) from location where latitude>=? and latitude<=? and longitude>=? and longitude<=?",(lat0,lat1,lng0,lng1)).fetchone()[0]
    dlng=0.5*360/(512*2**z); dlat=dlng*max(math.cos(math.radians(lat)),0.1)
    # A: today's query
    qa=f"""select *, max(timestamp) from location where latitude>=? and latitude<=? and longitude>=? and longitude<=?
           group by cast((longitude+180.0)/{dlng} as integer), cast((latitude+90.0)/{dlat} as integer)"""
    ta,ra=timeit(lambda: c.execute(qa,(lat0,lat1,lng0,lng1)).fetchall())
    # B: same shape, integer cells
    qb=f"""select *, max(timestamp) from location where latitude>=? and latitude<=? and longitude>=? and longitude<=?
           group by cy{z}, cx{z}"""
    tb,rb=timeit(lambda: c.execute(qb,(lat0,lat1,lng0,lng1)).fetchall())
    # C: loose index scan over the cell rectangle
    s=22-z
    x0,y0=merc(lat1,lng0); x1,y1=merc(lat0,lng1)  # y grows south
    cx0,cx1,cy0,cy1=x0>>s,x1>>s,y0>>s,y1>>s
    M=4294967295
    seek=lambda pos: f"""(select cy{z} * 4294967296 + cx{z} from location indexed by idx{z}
                      where (cy{z}, cx{z}) > (({pos}) >> 32, ({pos}) & {M}) and cy{z} <= {cy1}
                      order by cy{z}, cx{z} limit 1)"""
    pos_expr=f"""(case when (w.nxt & {M}) < {cx0} then (w.nxt >> 32 << 32) + {cx0} - 1
                       when (w.nxt & {M}) > {cx1} then (((w.nxt >> 32) + 1) << 32) + {cx0} - 1
                       else w.nxt end)"""
    p0=f"(({cy0} << 32) + {cx0} - 1)"
    qc=f"""with recursive w(pos, real, nxt) as (
        select {p0}, 0, {seek(p0)}
        union all
        select {pos_expr}, (w.nxt & {M}) between {cx0} and {cx1}, {seek(pos_expr)}
        from w where w.nxt is not null)
      select (select id from location indexed by idx{z} where cy{z} = w.pos >> 32 and cx{z} = w.pos & {M}
              order by timestamp desc limit 1) from w where real"""
    tc,rc=timeit(lambda: c.execute(qc).fetchall())
    # C + row fetch by id
    ids=[r[0] for r in rc]
    def fetch():
        c.execute("create temp table if not exists ids(id integer primary key)"); c.execute("delete from ids")
        c.executemany("insert into ids values(?)",[(i,) for i in ids])
        return c.execute("select l.* from ids join location l on l.id=ids.id").fetchall()
    tf,rf=timeit(fetch)
    print(f"z{z:<3} N={N:>7}  A degree-cells {ta*1000:6.0f} ms ({len(ra)} cells)   B int-cells {tb*1000:6.0f} ms ({len(rb)})   C loose-scan {tc*1000:6.0f} ms ({len(rc)}) + rows {tf*1000:5.0f} ms")
c.close()
