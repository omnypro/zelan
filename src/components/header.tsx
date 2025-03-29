import { Link } from '@tanstack/react-router'
import { useEffect, useState } from 'react'
import { MacOSWindowControls } from './window-controls'

export function Header() {
  const [isConnected, setIsConnected] = useState(false)

  useEffect(() => {
    // Here you would check connection status with Tauri backend
    // For now, we'll just simulate being connected
    setIsConnected(true)
  }, [])

  return (
    <header className="fixed w-full px-4" data-tauri-drag-region>
      <div className="flex items-center justify-between">
        <MacOSWindowControls />
        <div className="flex items-center gap-2">
          <div className="flex items-center gap-1">
            <div className={`h-2 w-2 rounded-full ${isConnected ? 'bg-green-500' : 'bg-red-500'}`} />
            <span className="text-xs">{isConnected ? 'Connected' : 'Disconnected'}</span>
          </div>
        </div>
        <nav>
          <ul className="flex gap-4">
            <li>
              <Link to="/" activeProps={{ className: 'font-bold' }} className="hover:underline">
                Dashboard
              </Link>
            </li>
            <li>
              <Link to="/settings" activeProps={{ className: 'font-bold' }} className="hover:underline">
                Settings
              </Link>
            </li>
            <li>
              <Link to="/developer" activeProps={{ className: 'font-bold' }} className="hover:underline">
                Developer
              </Link>
            </li>
          </ul>
        </nav>
      </div>
    </header>
  )
}
