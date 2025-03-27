# Zelan Frontend

This is the frontend for Zelan, a Tauri application that integrates with streaming services like Twitch and OBS.

## Technology Stack

- **UI Framework**: React + TypeScript
- **State Management**: TanStack Query + TanStack Store
- **Routing**: TanStack Router (file-based routing)
- **Styling**: Tailwind CSS
- **Desktop Integration**: Tauri

## Project Structure

```
src/
├── components/ - Reusable UI components (kebab-case filenames)
├── hooks/ - Custom React hooks
├── lib/ - Utility functions
├── routes/ - File-based routing with TanStack Router
├── store/ - State management with TanStack Store
├── types/ - TypeScript type definitions
├── main.tsx - Entry point
└── styles.css - Global styles
```

## File-Based Routing

This project uses TanStack Router's file-based routing system. The routes are defined in the `src/routes` directory:

- `_layout.tsx` - Root layout component
- `index.tsx` - Dashboard route (/)
- `settings.tsx` - Settings route (/settings)
- `developer.tsx` - Developer tools route (/developer)

## State Management

State is managed using TanStack Query for server state (API calls to the Tauri backend) and TanStack Store for client state.

## Component Design

Components follow these conventions:

- Kebab-case filenames (e.g., `adapter-card.tsx`)
- Functional components with named exports
- Props interfaces defined inline or in types directory
- CSS with Tailwind utility classes

## Backend Integration

The frontend communicates with the Tauri backend using the `@tauri-apps/api` package. All backend calls are wrapped in custom hooks located in the `hooks` directory:

- `use-tauri-command.ts` - Wrapper around Tauri's invoke function
- `use-data-fetching.ts` - Data fetching hook with loading/error states
- `use-adapter-control.ts` - Hook for controlling adapters (connect/disconnect)

## Tauri Integration

Key integration points with the Tauri backend:

- **Commands**: Call backend functions defined in `src-tauri/src/plugin.rs`
- **Events**: Listen for events from the backend (WebSocket events, connection status changes)
- **WebSocket**: Interface with the Zelan WebSocket server for real-time updates

## Development Workflow

1. Make changes to the frontend code
2. Run `npm run dev` to see changes in development mode
3. When ready, run `npm run build` to create a production build
4. Use `npm run tauri dev` to test the full application with Tauri integration