.PHONY: all build test run-api run-cli clean fmt lint check help

# Default target
all: check build test

# Build all crates
build:
	cargo build

# Build release
build-release:
	cargo build --release

# Run all tests
test:
	cargo test
	cargo test -p api
	cargo test -p heimsight

# Run tests with output
test-verbose:
	cargo test -- --nocapture

# Run API server
run-api:
	cargo run -p api

# Run API server with debug logging
run-api-debug:
	RUST_LOG=debug cargo run -p api

# Run CLI
run-cli:
	cargo run -p heimsight -- $(ARGS)

# Run CLI health check
cli-health:
	cargo run -p heimsight -- health

# Format code
fmt:
	cargo fmt

# Check formatting
fmt-check:
	cargo fmt --check

# Run clippy linter
lint:
	cargo clippy -- -D warnings

# Run all checks (format, lint, test)
check: fmt-check lint test

# Clean build artifacts
clean:
	cargo clean

# Watch and run tests on changes (requires cargo-watch)
watch-test:
	cargo watch -x test

# Watch and run API on changes (requires cargo-watch)
watch-api:
	cargo watch -x 'run -p api'

# Build documentation
docs:
	cargo doc --no-deps --open

# Help
help:
	@echo "Heimsight Development Commands"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  build          Build all crates (debug)"
	@echo "  build-release  Build all crates (release)"
	@echo "  test           Run all tests"
	@echo "  test-verbose   Run tests with output"
	@echo "  run-api        Run API server"
	@echo "  run-api-debug  Run API server with debug logging"
	@echo "  run-cli        Run CLI (use ARGS='...' for arguments)"
	@echo "  cli-health     Run CLI health check command"
	@echo "  fmt            Format code"
	@echo "  fmt-check      Check code formatting"
	@echo "  lint           Run clippy linter"
	@echo "  check          Run fmt-check, lint, and test"
	@echo "  clean          Clean build artifacts"
	@echo "  watch-test     Watch and run tests on changes"
	@echo "  watch-api      Watch and run API on changes"
	@echo "  docs           Build and open documentation"
	@echo "  help           Show this help message"
