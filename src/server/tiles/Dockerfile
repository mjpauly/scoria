FROM docker.io/library/rust:1.66-alpine as builder

# Sqlite static linking tips from:
# https://www.21analytics.ch/blog/docker-from-scratch-for-rust-applications/

RUN apk add --no-cache musl-dev sqlite-static

# Set `SYSROOT` to a dummy path (default is /usr) because pkg-config-rs *always*
# links those located in that path dynamically but we want static linking, c.f.
# https://github.com/rust-lang/pkg-config-rs/blob/54325785816695df031cef3b26b6a9a203bbc01b/src/lib.rs#L613
ENV SYSROOT=/dummy

# Tell libsqlite3-sys to statically link libsqlite3.
ENV SQLITE3_STATIC=1 SQLITE3_LIB_DIR=/usr/lib/

RUN cargo install --root /cargo mbtileserver@0.1.7


# Runner layer

FROM alpine:latest
COPY build/mbtiles-data /tiles
COPY --from=builder /cargo/bin/mbtileserver /
ENTRYPOINT ["./mbtileserver","-d","/tiles","--allowed-hosts","localhost,127.0.0.1,[::1],gumball-3ven8.ondigitalocean.app","-H","Access-Control-Allow-Origin:*"]
# Access-Control... (CORS) is configured in the digitial ocean spec.yaml,
# 90% sure it's ok to remove it


# For reference, a simpler dev container can look like this (tested):

# FROM rust:1.66.1 AS builder
# RUN cargo install --root /cargo mbtileserver@0.1.7
# COPY build/mbtiles-data /tiles
# ENTRYPOINT ["./cargo/bin/mbtileserver","-d","/tiles"]
