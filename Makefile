.PHONY: run build debug release clean check fmt lint test watch

# Default: build and run
run: build
	./target/debug/mini-pond

# Debug build
build:
	cargo build

# Debug build with RUST_BACKTRACE
debug:
	RUST_BACKTRACE=1 cargo run

# Release build (optimized)
release:
	cargo build --release

# Run release binary
run-release: release
	./target/release/mini-pond

# Check compilation without building
check:
	cargo check

# Format code
fmt:
	cargo fmt

# Lint with clippy
lint:
	cargo clippy -- -D warnings

# Run tests
test:
	cargo test

# Clean build artifacts
clean:
	cargo clean

# Watch: rebuild and run on file change
watch:
	cargo watch -x run

# Format + lint + check
ci: fmt lint check test
