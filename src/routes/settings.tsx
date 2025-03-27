import { createFileRoute } from '@tanstack/react-router'
import { Settings } from '../components/settings'

export const Route = createFileRoute('/settings')({
  component: Settings,
})