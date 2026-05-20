// PixiJS v8 + Wasm エントリ。Wasm モジュールは vite-plugin-wasm 経由で
// 動的 import する。最初の起動時に init() を一度だけ呼ぶ。

import { Application } from 'pixi.js'
import { App } from './game/App'
import { STAGE_WIDTH, STAGE_HEIGHT, TABLE_BG_COLOR } from './game/constants'
import { initWasm, WasmGameBridge } from './game/wasm'

const setLoadingProgress = (ratio: number): void => {
  const bar = document.querySelector<HTMLDivElement>('#loading-bar > div')
  if (bar) bar.style.width = `${Math.floor(ratio * 100)}%`
}

const removeLoading = (): void => {
  const el = document.getElementById('loading')
  if (el) el.remove()
}

const main = async (): Promise<void> => {
  setLoadingProgress(0.1)
  let canStartGame = true

  // Wasm を最初に読む。失敗してもゲームは描画したいので catch して続行。
  try {
    await initWasm()
  } catch (err) {
    canStartGame = false
    console.warn('[xmj] Wasm 初期化に失敗しました。UI のみで起動します:', err)
  }
  setLoadingProgress(0.5)

  const pixiApp = new Application()
  await pixiApp.init({
    width: STAGE_WIDTH,
    height: STAGE_HEIGHT,
    background: TABLE_BG_COLOR,
    antialias: true,
    autoDensity: true,
    resolution: Math.min(window.devicePixelRatio || 1, 2),
  })

  const host = document.getElementById('game') ?? document.body
  host.appendChild(pixiApp.canvas)

  setLoadingProgress(0.8)

  const app = new App(pixiApp, {
    cpuTurnDelayMs: 280,
    createBridge: canStartGame
      ? humanSeat => WasmGameBridge.createHybrid('あなた', humanSeat)
      : null,
  })
  if (import.meta.env.DEV) {
    window.__xmjApp = app
  }
  app.showTitleScene(
    canStartGame ? null : 'Wasm 初期化に失敗したため、対局を開始できません。'
  )

  setLoadingProgress(1)
  removeLoading()
}

main().catch(err => {
  console.error(err)
})
