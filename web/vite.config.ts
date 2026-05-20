/// <reference types="vitest" />
import { defineConfig } from 'vite'
import wasm from 'vite-plugin-wasm'
import topLevelAwait from 'vite-plugin-top-level-await'

// Wasm pkg/ は build-wasm.sh で生成される xmj_core (wasm-bindgen 出力)。
// `--target web` 形式のため、vite-plugin-wasm + top-level-await で読み込む。
export default defineConfig({
  base: '/',
  plugins: [wasm(), topLevelAwait()],
  assetsInclude: ['**/*.wasm'],
  server: {
    port: 3000,
    open: true,
  },
  build: {
    outDir: 'dist',
    sourcemap: true,
    target: 'esnext',
  },
  test: {
    environment: 'jsdom',
    // pkg は wasm-pack で生成される副産物。テスト時は wasm.ts 経由で vi.mock するため
    // テスト対象から外す。
    exclude: ['node_modules', 'dist', 'pkg', 'legacy'],
  },
})
