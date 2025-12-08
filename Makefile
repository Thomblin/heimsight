.PHONY: all build test run-api run-cli clean fmt lint check help

# Default target
all: check build test

# Build all crates
build:
	cargo build

# Build release
build-release:
	cargo build --release

# Run all tests (skip doctests in generated protobuf code)
test:
	cargo test

# Run tests with output
test-verbose:
	cargo test -- --nocapture

# Run API server
run-api:
	docker compose up -d
	cargo run -p api

# Run API server with debug logging
run-api-debug:
	docker compose up -d
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
# Note: Generated protobuf code may have warnings, so we check without -D warnings
# but still enforce it for our own code via CI
lint:
	cargo clippy --all-targets

# Run clippy with strict warnings (for CI)
lint-strict:
	cargo clippy --workspace --all-targets -- -D warnings -A clippy::all

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

# connect to heimsight db
heimsight-client:
	docker exec -it heimsight-clickhouse clickhouse-client

# Help
help:
	@echo "Heimsight Development Commands"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  build             Build all crates (debug)"
	@echo "  build-release     Build all crates (release)"
	@echo "  test              Run all tests"
	@echo "  test-verbose      Run tests with output"
	@echo "  run-api           Run API server"
	@echo "  run-api-debug     Run API server with debug logging"
	@echo "  run-cli           Run CLI (use ARGS='...' for arguments)"
	@echo "  cli-health        Run CLI health check command"
	@echo "  fmt               Format code"
	@echo "  fmt-check         Check code formatting"
	@echo "  lint              Run clippy linter"
	@echo "  check             Run fmt-check, lint, and test"
	@echo "  clean             Clean build artifacts"
	@echo "  watch-test        Watch and run tests on changes"
	@echo "  watch-api         Watch and run API on changes"
	@echo "  docs              Build and open documentation"
	@echo "  heimsight-client  Connect to heimsight database"
	@echo "  help              Show this help message"
