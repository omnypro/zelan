import { defineConfig } from 'vite'
import path from 'node:path'
import electron from 'vite-plugin-electron/simple'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [
    react(),
    electron({
      main: {
        entry: 'electron/main.ts',
        vite: {
          resolve: {
            alias: {
              '@': path.resolve(__dirname, './src'),
              '~': path.resolve(__dirname, './electron'),
              '@shared': path.resolve(__dirname, './shared')
            }
          }
        }
      },
      preload: {
        input: path.join(__dirname, 'electron/preload.ts'),
        vite: {
          resolve: {
            alias: {
              '@': path.resolve(__dirname, './src'),
              '~': path.resolve(__dirname, './electron'),
              '@shared': path.resolve(__dirname, './shared')
            }
          }
        }
      },
      renderer: {}
    }),
    tailwindcss()
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
      '~': path.resolve(__dirname, './electron'),
      '@shared': path.resolve(__dirname, './shared')
    }
  }
})