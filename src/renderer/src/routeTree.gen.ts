/* eslint-disable */

// @ts-nocheck

// noinspection JSUnusedGlobalSymbols

// This file was automatically generated by TanStack Router.
// You should NOT make any changes in this file as it will be overwritten.
// Additionally, you should also exclude this file from your linter and/or formatter to prevent it from being checked or modified.

// Import Routes

import { Route as rootRoute } from './routes/__root'
import { Route as WebsocketImport } from './routes/websocket'
import { Route as TrpcImport } from './routes/trpc'
import { Route as SettingsImport } from './routes/settings'
import { Route as EventsImport } from './routes/events'
import { Route as EnhancedEventsImport } from './routes/enhanced-events'
import { Route as AuthImport } from './routes/auth'
import { Route as IndexImport } from './routes/index'

// Create/Update Routes

const WebsocketRoute = WebsocketImport.update({
  id: '/websocket',
  path: '/websocket',
  getParentRoute: () => rootRoute
} as any)

const TrpcRoute = TrpcImport.update({
  id: '/trpc',
  path: '/trpc',
  getParentRoute: () => rootRoute
} as any)

const SettingsRoute = SettingsImport.update({
  id: '/settings',
  path: '/settings',
  getParentRoute: () => rootRoute
} as any)

const EventsRoute = EventsImport.update({
  id: '/events',
  path: '/events',
  getParentRoute: () => rootRoute
} as any)

const EnhancedEventsRoute = EnhancedEventsImport.update({
  id: '/enhanced-events',
  path: '/enhanced-events',
  getParentRoute: () => rootRoute
} as any)

const AuthRoute = AuthImport.update({
  id: '/auth',
  path: '/auth',
  getParentRoute: () => rootRoute
} as any)

const IndexRoute = IndexImport.update({
  id: '/',
  path: '/',
  getParentRoute: () => rootRoute
} as any)

// Populate the FileRoutesByPath interface

declare module '@tanstack/react-router' {
  interface FileRoutesByPath {
    '/': {
      id: '/'
      path: '/'
      fullPath: '/'
      preLoaderRoute: typeof IndexImport
      parentRoute: typeof rootRoute
    }
    '/auth': {
      id: '/auth'
      path: '/auth'
      fullPath: '/auth'
      preLoaderRoute: typeof AuthImport
      parentRoute: typeof rootRoute
    }
    '/enhanced-events': {
      id: '/enhanced-events'
      path: '/enhanced-events'
      fullPath: '/enhanced-events'
      preLoaderRoute: typeof EnhancedEventsImport
      parentRoute: typeof rootRoute
    }
    '/events': {
      id: '/events'
      path: '/events'
      fullPath: '/events'
      preLoaderRoute: typeof EventsImport
      parentRoute: typeof rootRoute
    }
    '/settings': {
      id: '/settings'
      path: '/settings'
      fullPath: '/settings'
      preLoaderRoute: typeof SettingsImport
      parentRoute: typeof rootRoute
    }
    '/trpc': {
      id: '/trpc'
      path: '/trpc'
      fullPath: '/trpc'
      preLoaderRoute: typeof TrpcImport
      parentRoute: typeof rootRoute
    }
    '/websocket': {
      id: '/websocket'
      path: '/websocket'
      fullPath: '/websocket'
      preLoaderRoute: typeof WebsocketImport
      parentRoute: typeof rootRoute
    }
  }
}

// Create and export the route tree

export interface FileRoutesByFullPath {
  '/': typeof IndexRoute
  '/auth': typeof AuthRoute
  '/enhanced-events': typeof EnhancedEventsRoute
  '/events': typeof EventsRoute
  '/settings': typeof SettingsRoute
  '/trpc': typeof TrpcRoute
  '/websocket': typeof WebsocketRoute
}

export interface FileRoutesByTo {
  '/': typeof IndexRoute
  '/auth': typeof AuthRoute
  '/enhanced-events': typeof EnhancedEventsRoute
  '/events': typeof EventsRoute
  '/settings': typeof SettingsRoute
  '/trpc': typeof TrpcRoute
  '/websocket': typeof WebsocketRoute
}

export interface FileRoutesById {
  __root__: typeof rootRoute
  '/': typeof IndexRoute
  '/auth': typeof AuthRoute
  '/enhanced-events': typeof EnhancedEventsRoute
  '/events': typeof EventsRoute
  '/settings': typeof SettingsRoute
  '/trpc': typeof TrpcRoute
  '/websocket': typeof WebsocketRoute
}

export interface FileRouteTypes {
  fileRoutesByFullPath: FileRoutesByFullPath
  fullPaths: '/' | '/auth' | '/enhanced-events' | '/events' | '/settings' | '/trpc' | '/websocket'
  fileRoutesByTo: FileRoutesByTo
  to: '/' | '/auth' | '/enhanced-events' | '/events' | '/settings' | '/trpc' | '/websocket'
  id: '__root__' | '/' | '/auth' | '/enhanced-events' | '/events' | '/settings' | '/trpc' | '/websocket'
  fileRoutesById: FileRoutesById
}

export interface RootRouteChildren {
  IndexRoute: typeof IndexRoute
  AuthRoute: typeof AuthRoute
  EnhancedEventsRoute: typeof EnhancedEventsRoute
  EventsRoute: typeof EventsRoute
  SettingsRoute: typeof SettingsRoute
  TrpcRoute: typeof TrpcRoute
  WebsocketRoute: typeof WebsocketRoute
}

const rootRouteChildren: RootRouteChildren = {
  IndexRoute: IndexRoute,
  AuthRoute: AuthRoute,
  EnhancedEventsRoute: EnhancedEventsRoute,
  EventsRoute: EventsRoute,
  SettingsRoute: SettingsRoute,
  TrpcRoute: TrpcRoute,
  WebsocketRoute: WebsocketRoute
}

export const routeTree = rootRoute._addFileChildren(rootRouteChildren)._addFileTypes<FileRouteTypes>()

/* ROUTE_MANIFEST_START
{
  "routes": {
    "__root__": {
      "filePath": "__root.tsx",
      "children": [
        "/",
        "/auth",
        "/enhanced-events",
        "/events",
        "/settings",
        "/trpc",
        "/websocket"
      ]
    },
    "/": {
      "filePath": "index.tsx"
    },
    "/auth": {
      "filePath": "auth.tsx"
    },
    "/enhanced-events": {
      "filePath": "enhanced-events.tsx"
    },
    "/events": {
      "filePath": "events.tsx"
    },
    "/settings": {
      "filePath": "settings.tsx"
    },
    "/trpc": {
      "filePath": "trpc.tsx"
    },
    "/websocket": {
      "filePath": "websocket.tsx"
    }
  }
}
ROUTE_MANIFEST_END */
