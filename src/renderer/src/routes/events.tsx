import { createFileRoute } from '@tanstack/react-router'
import EventsDemo from '../components/EventsDemo'

export const Route = createFileRoute('/events')({
  component: EventsDemo
})