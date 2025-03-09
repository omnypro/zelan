import { Link, Outlet, createRootRoute, useMatchRoute } from '@tanstack/react-router'
import electronLogo from '../assets/electron.svg'

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

// For file-based routing we need to use createRootRoute for the root layout
export const Route = createRootRoute({
  component: Layout
})

function Layout() {
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