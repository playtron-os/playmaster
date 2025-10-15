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
test:
	@echo "🚀 Running $(BIN_NAME)..."
	cd ./samples/flutter_sample_app && cargo run -- run

# ----- Run CLI Local -----
test-local:
	@echo "🚀 Running $(BIN_NAME)..."
	cd ./samples/flutter_sample_app && cargo run -- run --mode local -y

# ----- Run CLI Remote -----
test-remote:
	@echo "🚀 Running $(BIN_NAME)..."
	cd ./samples/flutter_sample_app && cargo run -- run --mode remote -y

# ----- Run in Fedora Container -----
test-fedora:
	@echo "🚀 Running $(BIN_NAME) in Fedora container..."
	# Check if container is already running
	@if [ "$$(docker ps -q -f name=playmaster-fedora)" ]; then \
		echo "⚡ Container 'playmaster-fedora' is already running."; \
	else \
		echo "📦 Building Docker image..."; \
		docker build -t playmaster-fedora -f ./testing/Dockerfile.fedora .; \
		echo "🚀 Starting container..."; \
		@xhost +si:localuser:root; \
		docker run -d --rm \
			-u $(id -u):$(id -g) \
			-e WAYLAND_DISPLAY=${WAYLAND_DISPLAY} \
			-e XDG_RUNTIME_DIR=${XDG_RUNTIME_DIR} \
			-v ${XDG_RUNTIME_DIR}/${WAYLAND_DISPLAY}:${XDG_RUNTIME_DIR}/${WAYLAND_DISPLAY} \
			-v /run/user/$(id -u)/bus:/run/user/$(id -u)/bus \
			--name playmaster-fedora -p 2222:22 playmaster-fedora; \
	fi
	export REMOTE_PASSWORD=dev; \
	cd ./samples/flutter_sample_app && cargo run -- run --mode remote --remote-addr dev@localhost:2222 -y

# ----- Run in GameOS Container -----
# test-gameos:
# 	@echo "🚀 Running $(BIN_NAME) in GameOS container..."
# 	# Check if container is already running
# 	@if [ "$$(docker ps -q -f name=playmaster-gameos)" ]; then \
# 		echo "⚡ Container 'playmaster-gameos' is already running."; \
# 	else \
# 		echo "📦 Building Docker image..."; \
# 		docker build -t playmaster-gameos -f ./testing/Dockerfile.gameos .; \
# 		echo "🚀 Starting container..."; \
# 		docker run -d --rm --name playmaster-gameos -p 2222:22 playmaster-gameos; \
# 	fi
# 	export REMOTE_PASSWORD=dev; \
# 	cd ./samples/flutter_sample_app && cargo run -- run --mode remote --remote-addr dev@localhost:2222 -y

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
