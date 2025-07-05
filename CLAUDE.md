# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Jikken is a Rust workspace containing tools for automated API testing. It uses YAML/JSON test definitions (`.jkt` files) to describe API tests with support for multi-stage testing, variable extraction, response validation, and more.

**Workspace structure:**
- `jikken-core`: Core library with all business logic
- `jikken-cli`: CLI application (`jk` binary)
- `jikken-gui`: Tauri-based GUI application

## Essential Commands

### Build Commands
```bash
# Build entire workspace
cargo build

# Build specific packages
cargo build -p jikken        # CLI only
cargo build -p jikken-core    # Core library only
cargo build -p jikken-gui     # GUI only

# Release builds
cargo build --release
cargo build -p jikken --release
```

### Running Tests
```bash
# Run all workspace tests
cargo test

# Run tests for specific package
cargo test -p jikken-core
cargo test -p jikken

# Run a single test
cargo test test_name
```

### Development Commands
```bash
# Format all code
cargo fmt

# Check formatting without applying
cargo fmt --check

# Run clippy linter
cargo clippy

# Run the CLI from source
cargo run -p jikken -- run example_tests/
cargo run --bin jk -- run example_tests/

# Run with specific tags
cargo run -p jikken -- run -t tag_name example_tests/
```

### Platform-Specific Builds
```bash
# Linux musl (static linking)
cargo build -p jikken --target x86_64-unknown-linux-musl --release

# macOS Intel
rustup target add x86_64-apple-darwin
SDKROOT=$(xcrun -sdk macosx --show-sdk-path) \
MACOSX_DEPLOYMENT_TARGET=$(xcrun -sdk macosx --show-sdk-platform-version) \
cargo build -p jikken --target=x86_64-apple-darwin --release

# macOS Apple Silicon
rustup target add aarch64-apple-darwin
SDKROOT=$(xcrun -sdk macosx --show-sdk-path) \
MACOSX_DEPLOYMENT_TARGET=$(xcrun -sdk macosx --show-sdk-platform-version) \
cargo build -p jikken --target=aarch64-apple-darwin --release
```

## High-Level Architecture

### Workspace Organization

1. **jikken-core** (`/jikken-core/`)
   - Pure library crate containing all core functionality
   - Modules: config, executor, test definitions, validation, json processing, telemetry
   - No binary entry points
   - Used by both CLI and potentially other frontends

2. **jikken-cli** (`/jikken-cli/`)
   - CLI application that consumes jikken-core
   - Entry point: `src/main.rs`
   - CLI-specific features: command parsing, test file creation, self-update
   - Published to crates.io as `jikken`

3. **jikken-gui** (`/jikken-gui/src-tauri/`)
   - Tauri-based GUI application
   - Independent implementation (doesn't use jikken-core)
   - Provides visual interface for test editing and execution

### Core Architecture (jikken-core)

**Key Modules:**
- `config.rs`: Configuration management (files, env vars, global variables)
- `executor.rs`: Test execution engine with different policies
- `test/`: Test definition structures and validation
- `json/`: JSON processing, extraction, and filtering
- `http.rs`: HTTP client and request/response handling
- `telemetry.rs`: Optional Jikken.io platform integration
- `errors.rs`: Error types and handling

**Execution Flow:**
1. Load configuration from multiple sources
2. Discover and parse `.jkt` test files
3. Validate test definitions
4. Execute tests according to policy (actual/dry-run)
5. Generate reports and output

### CLI Commands (jikken-cli)

- `run`: Execute tests (aliases: `r`)
- `dryrun`: Validate tests without execution (aliases: `dr`)
- `list`: List discovered test files
- `format`: Format test files (aliases: `fmt`)
- `validate`: Validate test syntax
- `new`: Create new test file from template
- `update`: Self-update the CLI

### Test Definition Format (.jkt)

YAML files with these key fields:
- `name`, `id`, `tags`: Test metadata
- `requires`: Test dependencies
- `variables`: Local and extracted variables
- `request`/`response`: HTTP definitions
- `stages`: Multi-stage test support
- `setup`/`cleanup`: Lifecycle hooks

Variables use `${variable}` syntax and support:
- Extraction from responses
- Global configuration
- Data generation
- Environment variables

## Important Conventions

1. **Workspace resolver**: Uses version "3" (latest Cargo workspace features)
2. **Binary names**: CLI is `jk`, GUI is `jk-gui`
3. **Package naming**: CLI publishes as `jikken` on crates.io
4. **Test files**: `.jkt` extension, YAML format
5. **Variable syntax**: `${variable_name}` (case-sensitive)
6. **Environment variables**: `JIKKEN_` prefix
7. **Config files**: `.jikken` in TOML format

## Development Workflow

1. Make changes in appropriate workspace member
2. Run `cargo fmt` to format code
3. Run `cargo clippy` for lints
4. Run `cargo test` to ensure tests pass
5. Test CLI changes: `cargo run -p jikken -- run example_tests/`
6. For release builds, use GitHub Actions workflows

## Key Files to Understand

**Core Library:**
- `jikken-core/src/executor.rs`: Test execution engine
- `jikken-core/src/test/definition.rs`: Test structures
- `jikken-core/src/config.rs`: Configuration system

**CLI Application:**
- `jikken-cli/src/main.rs`: CLI entry point and commands
- `jikken-cli/src/new.rs`: Test file generation

**Examples:**
- `example_tests/`: Various `.jkt` files demonstrating features