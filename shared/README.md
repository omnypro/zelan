# Shared Types Directory

This directory contains centralized type definitions and schemas that are shared between the Electron (main process) and React (renderer process) code. 

## Purpose

The purpose of this shared types directory is to:

1. Eliminate circular dependencies by providing a single source of truth for types
2. Ensure type consistency across main and renderer processes
3. Simplify imports and increase type safety
4. Make the type system more maintainable

## Directory Structure

```
/shared
  /types
    adapters.ts   - Adapter-related types and schemas
    auth.ts       - Authentication-related types
    config.ts     - Application configuration types
    events.ts     - Event system types and constants
    index.ts      - Re-exports all types for convenient imports 
    tokens.ts     - Token and authorization types
    websocket.ts  - WebSocket message and client types
```

## Usage

Types from this directory should be imported using the `@shared` alias:

```typescript
// Import specific types
import { Token, TokenSchema } from '@shared/types/tokens';

// Import everything from a module
import * as AuthTypes from '@shared/types/auth';

// Import all types (use with caution to avoid large imports)
import { Token, AuthState, AdapterConfig } from '@shared/types';
```

## Best Practices

1. Always import types directly from this shared directory instead of from implementation files
2. Keep the type definitions minimal and focused on structure, not implementation
3. When adding new types, consider if they should be shared or if they are implementation details
4. Use Zod schemas where validation is needed
5. Keep the directory structure flat and organized by domain
6. Document types with JSDoc comments for better IDE support

## Deprecation Strategy

The old type locations in the codebase (e.g., `~/core/adapters/types.ts`) are now re-exporting from this shared directory for backward compatibility. These re-export files include a deprecation notice and should eventually be removed.