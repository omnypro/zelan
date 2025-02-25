# Zelan Project Guide

## Project Overview
Zelan is a locally-hosted data aggregation service for streaming platforms that unifies data from various sources (Twitch, OBS, etc.) and exposes it through a standardized API. It's built with Rust for the backend (using Tauri) and TypeScript/React for the frontend.

## Build Commands
- `bun dev` - Start Vite development server
- `bun tauri dev` - Start Tauri app in development mode
- `bun build` - Build the frontend (TypeScript + Vite)
- `bun tauri build` - Build the full Tauri application
- `cargo test` - Run Rust tests in the src-tauri directory
- `cargo test [test_name]` - Run a specific Rust test
- `cargo clippy` - Run Rust linter
- `cargo fmt` - Format Rust code

## Environment Setup
This project requires environment variables for certain features to work properly.

### Twitch Integration
- Requires the `TWITCH_CLIENT_ID` environment variable to be set in a `.env` file
- Uses device code flow authentication (suitable for desktop applications)
- Only requests minimum necessary scopes for read operations
- See `.env.example` for reference and instructions to obtain a client ID

## Architecture

### Adapter System
- **BaseAdapter**: Foundation for all service adapters with common functionality
- **ServiceAdapter**: Core interface all adapters must implement
- Each adapter handles a specific platform:
  - **TwitchAdapter**: Connects to Twitch API and monitors channel/stream events
  - **ObsAdapter**: Connects to OBS via WebSockets for scene information
  - **TestAdapter**: Generates test events for debugging

### Event-Driven Design
- Adapters publish events to a central EventBus
- Events are propagated to subscribers (UI components, other services)
- Async/await patterns used for non-blocking operations

## Code Style Guidelines

### TypeScript/React
- Use React functional components with hooks
- Prefer explicit type annotations with TypeScript
- Use async/await for asynchronous operations
- Group related imports together (React, internal, external)
- Follow 2-space indentation

### Rust
- Follow standard Rust naming conventions (snake_case for functions/variables)
- Use the `anyhow` crate for error handling
- Implement proper error propagation with `?` operator
- Use async/await for asynchronous code with Tokio runtime
- Favor Arc<RwLock<T>> for shared mutable state
- Implement Clone trait for components requiring shared ownership
- Follow 4-space indentation
- Use structs and traits for adapter interfaces

### Authentication
- Prefer OAuth device code flow for desktop applications
- Request minimal scopes necessary for functionality
- Handle token refreshing and restoration automatically
- Store tokens securely using Tauri's secure storage when possible

### General
- Document public APIs and complex functions
- Handle errors explicitly rather than panicking
- Prefer the simplest solution that uses what the libraries provide
- Do not recreate functionality unless absolutely necessary
- Write tests for critical components
- Use cargo fmt and appropriate linters before committing
