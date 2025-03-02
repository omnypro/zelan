import { resolve } from 'path'
import { defineConfig, externalizeDepsPlugin, loadEnv } from 'electron-vite'
import react from '@vitejs/plugin-react'

export default defineConfig(({ mode }) => {
  const env = loadEnv(mode, process.cwd(), '')

  return {
    main: {
      define: {
        'process.env.TWITCH_CLIENT_ID': JSON.stringify(
          env.TWITCH_CLIENT_ID || process.env.TWITCH_CLIENT_ID
        )
      },
      plugins: [externalizeDepsPlugin()],
      resolve: {
        alias: {
          '@main': resolve('src/main'),
          '@shared': resolve('src/shared'),
          '@preload': resolve('src/preload')
        }
      }
    },
    preload: {
      plugins: [externalizeDepsPlugin()],
      resolve: {
        alias: {
          '@shared': resolve('src/shared'),
          '@preload': resolve('src/preload')
        }
      }
    },
    renderer: {
      resolve: {
        alias: {
          '@renderer': resolve('src/renderer/src'),
          '@shared': resolve('src/shared'),
          '@preload': resolve('src/preload')
        }
      },
      plugins: [react()]
    }
  }
})
