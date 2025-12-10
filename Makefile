.PHONY: all build test test-all run-api run-cli clean fmt lint check db-client db-schema db-test-normalization help

# Default target
all: check build test

# Build all crates
build:
	cargo build

# Build release
build-release:
	cargo build --release

# Run tests (excludes tests that require database)
test:
	cargo test

# Run all tests including database integration tests
test-all:
	docker compose up -d
	sleep 2
	cargo test -- --include-ignored

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

# Connect to ClickHouse database
db-client:
	docker exec -it heimsight-clickhouse clickhouse-client -d heimsight

# Apply database schema
db-schema:
	@echo "Applying database schema..."
	docker compose exec -T clickhouse clickhouse-client --multiquery < schema/00_functions.sql
	docker compose exec -T clickhouse clickhouse-client --multiquery < schema/01_logs.sql
	docker compose exec -T clickhouse clickhouse-client --multiquery < schema/02_metrics.sql
	docker compose exec -T clickhouse clickhouse-client --multiquery < schema/03_traces.sql
	docker compose exec -T clickhouse clickhouse-client --multiquery < schema/04_aggregations.sql
	@echo "Schema applied successfully!"

# Test message normalization function
db-test-normalization:
	@echo "Testing message normalization..."
	docker compose exec -T clickhouse clickhouse-client --multiquery < schema/test_normalization.sql

# Help
help:
	@echo "Heimsight Development Commands"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  build             Build all crates (debug)"
	@echo "  build-release     Build all crates (release)"
	@echo "  test              Run tests (no database required)"
	@echo "  test-all          Run all tests including database tests"
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
	@echo "  db-client         Connect to ClickHouse database"
	@echo "  db-schema         Apply database schema files"
	@echo "  db-test-normalization  Test message normalization function"
	@echo "  help              Show this help message"
