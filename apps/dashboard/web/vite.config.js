import { defineConfig } from 'vite'

// Use BASE_PATH env (e.g., "/<repo>/") for GitHub Pages; default to "/"
export default defineConfig({
  base: process.env.BASE_PATH || '/',
})
