import { defineConfig } from 'vite'

// Central Vite config so we don't need to pass --root in scripts
export default defineConfig({
  // point to the sibling web/ folder relative to this config (in src/)
  root: '../web'
})
