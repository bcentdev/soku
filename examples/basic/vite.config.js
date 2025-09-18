import { defineConfig } from 'vite'

export default defineConfig({
  build: {
    rollupOptions: {
      input: {
        main: './index.html'
      }
    },
    minify: true,
    outDir: './dist-vite',
    emptyOutDir: true
  }
})