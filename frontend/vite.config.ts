import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import tailwindcss from '@tailwindcss/vite'
import fs from 'node:fs'

// Only load HTTPS certificates in development mode
function getHttpsConfig() {
  try {
    return {
      key: fs.readFileSync("./running.tool-key.pem"),
      cert: fs.readFileSync("./running.tool.pem"),
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
    allowedHosts: ['running.tool'],
    hmr: { host: 'running.tool'},
    https: getHttpsConfig(),
  },
})
