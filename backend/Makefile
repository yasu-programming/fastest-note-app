# Makefile for fastest-note-backend

.PHONY: help dev build test lint format clean check

help: ## Show this help message
	@echo "Available commands:"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "\033[36m%-15s\033[0m %s\n", $$1, $$2}'

dev: ## Run development server with auto-reload  
	cargo watch -x run

build: ## Build the project in release mode
	cargo build --release

test: ## Run all tests
	cargo test

test-watch: ## Run tests in watch mode
	cargo watch -x test

lint: ## Run clippy lints
	cargo clippy --all-targets --all-features -- -D warnings

format: ## Format code with rustfmt
	cargo fmt

format-check: ## Check if code is formatted
	cargo fmt -- --check

check: lint format-check test ## Run all checks (lint, format, test)

clean: ## Clean build artifacts
	cargo clean

# Database commands
db-reset: ## Reset database (drop and recreate)
	sqlx database drop -y || true
	sqlx database create
	sqlx migrate run

db-migrate: ## Run database migrations
	sqlx migrate run

# Performance
benchmark: ## Run performance benchmarks  
	cargo bench

# Security
audit: ## Run security audit
	cargo audit

# Development setup
setup: ## Setup development environment
	rustup component add rustfmt clippy
	cargo install cargo-watch sqlx-cli
	@echo "Run 'cp .env.example .env' and configure your environment"