import { createFileRoute } from '@tanstack/react-router'
import AuthDemo from '../components/AuthDemo'

export const Route = createFileRoute('/auth')({
  component: AuthDemo
})