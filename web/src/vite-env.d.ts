/// <reference types="vite/client" />

declare global {
  interface Window {
    __xmjApp?: import('./game/App').App
  }
}

export {}
