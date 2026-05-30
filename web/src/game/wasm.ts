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

import type { MeldGroup, MeldKind, PlayerIndex, RoundWinSummary, Tile } from './types'
import { parseRoundOutcome, tileFromCuiCode, tileToCuiCode } from './types'

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

  /**
   * プレイヤーの副露 (鳴き面子) を取得する (#83 副露表示)。
   *
   * Rust 側 `getPlayerMelds` は JSON 配列を返す:
   * ```json
   * [{ "kind": "chi"|"pon"|"ankan"|"minkan"|"kakan",
   *    "tiles": ["1m","2m","3m"],
   *    "fromOffset": 0|1|2|3|null,
   *    "claimedIndex": 0|1|2|null }, ...]
   * ```
   * これを `MeldGroup[]` に parse する。空配列・パースエラーは `[]`。
   */
  getPlayerMelds(playerIdx: PlayerIndex): MeldGroup[] {
    const raw = this.game.getPlayerMelds(playerIdx)
    if (!raw) return []
    let parsed: unknown
    try {
      parsed = JSON.parse(raw)
    } catch {
      return []
    }
    if (!Array.isArray(parsed)) return []
    const out: MeldGroup[] = []
    for (const item of parsed) {
      if (!item || typeof item !== 'object') continue
      const m = item as Record<string, unknown>
      const kind = m.kind as MeldKind
      if (
        kind !== 'chi' &&
        kind !== 'pon' &&
        kind !== 'ankan' &&
        kind !== 'minkan' &&
        kind !== 'kakan'
      ) {
        continue
      }
      const rawTiles = Array.isArray(m.tiles) ? (m.tiles as unknown[]) : []
      const tiles: Tile[] = []
      for (const code of rawTiles) {
        if (typeof code !== 'string') continue
        const t = tileFromCuiCode(code)
        if (t) tiles.push(t)
      }
      const fromOffsetRaw = m.fromOffset
      const fromOffset: MeldGroup['fromOffset'] =
        fromOffsetRaw === null || fromOffsetRaw === undefined
          ? null
          : (((fromOffsetRaw as number) % 4) as 0 | 1 | 2 | 3)
      const claimedRaw = m.claimedIndex
      const claimedIndex: number | null =
        claimedRaw === null || claimedRaw === undefined
          ? null
          : (claimedRaw as number)
      out.push({ kind, tiles, fromOffset, claimedIndex })
    }
    return out
  }

  // ---- リーチ ----
  canRiichi(): boolean {
    return this.game.canRiichi()
  }

  declareRiichi(): boolean {
    return this.game.declareRiichi()
  }

  /** #60: オープンリーチを宣言する (通常立直 + 手牌公開 + 和了時 +1 飜)。 */
  declareOpenRiichi(): boolean {
    if (typeof this.game.declareOpenRiichi === 'function') {
      return this.game.declareOpenRiichi() as boolean
    }
    return false
  }

  isPlayerRiichi(playerIdx: number): boolean {
    return this.game.isPlayerRiichi(playerIdx)
  }

  isPlayerOpenRiichi(playerIdx: number): boolean {
    if (typeof this.game.isPlayerOpenRiichi === 'function') {
      return this.game.isPlayerOpenRiichi(playerIdx) as boolean
    }
    return false
  }

  // ---- 和了宣言 (Issue #35) ----

  /**
   * 指定プレイヤーがツモ和了可能か。
   * UI 側はこれが true のときだけ「ツモ」ボタンを enable する。
   *
   * @param playerIdx 判定対象プレイヤーの座席 index。省略時は内部 currentPlayer
   *   (主にテスト用途。アプリ実装側からは明示指定すること)
   */
  canTsumo(playerIdx?: PlayerIndex): boolean {
    const idx = playerIdx ?? (this.game.getCurrentPlayerId() as PlayerIndex)
    return this.game.canTsumo(idx)
  }

  /**
   * 指定プレイヤーが直前打牌に対してロン可能か。
   * @param playerIdx 判定対象プレイヤーの座席 index (必須)
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

  /**
   * 直前打牌に対するロンを見逃したことを宣言する (Issue #56)。
   *
   * 呼び出し側 (App の `skipMeldCall` 等) は `canRon(playerIdx)` が true の状態で
   * ロン宣言を選ばずに通常進行に戻した場面でのみ本 API を呼ぶ。WASM 側で:
   *   - 同巡フリテン: `skipped_ron_this_turn = true`
   *   - 立直済みなら永続フリテン: `permanent_furiten = true`
   * のフラグを立て、以降の `canRon` を強制的に false に落とす。
   *
   * 関数自体はべき等で、`canRon` 再判定はしない。
   */
  skipRon(playerIdx: PlayerIndex): void {
    this.game.skipRon(playerIdx)
  }

  // ---- 嘘リーチ (Issue #89) ----

  /**
   * 嘘リーチ（黙聴での虚偽リーチ）を許可するかどうかを設定する。
   * true のとき canRiichi のテンパイ・点数要件を外し、門前 + 未リーチのみで宣言可能にする。
   * 嘘リーチに追加の罰符は無く、流局で露見した場合の損失は普通の不成立リーチと同じ
   * （リーチ棒 1000 点の供託没収 + テンパイ外なのでノーテン罰符）。
   */
  setUsoRiichiEnabled(enabled: boolean): void {
    if (typeof this.game.setUsoRiichiEnabled === 'function') {
      this.game.setUsoRiichiEnabled(enabled)
    }
  }

  isUsoRiichiEnabled(): boolean {
    if (typeof this.game.isUsoRiichiEnabled === 'function') {
      return this.game.isUsoRiichiEnabled() as boolean
    }
    return false
  }

  /**
   * 指定プレイヤーが嘘リーチ中かどうかを返す。
   * 流局時の手牌公開判定に使用する。
   */
  isUsoRiichi(playerIdx: PlayerIndex): boolean {
    if (typeof this.game.isUsoRiichi === 'function') {
      return this.game.isUsoRiichi(playerIdx) as boolean
    }
    return false
  }

  // ---- 理牌 / 自動ツモ (Issue #80 / #81) ----

  /** #80: 手牌自動ソート（理牌）ON/OFF を設定する。 */
  setAutoSort(enabled: boolean): void {
    if (typeof this.game.setAutoSort === 'function') {
      this.game.setAutoSort(enabled)
    }
  }

  isAutoSortEnabled(): boolean {
    if (typeof this.game.isAutoSortEnabled === 'function') {
      return this.game.isAutoSortEnabled() as boolean
    }
    return true
  }

  /** #80: 現在の手番プレイヤーの手牌を即時ソートする。 */
  sortCurrentHand(): void {
    if (typeof this.game.sortCurrentHand === 'function') {
      this.game.sortCurrentHand()
    }
  }

  /** #81: 人間プレイヤーターン開始時の自動ツモ ON/OFF を設定する。 */
  setAutoDraw(enabled: boolean): void {
    if (typeof this.game.setAutoDraw === 'function') {
      this.game.setAutoDraw(enabled)
    }
  }

  isAutoDrawEnabled(): boolean {
    if (typeof this.game.isAutoDrawEnabled === 'function') {
      return this.game.isAutoDrawEnabled() as boolean
    }
    return true
  }

  /** #59: 食い替え禁止の強制 ON/OFF を設定する。 */
  setEnforceKuikae(enabled: boolean): void {
    if (typeof this.game.setEnforceKuikae === 'function') {
      this.game.setEnforceKuikae(enabled)
    }
  }

  isEnforceKuikae(): boolean {
    if (typeof this.game.isEnforceKuikae === 'function') {
      return this.game.isEnforceKuikae() as boolean
    }
    return true
  }

  /** #129: 喰いタン (鳴きタンヤオ) を認めるかを設定する。 */
  setAllowOpenTanyao(allowed: boolean): void {
    if (typeof this.game.setAllowOpenTanyao === 'function') {
      this.game.setAllowOpenTanyao(allowed)
    }
  }

  isAllowOpenTanyao(): boolean {
    if (typeof this.game.isAllowOpenTanyao === 'function') {
      return this.game.isAllowOpenTanyao() as boolean
    }
    return true
  }

  /** #58: ローカル役満 (人和/大車輪/四連刻/百万石/三連刻) を認めるかを設定する。 */
  setAllowLocalYakuman(allowed: boolean): void {
    if (typeof this.game.setAllowLocalYakuman === 'function') {
      this.game.setAllowLocalYakuman(allowed)
    }
  }

  isAllowLocalYakuman(): boolean {
    if (typeof this.game.isAllowLocalYakuman === 'function') {
      return this.game.isAllowLocalYakuman() as boolean
    }
    return false
  }

  /**
   * #61: 本場縛りルールを設定する。
   * 0=Standard / 1=5本場以降2飜縛り / 2=5本場以降満貫縛り / 3=7本場以降役満縛り。
   */
  setShibariRule(rule: number): void {
    if (typeof this.game.setShibariRule === 'function') {
      this.game.setShibariRule(rule)
    }
  }

  getShibariRule(): number {
    if (typeof this.game.getShibariRule === 'function') {
      return this.game.getShibariRule() as number
    }
    return 0
  }

  // ---- デバッグ (Issue #79) ----

  /**
   * #79: 指定プレイヤーの手牌を文字列で返す（デバッグ用）。
   * CPU 手牌を表向き表示するために使用する。
   */
  getPlayerHandString(playerIdx: PlayerIndex): string {
    if (typeof this.game.getPlayerHandString === 'function') {
      return this.game.getPlayerHandString(playerIdx) as string
    }
    return ''
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

  // ---- 鳴き系 (Issue #5/#6) ----

  /** 指定プレイヤーが (他家の打牌に対して) チー可能か。下家のみ true になる。 */
  canChi(playerIdx: PlayerIndex): boolean {
    return this.game.canChi(playerIdx)
  }

  /** 指定プレイヤーが (他家の打牌に対して) ポン可能か。 */
  canPon(playerIdx: PlayerIndex): boolean {
    return this.game.canPon(playerIdx)
  }

  /** 指定プレイヤーが (他家の打牌に対して) 明槓可能か。 */
  canKan(playerIdx: PlayerIndex): boolean {
    return this.game.canKan(playerIdx)
  }

  /**
   * チーを実行。pattern は 0=(n-2,n-1,n) / 1=(n-1,n,n+1) / 2=(n,n+1,n+2)。
   * 成功すれば true。失敗 (パターン不成立等) は false。
   */
  doChi(playerIdx: PlayerIndex, pattern: number): boolean {
    return this.game.doChi(playerIdx, pattern)
  }

  /** ポンを実行。成功 true / 失敗 false。 */
  doPon(playerIdx: PlayerIndex): boolean {
    return this.game.doPon(playerIdx)
  }

  /** 明槓を実行。成功 true / 失敗 false。 */
  doKan(playerIdx: PlayerIndex): boolean {
    return this.game.doKan(playerIdx)
  }

  // ---- 暗槓 / 加槓 (Issue #46) ----

  /**
   * 暗槓可能な牌の一覧。手牌に 4 枚揃いがある牌だけ返す。
   * Rust 側は空白区切りの tile-string を返すので Tile[] に parse する。
   * (空文字は空配列)
   */
  canAnkan(playerIdx: PlayerIndex): Tile[] {
    const raw = this.game.canAnkan(playerIdx)
    return parseTileList(raw)
  }

  /**
   * 暗槓を実行する。指定牌が 4 枚揃ってない / プレイヤー idx 不正なら false。
   * 成功すると嶺上ツモ + 槓ドラ追加 + last_was_rinshan=true (Rinshan 役発火)。
   */
  doAnkan(playerIdx: PlayerIndex, tile: Tile): boolean {
    return this.game.doAnkan(playerIdx, tileToCuiCode(tile))
  }

  /**
   * 加槓 (小明槓) 可能な牌の一覧。
   * 既存の Pon 副露と同じ牌が手牌に 1 枚以上ある場合のみ候補に上がる。
   */
  canShouminkan(playerIdx: PlayerIndex): Tile[] {
    const raw = this.game.canShouminkan(playerIdx)
    return parseTileList(raw)
  }

  /**
   * 加槓宣言を**開始**する (2 段階 API の前半)。
   * 戻り値: `{ ok, candidates }`
   * - ok=false: 宣言不可 (候補に含まれない / 既に pending 中等)
   * - candidates: 当該 tile でロン (槍槓) できる他家の座席 index 一覧
   *   - 空: 即 `completeShouminkan` で確定してよい
   *   - 非空: UI 側で槍槓ロンの猶予を見せた後、誰も宣言しなければ
   *     `completeShouminkan`、誰かが宣言したら `resolveWinChankan` + `cancelShouminkan`
   */
  startShouminkan(playerIdx: PlayerIndex, tile: Tile): {
    ok: boolean
    candidates: PlayerIndex[]
  } {
    const raw = this.game.startShouminkan(playerIdx, tileToCuiCode(tile))
    if (!raw) return { ok: false, candidates: [] }
    try {
      const parsed = JSON.parse(raw) as { ok?: boolean; candidates?: number[] }
      return {
        ok: parsed.ok === true,
        candidates: Array.isArray(parsed.candidates)
          ? parsed.candidates.map(n => n as PlayerIndex)
          : [],
      }
    } catch {
      return { ok: false, candidates: [] }
    }
  }

  /**
   * 加槓を**完了**する (2 段階 API の後半、誰もロン宣言しなかった場合)。
   * 内部で Pon meld → Kan meld 書き換え + 嶺上ツモ + 槓ドラ追加。
   */
  completeShouminkan(playerIdx: PlayerIndex, tile: Tile): boolean {
    return this.game.completeShouminkan(playerIdx, tileToCuiCode(tile))
  }

  /**
   * 加槓宣言をキャンセルする (誰かが槍槓ロンを宣言した場合に呼ぶ)。
   * `pending_chankan` を None に戻すだけのべき等な API。
   */
  cancelShouminkan(): void {
    this.game.cancelShouminkan()
  }

  /**
   * 槍槓ロン (加槓宣言中の牌でのロン) を確定する。
   * `pending_chankan` の tile を winning_tile として使う。
   * 戻り値は通常ロンと同じ `RoundWinSummary | null`。
   */
  resolveWinChankan(
    winnerIdx: PlayerIndex,
    fromIdx: PlayerIndex
  ): RoundWinSummary | null {
    const json = this.game.resolveWinChankan(winnerIdx, fromIdx)
    return parseSummaryAsWin(json, winnerIdx, 'ron', fromIdx)
  }

  /**
   * wasm-bindgen が生成した free() を呼んでメモリを解放する。
   * ゲーム終了時 / リスタート時に必ず呼ぶこと。
   */
  destroy(): void {
    this.game.free()
  }
}

/**
 * 空白区切りの tile-string を Tile[] に分解する。
 * 例: `"5m 8p"` → [{suit:'man',value:5}, {suit:'pin',value:8}]
 * 空文字 / 不正 token は除外する。
 */
const parseTileList = (raw: string): Tile[] => {
  if (!raw) return []
  const out: Tile[] = []
  for (const token of raw.split(/\s+/)) {
    if (!token) continue
    const tile = parseTileToken(token)
    if (tile) out.push(tile)
  }
  return out
}

const parseTileToken = (code: string): Tile | null => {
  // 数牌: "5m" / "5mr" 等
  const numMatch = /^([1-9])([mps])(r?)$/.exec(code)
  if (numMatch) {
    const [, value, suitChar, red] = numMatch
    const suit: Tile['suit'] =
      suitChar === 'm' ? 'man' : suitChar === 'p' ? 'pin' : 'sou'
    const tile: Tile = { suit, value: Number(value) }
    if (red) tile.isRed = true
    return tile
  }
  switch (code) {
    case 'to': return { suit: 'wind', value: 1 }
    case 'na': return { suit: 'wind', value: 2 }
    case 'sa': return { suit: 'wind', value: 3 }
    case 'pe': return { suit: 'wind', value: 4 }
    case 'hk': return { suit: 'dragon', value: 1 }
    case 'ht': return { suit: 'dragon', value: 2 }
    case 'cn': return { suit: 'dragon', value: 3 }
    default: return null
  }
}
