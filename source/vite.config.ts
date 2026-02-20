import { defineConfig } from 'vite'
import { TanStackRouterVite } from '@tanstack/router-plugin/vite'
import react from '@vitejs/plugin-react'
import path from 'path'

export default defineConfig({
  base: '/yeti-applications/',
  plugins: [
    TanStackRouterVite({
      routesDirectory: './src/routes',
      generatedRouteTree: './src/routeTree.gen.ts',
    }),
    react(),
  ],
  resolve: {
    alias: { '@yeti': path.resolve(__dirname, '../../../shared') },
  },
  build: {
    outDir: '../web',
    emptyOutDir: true,
  },
  server: {
    fs: { allow: ['..', '../../../shared'] },
    port: 5177,
    proxy: {
      '/yeti-applications': {
        target: 'https://localhost:9996',
        changeOrigin: true,
        secure: false,
      },
      '/yeti-auth': {
        target: 'https://localhost:9996',
        changeOrigin: true,
        secure: false,
      },
    },
  },
})
