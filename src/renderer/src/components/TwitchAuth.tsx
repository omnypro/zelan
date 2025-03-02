import { useEffect, useState } from 'react'
import { useSimpleObservable } from '../hooks/useSimpleObservable'

interface AuthState {
  isAuthorized: boolean
  isAuthenticating: boolean
  authUrl?: string
  userCode?: string
  error?: string
}

export default function TwitchAuth() {
  const [hasCopied, setHasCopied] = useState(false)

  // Access the auth state from the preload API
  const authState = useSimpleObservable<AuthState>(window.electron.ipcRenderer.auth.state, {
    isAuthorized: false,
    isAuthenticating: false
  })

  // Reset copy state whenever the code changes
  useEffect(() => {
    setHasCopied(false)
  }, [authState.userCode])

  // Handle clipboard copy
  const copyCode = () => {
    if (authState.userCode) {
      navigator.clipboard.writeText(authState.userCode)
      setHasCopied(true)
    }
  }

  // Start authentication
  const handleStartAuth = () => {
    window.electron.ipcRenderer.auth.startAuth()
  }

  // Log out
  const handleLogout = () => {
    window.electron.ipcRenderer.auth.logout()
  }

  // Render based on auth state
  if (authState.isAuthorized) {
    return (
      <div className="auth-container">
        <div className="auth-status authorized">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"></path>
            <polyline points="22 4 12 14.01 9 11.01"></polyline>
          </svg>
          <span>Connected to Twitch</span>
        </div>
        <button onClick={handleLogout} className="auth-button logout">
          Disconnect
        </button>
      </div>
    )
  }

  if (authState.isAuthenticating && authState.authUrl && authState.userCode) {
    return (
      <div className="auth-container">
        <div className="auth-status authenticating">
          <h3>Connect to Twitch</h3>
          <ol className="auth-steps">
            <li>
              <a
                href={authState.authUrl}
                target="_blank"
                rel="noopener noreferrer"
                className="auth-link"
              >
                Open Twitch Activation
              </a>
            </li>
            <li>
              <div className="auth-code-container">
                <span className="auth-code">{authState.userCode}</span>
                <button
                  onClick={copyCode}
                  className={`auth-copy-button ${hasCopied ? 'copied' : ''}`}
                >
                  {hasCopied ? 'Copied!' : 'Copy'}
                </button>
              </div>
            </li>
            <li>Waiting for authorization...</li>
          </ol>
        </div>
      </div>
    )
  }

  if (authState.error) {
    return (
      <div className="auth-container">
        <div className="auth-status error">
          <svg
            xmlns="http://www.w3.org/2000/svg"
            width="24"
            height="24"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            strokeWidth="2"
            strokeLinecap="round"
            strokeLinejoin="round"
          >
            <circle cx="12" cy="12" r="10"></circle>
            <line x1="12" y1="8" x2="12" y2="12"></line>
            <line x1="12" y1="16" x2="12.01" y2="16"></line>
          </svg>
          <span>Error: {authState.error}</span>
        </div>
        <button onClick={handleStartAuth} className="auth-button">
          Try Again
        </button>
      </div>
    )
  }

  return (
    <div className="auth-container">
      <div className="auth-status unauthorized">
        <span>Not connected to Twitch</span>
      </div>
      <button onClick={handleStartAuth} className="auth-button">
        Connect
      </button>
    </div>
  )
}
