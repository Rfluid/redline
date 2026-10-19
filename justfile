# redline task runner. `just` lists recipes; `just check` is what CI runs.

default:
    @just --list

# Format Rust sources
fmt:
    cargo fmt

# Lint: rustfmt check, clippy (warnings are errors), JS syntax, design tokens (contrast + sync)
lint:
    cargo fmt --check
    cargo clippy --all-targets -- -D warnings
    for f in web/*.js tests/browser/*.mjs tests/bindings/*.mjs bindings/react/*.js; do node --check "$f"; done
    python3 .design/check.py

# Dependency audit (deny.toml): advisories, licenses, bans, sources. Needs `cargo install cargo-deny`
deny:
    cargo deny check

# Unit tests (Rust, and the JS bindings under node:test)
test:
    cargo test
    node --test tests/bindings/*.test.mjs

# End-to-end test (temp state dir, random port)
e2e:
    cargo build --release
    bash tests/e2e.sh

# Browser tests in headless Chrome (CHROME=/path to override): overlay, and the Streamlit binding if installed
browser:
    cargo build --release
    node tests/browser/overlay.test.mjs
    node tests/browser/streamlit.test.mjs
    node tests/browser/react.test.mjs

# Regenerate the README screenshots in docs/screenshots/ (sandboxed daemon, headless Chrome)
screenshots:
    cargo build --release
    node docs/screenshots/shoot.mjs

# Everything CI checks
check: lint test e2e browser

# Bump versions + CHANGELOG for the next release (args: --bump minor, --pre beta, --dry-run, ...)
prepare-release *args:
    scripts/prepare-release.sh {{args}}

# Run a sandboxed daemon in the foreground (never touches ~/.local/state/redline)
dev port="7499":
    REDLINE_HOME="$(mktemp -d)" cargo run -- serve --port {{port}}
