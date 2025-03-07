import { useState } from 'react'
import Versions from './components/Versions'
import EventsDemo from './components/EventsDemo'
import AdapterStatus from './components/AdapterStatus'
import Settings from './components/Settings'
import electronLogo from './assets/electron.svg'
import './assets/main.css'

function App(): JSX.Element {
  const [activeTab, setActiveTab] = useState<'dashboard' | 'events' | 'settings'>('dashboard')

  const ipcHandle = (): void => window.electron.ipcRenderer.send('ping')

  return (
    <div className="min-h-screen bg-gray-100 flex flex-col">
      {/* Header */}
      <header className="bg-gray-800 text-white p-4 flex items-center shadow-md">
        <img alt="logo" className="h-8 w-8 mr-3" src={electronLogo} />
        <h1 className="text-xl font-semibold">Zelan</h1>
        <p className="ml-4 text-gray-400 text-sm">Stream Data Aggregation Service</p>
      </header>

      {/* Main Content */}
      <div className="flex flex-1 overflow-hidden">
        {/* Sidebar */}
        <nav className="w-48 bg-gray-700 text-white">
          <ul className="py-4">
            <li>
              <button
                className={`w-full text-left px-4 py-2 ${
                  activeTab === 'dashboard' 
                    ? 'bg-blue-600 text-white' 
                    : 'text-gray-300 hover:bg-gray-600'
                }`}
                onClick={() => setActiveTab('dashboard')}
              >
                Dashboard
              </button>
            </li>
            <li>
              <button
                className={`w-full text-left px-4 py-2 ${
                  activeTab === 'events' 
                    ? 'bg-blue-600 text-white' 
                    : 'text-gray-300 hover:bg-gray-600'
                }`}
                onClick={() => setActiveTab('events')}
              >
                Events
              </button>
            </li>
            <li>
              <button
                className={`w-full text-left px-4 py-2 ${
                  activeTab === 'settings' 
                    ? 'bg-blue-600 text-white' 
                    : 'text-gray-300 hover:bg-gray-600'
                }`}
                onClick={() => setActiveTab('settings')}
              >
                Settings
              </button>
            </li>
          </ul>
          
          <div className="mt-auto p-4 text-xs text-gray-400">
            <p>
              <a 
                className="text-blue-400 hover:underline cursor-pointer" 
                onClick={ipcHandle}
              >
                Send IPC Ping
              </a>
            </p>
          </div>
        </nav>

        {/* Content Area */}
        <main className="flex-1 overflow-auto bg-gray-100">
          {activeTab === 'dashboard' && (
            <div className="p-6">
              <h2 className="text-2xl font-bold mb-6">Dashboard</h2>
              
              <div className="mb-8">
                <AdapterStatus />
              </div>
              
              <div className="bg-white rounded-lg shadow p-6 mb-6">
                <h3 className="text-lg font-semibold mb-4">Welcome to Zelan</h3>
                <p className="mb-3">
                  This is your stream data aggregation service. Connect to various streaming platforms
                  and merge their data into a unified API.
                </p>
                <p>
                  Check out the <strong>Events</strong> tab to see the reactive event system in action.
                </p>
              </div>
              
              <Versions />
            </div>
          )}
          
          {activeTab === 'events' && <EventsDemo />}
          
          {activeTab === 'settings' && <Settings />}
        </main>
      </div>
    </div>
  )
}

export default App