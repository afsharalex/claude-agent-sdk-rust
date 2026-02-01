# Claude Agent SDK Rust - Build and Test Commands

# Build the project
build:
    cargo build

# Build release version
release:
    cargo build --release

# Format code
fmt:
    cargo fmt

# Check formatting
fmt-check:
    cargo fmt -- --check

# Run clippy linter
clippy:
    cargo clippy -- -D warnings

# Run all checks (fmt, clippy, test)
check: fmt-check clippy test

# Run tests
test:
    cargo test

# Run tests with verbose output
test-verbose:
    cargo test -- --nocapture

# Run specific test
test-one TEST:
    cargo test {{TEST}} -- --nocapture

# Run integration tests (requires Claude CLI installed)
test-integration:
    cargo test -- --ignored --nocapture

# Run all tests including integration tests
test-all:
    cargo test -- --include-ignored

# Generate code coverage report (requires cargo-llvm-cov)
coverage:
    cargo llvm-cov --lib --html --output-dir coverage --ignore-filename-regex "examples/.*"

# Generate documentation
docs:
    cargo doc --no-deps --open

# Run CI checks
ci: fmt-check clippy test

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and rebuild (requires cargo-watch)
watch:
    cargo watch -x build

# Watch for changes and run tests (requires cargo-watch)
watch-test:
    cargo watch -x test

# List all available commands
default:
    @just --list
