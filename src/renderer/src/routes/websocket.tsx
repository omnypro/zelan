import { createFileRoute } from '@tanstack/react-router'
import { WebSocketDemo } from '../components/demos/WebSocketDemo'

export const Route = createFileRoute('/websocket')({
  component: WebSocketRouteComponent
})

function WebSocketRouteComponent() {
  return (
    <div className="p-6">
      <h2 className="text-2xl font-bold mb-6">WebSocket Server</h2>
      <WebSocketDemo />
    </div>
  )
}