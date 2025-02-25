# Zelan Project Guide

## Build Commands
- `bun dev` - Start Vite development server
- `bun tauri dev` - Start Tauri app in development mode
- `bun build` - Build the frontend (TypeScript + Vite)
- `bun tauri build` - Build the full Tauri application
- `cargo test` - Run Rust tests in the src-tauri directory
- `cargo test [test_name]` - Run a specific Rust test
- `cargo clippy` - Run Rust linter

## Environment Setup
This project requires environment variables for certain features to work properly.

### Twitch Integration
Requires the `TWITCH_CLIENT_ID` environment variable to be set in a `.env` file.
See `.env.example` for reference and instructions to obtain a client ID.

## Code Style Guidelines

### TypeScript/React
- Use React functional components with hooks
- Prefer explicit type annotations with TypeScript
- Use async/await for asynchronous operations
- Group related imports together (React, internal, external)

### Rust
- Follow standard Rust naming conventions (snake_case for functions/variables)
- Use the `anyhow` crate for error handling
- Implement proper error propagation with `?` operator
- Use async/await for asynchronous code with Tokio runtime
- Favor Arc<T> for shared ownership when needed
- Implement Clone trait for components requiring shared ownership

### General
- Document public APIs and complex functions
- Handle errors explicitly rather than panicking
- Use consistent formatting (2-space indentation for TS/JS, 4-space for Rust)
- Prefer the simplest solution that uses what the libraries provide
- Do not recreate functionality unless absolutely necessary
