import { Outlet, createRootRoute } from '@tanstack/react-router'
import { TanStackRouterDevtools } from '@tanstack/router-devtools'
import { Header } from '../components/header'
import { Footer } from '../components/footer'

export const Route = createRootRoute({
  component: () => (
    <div className="flex min-h-screen flex-col overflow-hidden rounded-lg dark:bg-slate-900 dark:text-white">
      <div className="pointer-events-none fixed h-dvh w-dvw rounded-lg inset-ring dark:inset-ring-white/20" />
      <Header />
      <main className="mt-10">
        <Outlet />
      </main>
      <Footer />
      {process.env.NODE_ENV === 'development' && <TanStackRouterDevtools />}
    </div>
  )
})
