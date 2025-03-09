import { createFileRoute } from '@tanstack/react-router'
import EnhancedEventsDemo from '../components/EnhancedEventsDemo'

export const Route = createFileRoute('/enhanced-events')({
  component: EnhancedEventsDemo
})
