import { defineConfig } from 'vite'
import { TanStackRouterVite } from '@tanstack/router-plugin/vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  base: './',
  plugins: [
    TanStackRouterVite({
      routesDirectory: './src/routes',
      generatedRouteTree: './src/routeTree.gen.ts',
    }),
    react(),
  ],
  build: {
    outDir: '../web',
    emptyOutDir: true,
  },
  server: {
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
