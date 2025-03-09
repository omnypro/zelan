import { defineConfig, externalizeDepsPlugin } from 'electron-vite'
import { resolve } from 'path'
import react from '@vitejs/plugin-react'

// @ts-ignore - Can't switch to "bundler" or "node16"
import { TanStackRouterVite } from '@tanstack/router-plugin/vite'

export default defineConfig({
  main: {
    build: { outDir: 'dist/main' },
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
    build: { outDir: 'dist/preload' },
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
    build: { outDir: 'dist/renderer' },
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
