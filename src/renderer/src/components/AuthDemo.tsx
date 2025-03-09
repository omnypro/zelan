import { TwitchAuthCard } from './Auth'

/**
 * Demo component for authentication system
 */
export default function AuthDemo() {
  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6">Authentication Demo</h1>

      <div className="max-w-4xl mx-auto">
        <TwitchAuthCard />
      </div>

      <div className="mt-8 p-4 bg-gray-100 rounded-lg max-w-4xl mx-auto">
        <h2 className="text-lg font-medium mb-2">How Authentication Works</h2>
        <p className="mb-2">
          This demo showcases the authentication system using Twitch as an example. The system uses
          the Device Code flow, which is recommended for desktop applications.
        </p>

        <h3 className="font-medium mt-4 mb-1">Authentication Flow:</h3>
        <ol className="list-decimal ml-6 space-y-1">
          <li>User enters their Twitch Client ID and clicks "Connect"</li>
          <li>Application gets a device code from Twitch</li>
          <li>User authorizes the application on Twitch's website</li>
          <li>Application receives an access token and refresh token</li>
          <li>Tokens are securely stored with encryption</li>
          <li>Access token is automatically refreshed before it expires</li>
        </ol>

        <h3 className="font-medium mt-4 mb-1">Security Features:</h3>
        <ul className="list-disc ml-6 space-y-1">
          <li>Tokens are encrypted before storage</li>
          <li>Only minimal scopes are requested</li>
          <li>Automatic token refresh and validation</li>
          <li>Clear error handling and status updates</li>
        </ul>
      </div>
    </div>
  )
}
