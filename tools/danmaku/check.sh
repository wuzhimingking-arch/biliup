#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../.."
export CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
export CARGO_PROFILE_RELEASE_LTO=false
export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
export CARGO_PROFILE_RELEASE_OPT_LEVEL=1

rustfmt --edition 2024 --check --config skip_children=true \
  crates/danmaku/src/{client.rs,client_diagnostics_tests.rs,error.rs,output/xml.rs,protocols/{mod.rs,bilibili.rs,bilibili_diagnostics_tests.rs}} \
  crates/danmaku/examples/record_probe.rs \
  crates/biliup-cli/src/server/infrastructure/context.rs
cargo test -p danmaku --locked
BILIUP_LOCAL_DANMAKU_TEST=0 cargo test --release -p biliup-cli --lib --locked \
  --config profile.release.package.biliup-cli.opt-level=0 server::infrastructure::context::tests
env -u BILIUP_LOCAL_DANMAKU_TEST cargo test --release -p biliup-cli --lib --locked \
  --config profile.release.package.biliup-cli.opt-level=0 formal_mode_keeps_upload_config_and_recording_hooks
cargo check -p danmaku --examples --features reqwest/gzip,reqwest/brotli --locked
cargo build --release -p biliup-cli --bin biliup --locked
