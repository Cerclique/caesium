#!/usr/bin/env bash
# Installs the extra tools CI uses, at pinned versions, then enables the Git hooks.
# Bump the versions here by hand: Dependabot doesn't track them.
set -euo pipefail

BINSTALL_VERSION=1.26.1
PREK_VERSION=0.5.7
CARGO_DENY_VERSION=0.20.2
CARGO_SEMVER_CHECKS_VERSION=0.51.0
GIT_CLIFF_VERSION=2.14.2

# cargo-binstall fetches prebuilt binaries instead of compiling each tool.
curl -fsSL "https://github.com/cargo-bins/cargo-binstall/releases/download/v${BINSTALL_VERSION}/cargo-binstall-$(uname -m)-unknown-linux-musl.tgz" |
    tar -xz -C "${CARGO_HOME:-$HOME/.cargo}/bin"

cargo binstall --no-confirm --disable-telemetry \
    "prek@${PREK_VERSION}" \
    "cargo-deny@${CARGO_DENY_VERSION}" \
    "cargo-semver-checks@${CARGO_SEMVER_CHECKS_VERSION}" \
    "git-cliff@${GIT_CLIFF_VERSION}"

prek install
