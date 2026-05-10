CARGO	:= cargo
N		?= 2 #For superopt

# Default target
.DEFAULT_GOAL := help

.PHONY: all build test fmt lint clean fclean release superopt help

all: fmt lint test build ## Run formatting, linting, tests, and build the project

build: ## Build the project in debug mode and copy to root
	$(CARGO) build
	cp target/debug/push_swap .
	cp target/debug/checker .

test: ## Run the test suite
	$(CARGO) test

fmt: ## Format the code using rustfmt
	$(CARGO) fmt --all

lint: ## Run clippy to lint the code
	$(CARGO) clippy --all-targets --all-features -- -D warnings

clean: ## Remove build artifacts
	rm -f push_swap checker

fclean: clean ## Remove the target directory and build artifacts
	$(CARGO) clean

release: ## Build the project in release mode (optimized)  and copy to root
	$(CARGO) build --release
	cp target/release/push_swap .
	cp target/release/checker .

superopt: ## Build and run the superopt binary in release mode, use: make superopt N=5
	$(CARGO) run --bin superopt --release -- $(N)

clean-cache: ## Replace the current superopt cache with an empty file
	echo "{}" > superopt_cache.json

help: ## Print this help message
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "  %-12s %s\n", $$1, $$2}' $(MAKEFILE_LIST)
