# Tile Server

## Notes for Future

Prototyping used digitalocean's app platform, which has a 2GB file system max.
For the full 61.00GB OSM map we'll need ~41.2G space for the mbtiles. This can
work with a fairly cheap $12 digitalocean droplet with 50G of disk space.
[ref](https://www.digitalocean.com/pricing/droplets).

- small osm data: 254M
- global coastline: 1.1G
- mbtiles: 167M

`.167 / .254 * 61 + 1.1 = 41.2G` (worst-case)

## Building tiles

Build the tiles with `./buildtiles.sh`. This creates a `build` directory
containing the mbtiles and intermediate artifacts.

Run the tile server locally with `mbtileserver -d build/mbtiles-data`. (Prereq:
`cargo install mbtileserver@0.1.7`). This reduces system storage usage compared
to copying everything into a docker image and running that.

Generate the docker image and run it locally with:

```
docker build --tag tileserver .
docker run -it -p 3000:3000 tileserver
```

## Pushing to prod

We must rebuild with linux/amd64 as the platform target to be able to run it on
digitalocean. Warning: make sure you increase the default resource limits in
docker desktop! These were 1GB memory and 1 CPU by default. Docker will squat on
the memory so increase it some, but not too much. CPU usage is likely safer to
increase significantly. This is what will speed up the build most, but you also
need to ensure there's enough memory or the build might suddenly fail.

On an M1 mac this took 10 minutes with 4 cpus and 6 GB memory for docker.

```
docker build --tag tileserver --platform=linux/amd64 .
```

[Guide](https://docs.digitalocean.com/products/container-registry/quickstart/)

```
doctl registry login
docker tag tileserver registry.digitalocean.com/epsln/tile
docker push registry.digitalocean.com/epsln/tileserver
```

## Frontend example

Plotly and maplibre examples of consuming a style json which points to the
digital ocean app. (Change to `http://localhost:3000/services/bayarea` for local
dev)

Edit styles with maputnik: `docker run -it --rm -p 8888:8888 maputnik/editor`.

The styles in the example directory are minimally altered to point to different
tile resources.

Clone `git clone https://github.com/klokantech/klokantech-gl-fonts.git` to get
fonts. Ensure the names match up with the actual font directory names.
