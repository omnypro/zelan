import { createFileRoute } from '@tanstack/react-router'
import { TrpcDemo } from '../components/TrpcDemo'

export const Route = createFileRoute('/trpc')({
  component: TrpcDemo
})