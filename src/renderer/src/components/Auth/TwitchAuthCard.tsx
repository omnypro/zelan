import { useState } from 'react'
import { AuthProvider, AuthState } from '@s/auth/interfaces'
import { useAuth } from '@r/hooks/useAuth'

/**
 * Card component for Twitch authentication
 */
export default function TwitchAuthCard() {
  const { status, deviceCode, isLoading, error, login, logout } = useAuth(AuthProvider.TWITCH)
  const [clientId, setClientId] = useState('')

  // Default Twitch scopes for basic integration
  const scopes = [
    'user:read:email',
    'channel:read:subscriptions',
    'channel:read:redemptions',
    'channel:read:polls',
    'channel:read:predictions',
    'chat:read'
  ]

  // Handle login click
  const handleLogin = async () => {
    if (!clientId) {
      alert('Please enter your Twitch Client ID')
      return
    }

    await login({
      clientId,
      scopes
    })
  }

  // Handle logout click
  const handleLogout = async () => {
    await logout()
  }

  return (
    <div className="bg-white rounded-lg shadow-md p-6 max-w-md mx-auto">
      <div className="flex items-center justify-between mb-6">
        <h2 className="text-xl font-semibold">Twitch Integration</h2>
        <div className="flex items-center">
          <span
            className={`inline-block w-3 h-3 rounded-full mr-2 ${
              status.isAuthenticated ? 'bg-green-500' : 'bg-red-500'
            }`}
          />
          <span className="text-sm">
            {status.isAuthenticated ? 'Connected' : 'Disconnected'}
          </span>
        </div>
      </div>

      {status.isAuthenticated ? (
        // Authenticated state
        <div className="space-y-4">
          <div className="border rounded-md p-3 bg-gray-50">
            <div className="grid grid-cols-2 gap-2 text-sm">
              <div className="text-gray-500">Username:</div>
              <div className="font-medium">{status.username || 'Unknown'}</div>
              
              <div className="text-gray-500">User ID:</div>
              <div className="font-medium">{status.userId || 'Unknown'}</div>
              
              <div className="text-gray-500">Status:</div>
              <div className="font-medium">{status.state}</div>
              
              <div className="text-gray-500">Expires:</div>
              <div className="font-medium">
                {status.expiresAt 
                  ? new Date(status.expiresAt).toLocaleString() 
                  : 'Unknown'}
              </div>
            </div>
          </div>
          
          <div>
            <button
              onClick={handleLogout}
              disabled={isLoading}
              className="w-full py-2 px-4 bg-red-500 text-white rounded-md hover:bg-red-600 
                focus:outline-none focus:ring-2 focus:ring-red-500 focus:ring-opacity-50
                disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {isLoading ? 'Disconnecting...' : 'Disconnect from Twitch'}
            </button>
          </div>
        </div>
      ) : (
        // Unauthenticated state
        <>
          {deviceCode ? (
            // Device code flow step
            <div className="space-y-4">
              <div className="border rounded-md p-4 bg-gray-50">
                <h3 className="text-md font-medium mb-2">Enter this code on Twitch:</h3>
                <div className="bg-gray-200 p-2 text-center rounded font-mono text-lg tracking-wide">
                  {deviceCode.user_code}
                </div>
                <p className="text-sm text-gray-600 mt-2">
                  Go to <a
                    href={deviceCode.verification_uri_complete || deviceCode.verification_uri}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-500 underline"
                  >
                    {deviceCode.verification_uri}
                  </a> to authorize this application.
                </p>
                <p className="text-xs text-gray-500 mt-2">
                  Code expires in {Math.round(deviceCode.expires_in / 60)} minutes
                </p>
              </div>
              
              {isLoading && (
                <div className="text-center text-sm text-gray-600">
                  Waiting for authorization...
                </div>
              )}
            </div>
          ) : (
            // Login form
            <div className="space-y-4">
              <div>
                <label htmlFor="client-id" className="block text-sm font-medium text-gray-700 mb-1">
                  Twitch Client ID
                </label>
                <input
                  id="client-id"
                  type="text"
                  value={clientId}
                  onChange={(e) => setClientId(e.target.value)}
                  placeholder="Enter your Twitch Client ID"
                  className="w-full p-2 border rounded-md focus:ring-blue-500 focus:border-blue-500"
                />
                <p className="text-xs text-gray-500 mt-1">
                  You can get your Client ID from the{' '}
                  <a
                    href="https://dev.twitch.tv/console/apps"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-500 underline"
                  >
                    Twitch Developer Console
                  </a>
                </p>
              </div>
              
              <div>
                <button
                  onClick={handleLogin}
                  disabled={isLoading || !clientId}
                  className="w-full py-2 px-4 bg-purple-600 text-white rounded-md hover:bg-purple-700
                    focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-opacity-50
                    disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {isLoading ? 'Connecting...' : 'Connect to Twitch'}
                </button>
              </div>
            </div>
          )}
        </>
      )}

      {/* Error message */}
      {error && (
        <div className="mt-4 p-3 bg-red-100 border border-red-300 text-red-800 rounded-md text-sm">
          {error}
        </div>
      )}
      
      <div className="mt-4 text-xs text-gray-500">
        <p>
          This integration uses the Device Code flow to authenticate with Twitch.
          Your credentials are securely stored and only the minimum required scopes
          are requested.
        </p>
      </div>
    </div>
  )
}