default:
    just --list

# Generate an HTML test coverage report and open it in the browser
coverage *ARGS:
    cargo llvm-cov --version >/dev/null 2>&1 || cargo install cargo-llvm-cov
    rustup component add llvm-tools-preview
    cargo llvm-cov --html --open --all-features {{ ARGS }}

format:
    cargo +nightly fmt
    just --unstable --format
