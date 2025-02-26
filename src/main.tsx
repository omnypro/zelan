import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import "./App.css";

// Import Tauri event APIs
import { emit } from '@tauri-apps/api/event';

// Simple app initialization
ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);

// Signal to Tauri that frontend is loaded (for debugging/logging purposes)
emit('frontend-loaded', { status: 'ready' })
  .catch(e => console.error('Failed to emit frontend-loaded event:', e));