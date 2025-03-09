import { resolve } from 'path'
import { defineConfig, externalizeDepsPlugin } from 'electron-vite'
import react from '@vitejs/plugin-react'
import { TanStackRouterVite } from '@tanstack/router-plugin/vite'

export default defineConfig({
  main: {
    plugins: [externalizeDepsPlugin()],
    resolve: {
      alias: {
        '@m': resolve('src/main'),
        '@s': resolve('src/shared'),
        '@p': resolve('src/preload')
      }
    }
  },
  preload: {
    plugins: [externalizeDepsPlugin()],
    resolve: {
      alias: {
        '@m': resolve('src/main'),
        '@s': resolve('src/shared'),
        '@p': resolve('src/preload')
      }
    }
  },
  renderer: {
    resolve: {
      alias: {
        '@r': resolve('src/renderer/src'),
        '@s': resolve('src/shared'),
        '@p': resolve('src/preload')
      }
    },
    plugins: [
      react(),
      TanStackRouterVite({
        routesDirectory: './src/renderer/src/routes',
        generatedRouteTree: './src/renderer/src/routeTree.gen.ts'
      })
    ]
  }
})
