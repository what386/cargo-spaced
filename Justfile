set positional-arguments

default:
    just --list

fmt:
    cargo clippy --fix --bin "cargo-spaced"
    cargo fmt --all

lint:
    cargo fmt -- --check
    cargo clippy --all-targets -- -D warnings

test:
    cargo nextest run --all --no-tests=pass

verify-release:
    just lint
    just test

run *args:
    cargo run --bin "cargo-spaced" -- {{args}}


prepare version:
    scripts/release/prepare.sh {{version}}

promote:
    scripts/release/promote.sh

publish version:
    scripts/release/publish.sh {{version}}
    git switch dev
    printf "ready" > .release-state
