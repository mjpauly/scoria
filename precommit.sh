#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

bazel test //src/app/stem:unit_tests
bazel test //src/app/stem:int_tests --spawn_strategy=local
bazel build --aspects=@rules_rust//rust:defs.bzl%rust_clippy_aspect --output_groups=clippy_checks //...
bazel build --@rules_rust//:rustfmt.toml=//:rustfmt.toml --aspects=@rules_rust//rust:defs.bzl%rustfmt_aspect --output_groups=rustfmt_checks //...
