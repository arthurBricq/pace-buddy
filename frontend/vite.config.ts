import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import fs from 'node:fs'

// Only load HTTPS certificates in development mode
function getHttpsConfig() {
  try {
    return {
      key: fs.readFileSync("./pacebuddy-key.pem"),
      cert: fs.readFileSync("./pacebuddy.pem"),
    };
  } catch {
    // Certificates not available (e.g., in CI/production build)
    return undefined;
  }
}
export default defineConfig({
  plugins: [react(), tailwindcss()],
  server: {
    proxy: {
      '/api': 'http://localhost:8080',
    },
    host: true,
    allowedHosts: ['pacebuddy'],
    hmr: { host: 'pacebuddy'},
    https: getHttpsConfig(),
  },
})
