BIN_NAME := playmaster

# Default target
.DEFAULT_GOAL := help

# ----- Utility -----
help:
	@echo "Available commands:"
	@echo "  make schema     - Generate JSON schemas"
	@echo "  make gen        - Run code generation (e.g. Dart)"
	@echo "  make run        - Run the binary"
	@echo "  make install    - Install globally"
	@echo "  make clippy     - Run Clippy lints (with warnings as errors)"
	@echo "  make fmt        - Format all Rust code"
	@echo "  make check      - Check build without compiling"
	@echo "  make clean      - Clean target directory"

# ----- Schema Generation -----
schema:
	@echo "🔧 Generating schemas..."
	cargo run -- schema

# ----- Code Generation -----
gen:
	@echo "🧩 Running code generation..."
	cd ./samples/flutter_sample_app && cargo run -- gen

# ----- Run CLI -----
run:
	@echo "🚀 Running $(BIN_NAME)..."
	cd ./samples/flutter_sample_app && cargo run -- run --mode local

# ----- Setup Tasks -----
setup:
	@echo "⚙️  Running setup tasks..."
	cd ./samples/flutter_sample_app && cargo run -- run --mode local --setup

# ----- Install Globally -----
install:
	@echo "📦 Installing $(BIN_NAME) globally..."
	cargo install --path . --force

# ----- Linting -----
clippy:
	@echo "🧹 Running Clippy..."
	cargo clippy --all-targets --all-features -- -D warnings

# ----- Formatting -----
fmt:
	@echo "🎨 Formatting code..."
	cargo fmt --all

# ----- Build Check -----
check:
	@echo "🔍 Checking build..."
	cargo check --all

# ----- Clean -----
clean:
	@echo "🧽 Cleaning project..."
	cargo clean
