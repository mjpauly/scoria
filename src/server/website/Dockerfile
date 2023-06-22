FROM scratch as copier
# tiny layer for the sources

WORKDIR epsilon

# Copy into the container the components needed for the website.
# We granularly pick out the needed pieces since some things like the Dockerfile
# prevent caching while being actively worked on.

# NOTE: if updating this, also update copy_sources.sh!

COPY BUILD .
COPY WORKSPACE .
COPY .bazelversion .
COPY src/server/website/BUILD src/server/website/BUILD
COPY src/server/website/Cargo.lock src/server/website/Cargo.lock
COPY src/server/website/Cargo.Bazel.lock src/server/website/Cargo.Bazel.lock
COPY src/server/website/aft src/server/website/aft
COPY src/server/website/static src/server/website/static
# tailwind
COPY src/server/website/tailwind src/server/website/tailwind
COPY src/app/stem/front/yarn.lock src/app/stem/front/yarn.lock
COPY src/app/stem/front/package.json src/app/stem/front/package.json
COPY src/app/stem/front/BUILD src/app/stem/front/BUILD
# other items referened in the WORKSPACE
COPY src/app/stem/Cargo.lock src/app/stem/Cargo.lock
COPY src/app/stem/Cargo.Bazel.lock src/app/stem/Cargo.Bazel.lock
COPY src/app/stem/BUILD src/app/stem/BUILD



FROM debian:bookworm-slim as deps

# pkg-config causes tzdata to install, which prompts for timezone, so we set
# DEBIAN_FRONTEND to prevent the interactive prompt
RUN apt update && DEBIAN_FRONTEND=noninteractive apt install -y \
    wget g++ unzip zip pkg-config openssl libssl-dev

# $TARGETARCH gives either "amd64" or "arm64", which we use to get the right
# bazelisk binary. More info:
# https://www.docker.com/blog/faster-multi-platform-builds-dockerfile-cross-compilation-guide/
ARG TARGETARCH
# digitalocean might not set TARGETARCH, so we pick amd64 as the default value
ENV ARCH=${TARGETARCH:-amd64}
RUN wget https://github.com/bazelbuild/bazelisk/releases/download/v1.17.0/bazelisk-linux-$ARCH && \
    chmod 755 bazelisk-linux-$ARCH && \
    mv bazelisk-linux-$ARCH /usr/bin/bazel

# prompt bazelisk to download bazel
COPY --from=copier epsilon/.bazelversion .
RUN bazel version



FROM deps as builder

COPY --from=copier epsilon epsilon
WORKDIR epsilon

RUN bazel build //src/server/website:website_bin

RUN cp $(bazel cquery --output=starlark --starlark:expr=target.files_to_run.executable.path //src/server/website:website_bin) website_bin



FROM debian:bookworm-slim as runner

COPY --from=builder epsilon/website_bin .

RUN apt update \
    && DEBIAN_FRONTEND=noninteractive apt install -y --no-install-recommends \
        openssl ca-certificates libssl-dev \
    # clean up
    && apt autoremove -y \
    && apt clean -y

ENV INSIDE_DOCKER=true
EXPOSE 8000
ENTRYPOINT ./website_bin
