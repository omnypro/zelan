import { useState } from 'react'
import { ScrollArea } from './ui/scroll-area'
import { ConfigurationForm } from './configuration-form'

export function Settings() {
  const [activeTab, setActiveTab] = useState('general')
  
  const adapters = [
    { id: 'twitch', name: 'Twitch', status: 'connected' },
    { id: 'obs', name: 'OBS', status: 'disconnected' },
  ]

  return (
    <div className="space-y-6">
      <h2 className="text-2xl font-bold">Settings</h2>
      
      <div className="grid grid-cols-1 md:grid-cols-4 gap-6">
        <div className="md:col-span-1 bg-card rounded-lg p-4 shadow">
          <div className="h-[70vh] overflow-auto">
            <ul className="space-y-2">
              <li>
                <button
                  className={`w-full text-left p-2 rounded ${activeTab === 'general' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'}`}
                  onClick={() => setActiveTab('general')}
                >
                  General
                </button>
              </li>
              <li>
                <div className="font-medium text-sm text-muted-foreground uppercase tracking-wider py-2">
                  Adapters
                </div>
                <ul className="space-y-1">
                  {adapters.map((adapter) => (
                    <li key={adapter.id}>
                      <button
                        className={`w-full text-left p-2 rounded flex items-center gap-2 ${activeTab === adapter.id ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'}`}
                        onClick={() => setActiveTab(adapter.id)}
                      >
                        <div
                          className={`w-2 h-2 rounded-full ${adapter.status === 'connected' ? 'bg-green-500' : 'bg-gray-400'}`}
                        />
                        {adapter.name}
                      </button>
                    </li>
                  ))}
                </ul>
              </li>
              <li>
                <button
                  className={`w-full text-left p-2 rounded ${activeTab === 'websocket' ? 'bg-primary text-primary-foreground' : 'hover:bg-muted'}`}
                  onClick={() => setActiveTab('websocket')}
                >
                  WebSocket Server
                </button>
              </li>
            </ul>
          </div>
        </div>
        
        <div className="md:col-span-3 bg-card rounded-lg p-6 shadow">
          <div className="h-[70vh] overflow-auto">
            {activeTab === 'general' && (
              <ConfigurationForm
                title="General Settings"
                description="Configure global application settings"
                fields={[
                  { name: 'appName', label: 'Application Name', type: 'text', defaultValue: 'Zelan' },
                  { name: 'startOnBoot', label: 'Start on System Boot', type: 'checkbox', defaultValue: false },
                ]}
              />
            )}
            
            {activeTab === 'twitch' && (
              <ConfigurationForm
                title="Twitch Settings"
                description="Configure your Twitch integration"
                fields={[
                  { name: 'clientId', label: 'Client ID', type: 'text', defaultValue: '' },
                  { name: 'clientSecret', label: 'Client Secret', type: 'password', defaultValue: '' },
                  { name: 'redirectUri', label: 'Redirect URI', type: 'text', defaultValue: 'http://localhost:8000/callback' },
                ]}
              />
            )}
            
            {activeTab === 'obs' && (
              <ConfigurationForm
                title="OBS Settings"
                description="Configure your OBS Studio integration"
                fields={[
                  { name: 'host', label: 'Host', type: 'text', defaultValue: 'localhost' },
                  { name: 'port', label: 'Port', type: 'number', defaultValue: 4455 },
                  { name: 'password', label: 'Password', type: 'password', defaultValue: '' },
                ]}
              />
            )}
            
            {activeTab === 'websocket' && (
              <ConfigurationForm
                title="WebSocket Server Settings"
                description="Configure the WebSocket server for external connections"
                fields={[
                  { name: 'port', label: 'Port', type: 'number', defaultValue: 9000 },
                  { name: 'maxConnections', label: 'Maximum Connections', type: 'number', defaultValue: 100 },
                  { name: 'timeout', label: 'Connection Timeout (seconds)', type: 'number', defaultValue: 300 },
                ]}
              />
            )}
          </div>
        </div>
      </div>
    </div>
  )
}