BIN_NAME := playmaster

NAME := $(shell grep 'name =' Cargo.toml | head -n 1 | cut -d'"' -f2)
VERSION := $(shell grep '^version =' Cargo.toml | head -n 1 | cut -d'"' -f2)
ARCH ?= $(shell uname -m)
TARGET_ARCH ?= $(ARCH)-unknown-linux-gnu
ALL_RS := $(shell find src -name '*.rs')
PREFIX ?= /usr
CACHE_DIR := .cache

# Docker image variables
IMAGE_NAME ?= $(BIN_NAME)-builder
IMAGE_TAG ?= latest

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

# ----- Run Gmail Code Refresh -----
gmail:
	@echo "🚀 Running $(BIN_NAME)..."
	cargo run -- gmail

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

.PHONY: debug
debug: target/$(TARGET_ARCH)/debug/$(NAME)  ## Build debug build
target/$(TARGET_ARCH)/debug/$(NAME): $(ALL_RS) Cargo.lock Cargo.toml
	cargo build --target $(TARGET_ARCH)

.PHONY: build
build: target/$(TARGET_ARCH)/release/$(NAME) ## Build release build
target/$(TARGET_ARCH)/release/$(NAME): $(ALL_RS) Cargo.lock Cargo.toml
	cargo build --release --target $(TARGET_ARCH)

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
	rm -rf dist target $(CACHE_DIR)
	cargo clean

# Refer to .releaserc.yaml for release configuration
.PHONY: sem-release 
sem-release: ## Publish a release with semantic release 
	npx semantic-release

.PHONY: dist
dist: dist/$(NAME)-$(ARCH).tar.gz dist/$(NAME)-$(VERSION)-1.$(ARCH).rpm ## Create all redistributable versions of the project

.PHONY: dist-archive
dist-archive: dist/$(NAME)-$(ARCH).tar.gz ## Build a redistributable archive of the project
dist/$(NAME)-$(ARCH).tar.gz: build
	rm -rf $(CACHE_DIR)/$(BIN_NAME)
	mkdir -p $(CACHE_DIR)/$(BIN_NAME)
	$(MAKE) install PREFIX=$(CACHE_DIR)/$(BIN_NAME)/usr NO_RELOAD=true
	mkdir -p dist
	tar cvfz $@ -C $(CACHE_DIR) $(BIN_NAME)
	cd dist && sha256sum $(BIN_NAME)-$(ARCH).tar.gz > $(BIN_NAME)-$(ARCH).tar.gz.sha256.txt

.PHONY: dist-rpm
dist-rpm: dist/$(NAME)-$(VERSION)-1.$(ARCH).rpm ## Build a redistributable RPM package
dist/$(NAME)-$(VERSION)-1.$(ARCH).rpm: target/$(TARGET_ARCH)/release/$(NAME)
	mkdir -p dist
	cargo install cargo-generate-rpm
	cargo generate-rpm --target $(TARGET_ARCH)
	cp ./target/$(TARGET_ARCH)/generate-rpm/$(NAME)-$(VERSION)-1.$(ARCH).rpm dist
	cd dist && sha256sum $(NAME)-$(VERSION)-1.$(ARCH).rpm > $(NAME)-$(VERSION)-1.$(ARCH).rpm.sha256.txt

# E.g. make in-docker TARGET=build
.PHONY: in-docker
in-docker:
	@# Run the given make target inside Docker
	docker build -t $(IMAGE_NAME):$(IMAGE_TAG) .
	docker run --rm \
		-v $(PWD):/src \
		--workdir /src \
		-e HOME=/home/build \
		-e ARCH=$(ARCH) \
		--user $(shell id -u):$(shell id -g) \
		$(IMAGE_NAME):$(IMAGE_TAG) \
		make $(TARGET)
