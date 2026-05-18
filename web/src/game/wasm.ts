// Wasm 連携の集約ポイント (Issue #2 + #3)
//
// 役割:
//   - wasm-bindgen が生成する init() を一度だけ呼ぶ (initWasm)
//   - WasmGame コンストラクタ呼び出しを TS 側で隠蔽 (createWasmGame)
//   - drawTile / discardTile / executeCpuTurn 等のラッパを揃え、
//     UI 側 (App / GameScene) は WasmGameBridge だけ触れば済むようにする
//
// テスト戦略:
//   - pkg からの動的 import は vi.mock で差し替える (wasm.test.ts)
//   - ロジック自体はラッパなので、引数の素通しと initialized フラグを確認する

import type { Tile } from './types'
import { tileToCuiCode } from './types'

// pkg の型を再 export しないが、JSDoc で参照できるよう型 import だけしておく。
// (実体は dynamic import なので tree-shaking には影響しない)
type WasmModule = typeof import('../../pkg/xmj_core.js')
type WasmGameInstance = InstanceType<WasmModule['WasmGame']>

let wasmModule: WasmModule | null = null
let initialized = false

/**
 * Wasm モジュールをロードして init を呼ぶ。冪等。
 */
export const initWasm = async (): Promise<void> => {
  if (initialized) return
  // vite-plugin-wasm が wasm-bindgen 形式 (default export = init function) を扱う。
  // pkg は build-wasm.sh で生成される副産物のため tsconfig include / lint exclude 済。
  const mod = (await import('../../pkg/xmj_core.js')) as WasmModule
  if (typeof mod.default === 'function') {
    await mod.default()
  }
  wasmModule = mod
  initialized = true
}

/**
 * Wasm が初期化済みかを返す。
 */
export const isWasmReady = (): boolean => initialized

/**
 * テスト用リセット。本番コードからは呼ばない。
 */
export const __resetWasmForTest = (): void => {
  initialized = false
  wasmModule = null
}

/**
 * テストから WasmModule をモック注入するためのフック。
 * 通常のコードは initWasm() を使う。
 */
export const __setWasmModuleForTest = (mod: WasmModule | null): void => {
  wasmModule = mod
  initialized = mod !== null
}

// ============================================================================
// WasmGame ラッパ
// ============================================================================

/**
 * Rust 側 WasmGame の TS ラッパ。
 * UI 側はこれを通じて操作することで、Wasm の生 API 変更を吸収する。
 */
export class WasmGameBridge {
  private game: WasmGameInstance

  private constructor(game: WasmGameInstance) {
    this.game = game
  }

  /**
   * ハイブリッドモード (1 人間 + 3 CPU) でゲームを作る。
   * Wasm が未初期化なら例外。
   */
  static createHybrid(humanName: string, humanPosition: number): WasmGameBridge {
    if (!wasmModule) {
      throw new Error('Wasm が初期化されていません。initWasm() を先に呼んでください。')
    }
    const game = wasmModule.WasmGame.newHybrid(humanName, humanPosition)
    return new WasmGameBridge(game)
  }

  /**
   * 4 人全員人間のゲームを作る (debug 用)。
   */
  static createAllHuman(playerNames: string[]): WasmGameBridge {
    if (!wasmModule) {
      throw new Error('Wasm が初期化されていません。initWasm() を先に呼んでください。')
    }
    const game = new wasmModule.WasmGame(playerNames)
    return new WasmGameBridge(game)
  }

  // ---- ターン操作 ----
  drawTile(): boolean {
    return this.game.drawTile()
  }

  discardTile(tile: Tile): boolean {
    return this.game.discardTile(tileToCuiCode(tile))
  }

  executeCpuTurn(): string {
    return this.game.executeCpuTurn()
  }

  // ---- 状態取得 ----
  getGameStateJson(): string {
    return this.game.getGameState()
  }

  getCurrentHandString(): string {
    return this.game.getCurrentHand()
  }

  getCurrentPlayerId(): number {
    return this.game.getCurrentPlayerId()
  }

  isCurrentPlayerHuman(): boolean {
    return this.game.isCurrentPlayerHuman()
  }

  isCurrentPlayerCpu(): boolean {
    return this.game.isCurrentPlayerCpu()
  }

  isGameOver(): boolean {
    return this.game.isGameOver()
  }

  getWallCount(): number {
    return this.game.getWallCount()
  }

  getShanten(): number {
    return this.game.getShanten()
  }

  getPlayerScore(playerIdx: number): number {
    return this.game.getPlayerScore(playerIdx)
  }

  getPlayerName(playerIdx: number): string {
    return this.game.getPlayerName(playerIdx)
  }

  getDoraIndicators(): string {
    return this.game.getDoraIndicators()
  }

  getPlayerDiscards(playerIdx: number): string {
    return this.game.getPlayerDiscards(playerIdx)
  }

  // ---- リーチ ----
  canRiichi(): boolean {
    return this.game.canRiichi()
  }

  declareRiichi(): boolean {
    return this.game.declareRiichi()
  }

  isPlayerRiichi(playerIdx: number): boolean {
    return this.game.isPlayerRiichi(playerIdx)
  }

  /**
   * wasm-bindgen が生成した free() を呼んでメモリを解放する。
   * ゲーム終了時 / リスタート時に必ず呼ぶこと。
   */
  destroy(): void {
    this.game.free()
  }

  // TODO(Issue #5/#6): 鳴き系 API (canChi / canPon / canKan / doChi / doPon / doKan)
  // は Rust 側 (src/wasm.rs) では実装済みだが、GameScene 実装と入力ハンドラが
  // 揃ってからラップする。現状の Bridge はターン進行・状態取得・リーチに絞っている。
}
