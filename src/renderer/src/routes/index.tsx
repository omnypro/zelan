import { createFileRoute } from '@tanstack/react-router'
import AdapterStatus from '../components/AdapterStatus'
import EnhancedAdapterStatus from '../components/EnhancedAdapterStatus'

export const Route = createFileRoute('/')({
  component: Dashboard
})

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
          This is your stream data aggregation service. Connect to various streaming platforms and merge their data into
          a unified API.
        </p>
        <p className="mb-3">
          Check out the <strong>Events</strong> tab to see the reactive event system in action.
        </p>
        <p>
          The <strong>WebSocket</strong> tab allows you to control the WebSocket server that external applications can
          connect to for real-time event data.
        </p>
      </div>
    </div>
  )
}
