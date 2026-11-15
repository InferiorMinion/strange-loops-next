# The CI image for the model gate: Node 22, elan with the pinned Lean toolchain,
# and Rust for the loops-eval build: the release rust/rust-toolchain.toml names,
# with the components it lists, so that no job downloads them.
#
# .gitlab-ci.yml builds it with LEAN_TOOLCHAIN read from model/lean-toolchain,
# and rebuilds it whenever this file or that one changes. If the two ever
# disagree, elan installs the toolchain the checkout asks for, so a stale image
# is slower but never wrong.

FROM node:22-bookworm@sha256:7702d8aa4d5910871692af4480eb3ba86f23047a6417a44d857d06c95d060d3f

ARG LEAN_TOOLCHAIN

ENV ELAN_HOME=/opt/elan \
    RUSTUP_HOME=/opt/rustup \
    CARGO_HOME=/opt/cargo \
    PATH=/opt/elan/bin:/opt/cargo/bin:$PATH

RUN test -n "$LEAN_TOOLCHAIN" \
 && curl -sSfL https://raw.githubusercontent.com/leanprover/elan/master/elan-init.sh \
    | sh -s -- -y --no-modify-path --default-toolchain "$LEAN_TOOLCHAIN" \
 && curl -sSf https://sh.rustup.rs \
    | sh -s -- -y --no-modify-path --profile minimal --default-toolchain 1.100.0 \
         --component rustfmt --component clippy \
 && lean --version && cargo --version && node --version
