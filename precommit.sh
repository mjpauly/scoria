#!/bin/bash
set -euo pipefail
IFS=$'\n\t'

printf "\nRunning unit tests\n\n"

bazel test //src/app/stem:unit_tests

printf "\nRunning integration tests\n\n"

bazel test //src/app/stem:int_tests --spawn_strategy=local --test_output=all

printf "\nChecking Clippy lints\n\n"

# Sometimes the db synchronization files -wal and -shm get deleted, so we do an
# extra db_gen so compilation succeeds. Though sometimes this still doesn't fix
# it, in which case you'll want to run :db_gen from the command line directly.
bazel run //src/app/stem:db_gen
bazel build --aspects=@rules_rust//rust:defs.bzl%rust_clippy_aspect --output_groups=clippy_checks //src/app/...
# need --config=linux_amd64 to build the :image_amd64 target, which isn't
# perfect, but we do this for now:
bazel build --aspects=@rules_rust//rust:defs.bzl%rust_clippy_aspect --output_groups=clippy_checks //src/server/... --config=linux_amd64

printf "\nChecking formatting\n\n"

bazel build --@rules_rust//:rustfmt.toml=//:rustfmt.toml --aspects=@rules_rust//rust:defs.bzl%rustfmt_aspect --output_groups=rustfmt_checks //src/app/...
bazel build --@rules_rust//:rustfmt.toml=//:rustfmt.toml --aspects=@rules_rust//rust:defs.bzl%rustfmt_aspect --output_groups=rustfmt_checks //src/server/... --config=linux_amd64

printf "\nPrecommit passed ✅\n\n"
