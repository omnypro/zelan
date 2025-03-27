import { createFileRoute } from '@tanstack/react-router'
import { DeveloperDebug } from '../components/developer-debug'

export const Route = createFileRoute('/developer')({
  component: DeveloperDebug,
})