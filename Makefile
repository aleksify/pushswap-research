CARGO = cargo
PROJECT_NAME = push_swap_rs

# Default target
.DEFAULT_GOAL := help

.PHONY: all build run run-checker test fmt lint clean release help

all: fmt lint test build ## Run formatting, linting, tests, and build the project

build: ## Build the project in debug mode and copy to root
	$(CARGO) build
	cp target/debug/push_swap_rs ./push_swap
	cp target/debug/checker .

run: ## Run push_swap (pass ARGS, e.g. make run ARGS="3 2 1")
	$(CARGO) run --bin $(PROJECT_NAME) -- $(ARGS)

run-checker: ## Run checker (pass ARGS, e.g. make checker ARGS="3 2 1")
	$(CARGO) run --bin checker -- $(ARGS)

test: ## Run the test suite
	$(CARGO) test

fmt: ## Format the code using rustfmt
	$(CARGO) fmt --all

lint: ## Run clippy to lint the code
	$(CARGO) clippy --all-targets --all-features -- -D warnings

clean: ## Remove the target directory and build artifacts
	$(CARGO) clean
	rm -f push_swap checker

release: ## Build the project in release mode (optimized)  and copy to root
	$(CARGO) build --release
	cp target/release/push_swap_rs ./push_swap
	cp target/release/checker .

help: ## Print this help message
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-12s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
