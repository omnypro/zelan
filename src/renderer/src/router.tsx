import {
  Link,
  Outlet,
  RootRoute,
  Route,
  Router,
  RouterProvider,
  createHashHistory,
  useMatchRoute
} from '@tanstack/react-router'

// Import your components
import EventsDemo from './components/EventsDemo'
import EnhancedEventsDemo from './components/EnhancedEventsDemo'
import AdapterStatus from './components/AdapterStatus'
import EnhancedAdapterStatus from './components/EnhancedAdapterStatus'
import Settings from './components/Settings'
import AuthDemo from './components/AuthDemo'
import { TrpcDemo } from './components/TrpcDemo'
import { WebSocketDemo } from './components/demos/WebSocketDemo'

// Import assets
import electronLogo from './assets/electron.svg'

// Navigation link component with active state
function NavLink({ to, label }: { to: string; label: string }) {
  const matchRoute = useMatchRoute()
  const isActive = matchRoute({ to, fuzzy: to !== '/' })
  
  return (
    <li>
      <Link
        to={to}
        className={`block w-full text-left px-4 py-2 ${
          isActive ? 'bg-blue-600 text-white' : 'text-gray-300 hover:bg-gray-600'
        }`}
        activeProps={{ className: 'bg-blue-600 text-white' }}
      >
        {label}
      </Link>
    </li>
  )
}

// Define the root layout component
function RootLayout() {
  return (
    <div className="min-h-screen bg-gray-100 text-black flex flex-col">
      {/* Header is shared across all routes */}
      <header className="bg-gray-800 text-white p-4 flex items-center shadow-md">
        <img alt="logo" className="h-8 w-8 mr-3" src={electronLogo} />
        <h1 className="text-xl font-semibold">Zelan</h1>
        <p className="ml-4 text-gray-400 text-sm">Stream Data Aggregation Service</p>
      </header>

      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar Navigation */}
        <nav className="w-48 bg-gray-700 text-white">
          <ul className="py-4">
            <NavLink to="/" label="Dashboard" />
            <NavLink to="/events" label="Events" />
            <NavLink to="/enhanced-events" label="Enhanced Events" />
            <NavLink to="/auth" label="Authentication" />
            <NavLink to="/settings" label="Settings" />
            <NavLink to="/trpc" label="tRPC Demo" />
            <NavLink to="/websocket" label="WebSocket" />
          </ul>

          <div className="mt-auto p-4 text-xs text-gray-400">
            <p>
              <a 
                className="text-blue-400 hover:underline cursor-pointer" 
                onClick={() => window.electron.ipcRenderer.send('ping')}
              >
                Send IPC Ping
              </a>
            </p>
          </div>
        </nav>

        {/* Content Area - will render the matched route */}
        <main className="flex-1 overflow-auto bg-gray-100">
          <Outlet />
        </main>
      </div>
    </div>
  )
}

// Dashboard specific component
function Dashboard() {
  return (
    <div className="p-6">
      <h2 className="text-2xl font-bold mb-6">Dashboard</h2>

      <div className="mb-8">
        <EnhancedAdapterStatus />
      </div>
      
      <div className="mb-8">
        <AdapterStatus />
      </div>

      <div className="bg-white rounded-lg shadow p-6 mb-6">
        <h3 className="text-lg font-semibold mb-4">Welcome to Zelan</h3>
        <p className="mb-3">
          This is your stream data aggregation service. Connect to various streaming
          platforms and merge their data into a unified API.
        </p>
        <p className="mb-3">
          Check out the <strong>Events</strong> tab to see the reactive event system in
          action.
        </p>
        <p>
          The <strong>WebSocket</strong> tab allows you to control the WebSocket server that
          external applications can connect to for real-time event data.
        </p>
      </div>
    </div>
  )
}

// WebSocket route component
function WebSocketRoute() {
  return (
    <div className="p-6">
      <h2 className="text-2xl font-bold mb-6">WebSocket Server</h2>
      <WebSocketDemo />
    </div>
  )
}

// Create routes
const rootRoute = new RootRoute({
  component: RootLayout,
})

// Define the routes
const indexRoute = new Route({
  getParentRoute: () => rootRoute,
  path: '/',
  component: Dashboard,
})

const eventsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: '/events',
  component: EventsDemo,
})

const enhancedEventsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: '/enhanced-events',
  component: EnhancedEventsDemo,
})

const settingsRoute = new Route({
  getParentRoute: () => rootRoute,
  path: '/settings',
  component: Settings,
})

const authRoute = new Route({
  getParentRoute: () => rootRoute,
  path: '/auth',
  component: AuthDemo,
})

const trpcRoute = new Route({
  getParentRoute: () => rootRoute,
  path: '/trpc',
  component: TrpcDemo,
})

const websocketRoute = new Route({
  getParentRoute: () => rootRoute,
  path: '/websocket',
  component: WebSocketRoute,
})

// Create the route tree
const routeTree = rootRoute.addChildren([
  indexRoute,
  eventsRoute,
  enhancedEventsRoute,
  settingsRoute,
  authRoute,
  trpcRoute,
  websocketRoute,
])

// Create the router
const hashHistory = createHashHistory()
const router = new Router({
  routeTree,
  history: hashHistory,
  // Using hash history for Electron compatibility
})

// Register the router for type safety
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router
  }
}

// Export the router provider component
export function AppRouter() {
  return <RouterProvider router={router} />
}