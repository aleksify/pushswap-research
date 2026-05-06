CARGO = cargo
PROJECT_NAME = push_swap_rs

# Default target
.DEFAULT_GOAL := help

.PHONY: all build run test fmt lint clean release help

all: fmt lint test build ## Run formatting, linting, tests, and build the project

build: ## Build the project in debug mode
	$(CARGO) build

run: ## Run the project in debug mode
	$(CARGO) run

test: ## Run the test suite
	$(CARGO) test

fmt: ## Format the code using rustfmt
	$(CARGO) fmt --all

lint: ## Run clippy to lint the code
	$(CARGO) clippy --all-targets --all-features -- -D warnings

clean: ## Remove the target directory and build artifacts
	$(CARGO) clean

release: ## Build the project in release mode (optimized)
	$(CARGO) build --release

help: ## Print this help message
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-12s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
