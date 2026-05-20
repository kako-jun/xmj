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

import type { PlayerIndex, RoundWinSummary, Tile } from './types'
import { parseRoundOutcome, tileToCuiCode } from './types'

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
// 内部ヘルパ
// ============================================================================

/**
 * `resolveWinTsumo` / `resolveWinRon` の戻り値 JSON
 * (`{"han":n,"fu":n,"totalPoints":n,"yaku":[...]}`) を `RoundWinSummary` に整形。
 * 空文字 / パースエラーは null。
 */
const parseSummaryAsWin = (
  json: string,
  winner: PlayerIndex,
  winType: 'tsumo' | 'ron',
  from: PlayerIndex | undefined
): RoundWinSummary | null => {
  if (!json) return null
  let parsed: Record<string, unknown>
  try {
    parsed = JSON.parse(json) as Record<string, unknown>
  } catch {
    return null
  }
  const yaku = Array.isArray(parsed.yaku) ? (parsed.yaku as string[]) : []
  return {
    winner,
    winType,
    ...(from !== undefined ? { from } : {}),
    han: (parsed.han as number) ?? 0,
    fu: (parsed.fu as number) ?? 0,
    totalPoints: (parsed.totalPoints as number) ?? 0,
    yaku,
  }
}

// 再 export: 呼び出し側が wasm.ts 1 つだけ import すれば parse まで済む
export { parseRoundOutcome }

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

  // ---- 和了宣言 (Issue #35) ----

  /**
   * 指定プレイヤーがツモ和了可能か。引数省略時は現在のプレイヤー。
   * UI 側はこれが true のときだけ「ツモ」ボタンを enable する。
   */
  canTsumo(playerIdx?: PlayerIndex): boolean {
    const idx = playerIdx ?? (this.game.getCurrentPlayerId() as PlayerIndex)
    return this.game.canTsumo(idx)
  }

  /**
   * 指定プレイヤーが直前打牌に対してロン可能か。
   * 引数省略時は人間プレイヤーの判定ではなく「呼び出し側で決めた idx」を渡す前提なので、
   * 安全のため省略時は 0 を返す呼び出しは避け、必ず idx を指定する。
   */
  canRon(playerIdx: PlayerIndex): boolean {
    return this.game.canRon(playerIdx)
  }

  /**
   * 直前に打牌したプレイヤーの座席 index。`last_discard` が無ければ undefined。
   */
  getLastDiscarder(): PlayerIndex | undefined {
    const v = this.game.getLastDiscarder()
    return v === undefined || v === null ? undefined : (v as PlayerIndex)
  }

  // ---- 局結着 / 次局 (Issue #27) ----

  /**
   * 流局を確定する。聴牌者の座席 index 配列を渡す。
   * Rust 側 wasm-bindgen は `Vec<usize>` を `Uint32Array` として受け取るため変換する。
   */
  resolveDraw(tenpaiPlayerIndices: number[]): void {
    const arr = new Uint32Array(tenpaiPlayerIndices)
    // wasm-bindgen が生成した `pkg/xmj_core.d.ts` の `WasmGame.resolveDraw`
    // は `Uint32Array` を直接受ける宣言になっているのでそのまま渡す。
    this.game.resolveDraw(arr)
  }

  /**
   * 流局時のテンパイ者の座席 index 配列を WASM 側で算出して返す。
   * `Player::is_tenpai()` を 4 人ぶん回した結果をそのまま貰う。
   * 戻り値は `Uint32Array` なので素の `PlayerIndex[]` に変換して返す。
   */
  computeTenpaiPlayers(): PlayerIndex[] {
    const arr = this.game.computeTenpaiPlayers()
    return Array.from(arr).map(n => n as PlayerIndex)
  }

  /**
   * ツモ和了を確定する。和了形でなければ null を返す。
   * 戻り値は ScoringResult のサマリ。`getLastOutcomeJson` 経由でも取得可能。
   */
  resolveWinTsumo(winnerIdx: PlayerIndex): RoundWinSummary | null {
    const json = this.game.resolveWinTsumo(winnerIdx)
    return parseSummaryAsWin(json, winnerIdx, 'tsumo', undefined)
  }

  /**
   * ロン和了を確定する。打牌者は fromIdx で指定。和了形でなければ null。
   */
  resolveWinRon(winnerIdx: PlayerIndex, fromIdx: PlayerIndex): RoundWinSummary | null {
    const json = this.game.resolveWinRon(winnerIdx, fromIdx)
    return parseSummaryAsWin(json, winnerIdx, 'ron', fromIdx)
  }

  /** 次の局へ。true=続行 / false=対局終了。 */
  nextRound(): boolean {
    return this.game.nextRound()
  }

  getRound(): number {
    return this.game.getRound()
  }

  getHonba(): number {
    return this.game.getHonba()
  }

  getDealer(): PlayerIndex {
    return this.game.getDealer() as PlayerIndex
  }

  getRiichiSticks(): number {
    return this.game.getRiichiSticks()
  }

  /**
   * 直前局の結果 JSON を生で返す。空文字なら未確定。
   * UI 側は `parseRoundOutcome` で `RoundOutcome` に変換する。
   */
  getLastOutcomeJson(): string {
    return this.game.getLastOutcomeJson()
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
