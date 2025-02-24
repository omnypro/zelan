# Zelan

A lightweight, locally-hosted data aggregation service that ingests data from streaming platforms (Twitch, OBS, etc.) and exposes it through a standardized API. Built with Rust and Tauri, Zelan enables stream overlays and third-party applications to access unified data without direct coupling to source services.

## Purpose

Zelan eliminates redundant integration work by providing a single access point for streaming-related data. Instead of implementing and debugging authentication, OAuth flows, and API interactions for each service in every project, one can connect to Zelan's local API once and access data from all supported platforms through a consistent interface.

## Key Features

- Local hosting - no dependency on cloud services like Streamlabs or StreamElements
- Unified data model across multiple streaming platforms and services
- Configurable interface for adding/managing data sources
- Standardized API for stream overlays and tools to consume aggregated data
