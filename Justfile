set positional-arguments

default:
    just --list

fmt:
    cargo clippy --fix --bin "cargo-spaced"
    cargo fmt --all

lint:
    cargo fmt -- --check
    cargo clippy --all-targets -- -D warnings
    cargo xwin clippy --all-targets -- -D warnings

test:
    cargo nextest run --all --no-tests=pass
    cargo xwin test --all --target x86_64-pc-windows-msvc

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

gen-completions:
    #!/usr/bin/env bash
    mkdir -p ./completions
    for shell in bash fish powershell zsh elvish; do
        ext=$([ "$shell" = "powershell" ] && echo "ps1" || echo "$shell")
        cargo run --bin completions --features="shell-completions" -- "$shell" \
            > "./completions/completions.$ext"
    done
