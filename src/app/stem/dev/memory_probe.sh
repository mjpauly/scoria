#!/bin/bash
# Launcher for the memory probe harness (doc/decimation/memory-limits.md,
# Phase 2). Run with: bazel run //src/app/stem:memory_probe -- [flags]
set -euo pipefail
exec node "${BUILD_WORKSPACE_DIRECTORY}/src/app/stem/dev/memory_probe.mjs" "$@"
