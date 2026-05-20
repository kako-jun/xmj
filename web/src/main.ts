// PixiJS v8 + Wasm エントリ。Wasm モジュールは vite-plugin-wasm 経由で
// 動的 import する。最初の起動時に init() を一度だけ呼ぶ。

import { Application } from 'pixi.js'
import { App } from './game/App'
import { STAGE_WIDTH, STAGE_HEIGHT, TABLE_BG_COLOR } from './game/constants'
import { initWasm, WasmGameBridge } from './game/wasm'
import type { PlayerIndex } from './game/types'

const setLoadingProgress = (ratio: number): void => {
  const bar = document.querySelector<HTMLDivElement>('#loading-bar > div')
  if (bar) bar.style.width = `${Math.floor(ratio * 100)}%`
}

const removeLoading = (): void => {
  const el = document.getElementById('loading')
  if (el) el.remove()
}

/**
 * URL クエリで指定できるデバッグ開始位置。
 *   ?scene=title|mode|dice|table       (省略時は title)
 *   ?seed=42                            (将来 wasm 側で乱数 seed 注入用、現状 hint のみ)
 *   ?seat=0|1|2|3                       (場決めをスキップして人間座席を直指定)
 *   ?cpuDelay=120                       (CPU 打牌の演出 ms 上書き)
 *
 * 例: `http://localhost:3000/?scene=table&seat=0` で対局シーン直起動。
 */
type DebugScene = 'title' | 'mode' | 'dice' | 'table'

interface DebugStart {
  scene: DebugScene
  seat: PlayerIndex | null
  cpuDelayMs: number | null
}

const parseDebugStart = (): DebugStart => {
  const fallback: DebugStart = { scene: 'title', seat: null, cpuDelayMs: null }
  if (typeof window === 'undefined') return fallback
  const url = new URL(window.location.href)
  const sceneParam = url.searchParams.get('scene')
  const allowed: DebugScene[] = ['title', 'mode', 'dice', 'table']
  const scene = allowed.includes(sceneParam as DebugScene)
    ? (sceneParam as DebugScene)
    : 'title'

  const seatParam = url.searchParams.get('seat')
  let seat: PlayerIndex | null = null
  if (seatParam !== null) {
    const n = Number(seatParam)
    if (n === 0 || n === 1 || n === 2 || n === 3) seat = n as PlayerIndex
  }

  const cpuDelayParam = url.searchParams.get('cpuDelay')
  const cpuDelayMs = cpuDelayParam !== null ? Number(cpuDelayParam) : null

  return { scene, seat, cpuDelayMs: Number.isFinite(cpuDelayMs) ? cpuDelayMs : null }
}

const main = async (): Promise<void> => {
  setLoadingProgress(0.1)
  let canStartGame = true

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

  const debugStart = parseDebugStart()
  const app = new App(pixiApp, {
    cpuTurnDelayMs: debugStart.cpuDelayMs ?? 280,
    createBridge: canStartGame
      ? humanSeat => WasmGameBridge.createHybrid('あなた', humanSeat)
      : null,
  })
  if (import.meta.env.DEV) {
    window.__xmjApp = app
  }

  // URL クエリで scene を指定された場合は直接遷移する。
  // 'table' は人間座席が決まっていないと bridge を作れないので seat 指定が無ければ 0 に倒す。
  if (!canStartGame) {
    app.showTitleScene('Wasm 初期化に失敗したため、対局を開始できません。')
  } else {
    switch (debugStart.scene) {
      case 'mode':
        app.showModeSelectScene()
        break
      case 'dice':
        app.showDiceRollScene()
        break
      case 'table': {
        const seat = debugStart.seat ?? 0
        app.selectedHumanSeat = seat
        app.startNewGame()
        break
      }
      case 'title':
      default:
        app.showTitleScene(null)
        break
    }
  }

  setLoadingProgress(1)
  removeLoading()
}

main().catch(err => {
  console.error(err)
})
