#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

# Copies the sources needed to build the website to the repo that is shared with
# the cloud provider
#
# Run from repo root with argument that is the path to the repo to copy to
# 
# NOTE: if updating this, also update the Dockerfile
#
# WARNING: `rsync path/to/dir dest/dir` copies the source dir INTO the dest dir

rsync -a --delete BUILD $1/
rsync -a --delete WORKSPACE $1/
rsync -a --delete .bazelversion $1/
mkdir -p $1/src/server/website
rsync -a --delete src/server/website/Dockerfile $1/
rsync -a --delete src/server/website/BUILD $1/src/server/website/BUILD
rsync -a --delete src/server/website/templates.bzl $1/src/server/website/templates.bzl
touch $1/src/server/website/.env
rsync -a --delete src/server/website/Cargo.lock $1/src/server/website/Cargo.lock
rsync -a --delete src/server/website/Cargo.Bazel.lock $1/src/server/website/Cargo.Bazel.lock
rsync -a --delete src/server/website/aft $1/src/server/website/
rsync -a --delete src/server/website/static $1/src/server/website/

# tailwind
rsync -a --delete src/server/website/tailwind $1/src/server/website/
mkdir -p $1/src/app/stem/front
rsync -a --delete src/app/stem/front/yarn.lock $1/src/app/stem/front/yarn.lock
rsync -a --delete src/app/stem/front/package.json $1/src/app/stem/front/package.json
# need BUILD file to mark directory as a package, but contents are not needed
touch $1/src/app/stem/front/BUILD

# other items referened in the WORKSPACE
rsync -a --delete src/app/stem/Cargo.lock $1/src/app/stem/Cargo.lock
rsync -a --delete src/app/stem/Cargo.Bazel.lock $1/src/app/stem/Cargo.Bazel.lock
touch $1/src/app/stem/BUILD
