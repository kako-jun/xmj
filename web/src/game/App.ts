import { Application, Container, Graphics } from 'pixi.js'
import {
  STAGE_WIDTH,
  STAGE_HEIGHT,
  TABLE_BG_COLOR,
  EVENT_LOG_LIMIT,
  EVENT_LOG_VISIBLE_COUNT,
} from './constants'
import { createTableScene } from './table'
import type { DiceRoll, GameMode, GameModeOption, GameState, PlayerIndex, Tile } from './types'
import { createGameStateFromBridge } from './bridgeState'
import { diceRollToHumanSeat, tileFromCuiCode, tileToCuiCode, tileToGlyph } from './types'
import { WasmGameBridge } from './wasm'
import { createTitleScene } from './titleScene'
import { createModeSelectScene } from './modeSelectScene'
import { createDiceRollScene } from './diceRollScene'
import { createResultScene, type ResultEntry } from './resultScene'
import { createRoundResultScene } from './roundResultScene'
import { parseRoundOutcome } from './types'
import type { RoundOutcome } from './types'
import {
  installKeyboardShortcuts,
  renderHtmlUi,
  type HtmlUiActionButton,
  type HtmlUiState,
} from './htmlUi'

interface AppOptions {
  cpuTurnDelayMs?: number
  createBridge?: ((humanSeat: PlayerIndex) => WasmGameBridge) | null
  /** 場決め用サイコロの乱数源。テストで決定的に注入できる。 */
  rollDice?: () => DiceRoll
  /** HTML オーバーレイの描画先 (ui-side 要素)。省略時は document.getElementById('ui-side') */
  htmlUiRoot?: HTMLElement | null
}

/**
 * 「ユーザーがタッチで答えるべき確認モーダル」の状態。
 * - 'riichi-prompt': 自家ツモ後、立直宣言可能なので「リーチ / リーチしない」を尋ねている
 * - 'meld-call': 他家打牌後、ロン/ポン/カン/チーのいずれかが可能なので尋ねている
 *   能なものだけボタンに出す。優先順位: ロン > ポン/カン > チー (麻雀標準)
 */
export type PendingDecision =
  | { kind: 'riichi-prompt' }
  | {
      kind: 'meld-call'
      from: PlayerIndex
      /**
       * 鳴き対象の牌 (= 直前打牌)。基本的には `gameState.lastDiscard` と同じ。
       * テスト用 mock 等で lastDiscard が parse 不可な場合は null になり得るが、
       * pendingDecision を立てる判定自体は canRon/canPon/canKan/canChi だけで行う。
       */
      tile: Tile | null
      canRon: boolean
      canPon: boolean
      canKan: boolean
      canChi: boolean
    }
  | {
      /**
       * 自家ツモ後の暗槓 / 加槓 (小明槓) 宣言モーダル。
       * Issue #46: 4 枚揃いまたは Pon 副露と一致する手牌があるとき、ユーザに
       * 「カンする / しない」を尋ねる。複数候補がある場合は最初の候補を採用する
       * 簡易実装 (UI で牌選択 picker を出すのは follow-up)。
       *
       * - `ankan`: 暗槓可能な牌一覧 (空でないなら少なくとも 1 つは選べる)
       * - `shouminkan`: 加槓可能な牌一覧
       * - 両方とも空配列で本 variant は立たないので、UI は少なくとも 1 種類のボタンを出す
       */
      kind: 'self-kan-prompt'
      ankan: Tile[]
      shouminkan: Tile[]
    }

const defaultRollDice = (): DiceRoll => ({
  d1: 1 + Math.floor(Math.random() * 6),
  d2: 1 + Math.floor(Math.random() * 6),
})

/**
 * `after` のマルチセットから `before` のマルチセットを引いて、新しく増えた牌を 1 枚返す。
 * 1 枚多くなっているはずだが、差分が無いなら null。複数あれば先頭を返す (普通発生しない)。
 */
const diffNewlyAddedTile = (before: Tile[], after: Tile[]): Tile | null => {
  const counts = new Map<string, number>()
  for (const t of before) {
    const k = tileToCuiCode(t)
    counts.set(k, (counts.get(k) ?? 0) + 1)
  }
  for (const t of after) {
    const k = tileToCuiCode(t)
    const left = counts.get(k) ?? 0
    if (left <= 0) return t
    counts.set(k, left - 1)
  }
  return null
}

/** 現在表示しているシーン種別。HTML overlay の表示内容を切り替えるのに使う。 */
type ActiveScene = 'title' | 'mode-select' | 'dice-roll' | 'table' | 'round-result' | 'result'

export class App {
  app: Application
  bridge: WasmGameBridge | null = null
  humanPlayerIndex: PlayerIndex = 0
  gameState: GameState | null = null
  selectedHandIndex: number | null = null
  eventLog: string[] = []
  resultMessage: string | null = null
  titleNotice: string | null = null
  /** 中間結果シーン表示中の局結果。null なら表示していない (= 対局中)。 */
  pendingRoundOutcome: RoundOutcome | null = null
  /**
   * 現在ユーザーがタッチで答えるべき確認モーダル状態。
   * 'meld-call' (旧 pendingRonChance を吸収) と 'riichi-prompt' を統合した。
   * null = モーダル無し = 通常プレイ。
   */
  pendingDecision: PendingDecision | null = null
  /**
   * ユーザーが「リーチ」モーダルで「リーチ」を選んだ後、打牌待ちの状態。
   * true なら次の打牌は `declareRiichi() + discardTile()` として扱われる。
   * 打牌成立 / 次局移行でリセット。
   */
  riichiArmed = false
  /**
   * このターンで一度リーチを「しない」と答えたか。同じターンで再度問わない用。
   * 次のターン (= 自家ツモが新しく入る) でリセットされる。
   */
  riichiDeclinedThisTurn = false
  /**
   * 自家が直近のツモで引いた牌。手牌の右端に分離表示するため。
   * 打牌で null に戻す。Rust 側 `hand.rs` が push 直後に sort() するため、
   * TS 側で `drawTile()` 前後の手牌差分から特定する。
   */
  justDrawnTile: Tile | null = null
  selectedGameMode: GameMode = 'tonpuusen'
  selectedHumanSeat: PlayerIndex | null = null
  /** #89: 嘘リーチ（黙聴でのリーチ）を許可するか。ゲーム開始前に設定する。 */
  usoRiichiEnabled = false
  private cpuTurnDelayMs: number
  private createBridge: ((humanSeat: PlayerIndex) => WasmGameBridge) | null
  private rollDice: () => DiceRoll
  private cpuTurnTask: Promise<void> | null = null
  private cpuTurnGeneration = 0
  private destroyedBridges = new WeakSet<WasmGameBridge>()
  private htmlUiRoot: HTMLElement | null
  private uninstallKeyboard: (() => void) | null = null
  private activeScene: ActiveScene = 'title'

  constructor(app: Application, options: AppOptions = {}) {
    this.app = app
    this.cpuTurnDelayMs = options.cpuTurnDelayMs ?? 0
    this.createBridge = options.createBridge ?? null
    this.rollDice = options.rollDice ?? defaultRollDice
    // #79: ?reveal=1 クエリでデバッグモード有効化
    if (typeof window !== 'undefined') {
      const params = new URLSearchParams(window.location.search)
      if (params.get('reveal') === '1') this.debugReveal = true
    }
    this.htmlUiRoot =
      options.htmlUiRoot ??
      (typeof document !== 'undefined' ? document.getElementById('ui-side') : null)
    if (this.htmlUiRoot) {
      this.uninstallKeyboard = installKeyboardShortcuts({
        onSelect: index => this.handleHotkeySelect(index),
        onDiscard: () => this.handleHotkeyDiscard(),
        onTsumo: () => this.handleHotkeyTsumo(),
        onRon: () => this.handleHotkeyRon(),
        onRiichiDiscard: () => this.handleHotkeyRiichiDiscard(),
        onCancel: () => this.handleHotkeyCancel(),
        onConfirm: () => this.handleHotkeyConfirm(),
        onBackTile: () => this.handleHotkeyShift(-1),
        onNextTile: () => this.handleHotkeyShift(1),
        onSortHand: () => this.handleHotkeySortHand(),
      })
    }
  }

  /** テスト用クリーンアップ。本番はページ離脱まで持つので任意。 */
  destroy(): void {
    this.uninstallKeyboard?.()
    this.uninstallKeyboard = null
    this.invalidateCpuTurnTask()
    this.releaseCurrentBridge()
  }

  showTableBackground(): void {
    const bg = new Graphics()
    bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: TABLE_BG_COLOR })
    this.app.stage.addChild(bg)
  }

  showInitialTable(gameState: GameState): void {
    this.invalidateCpuTurnTask()
    this.releaseCurrentBridge()
    this.gameState = gameState
    this.selectedHandIndex = null
    this.eventLog = []
    this.resultMessage = null
    this.titleNotice = null
    this.pendingRoundOutcome = null
    this.pendingDecision = null
    this.riichiArmed = false
    this.riichiDeclinedThisTurn = false
    this.justDrawnTile = null
    this.activeScene = 'table'
    this.renderTable()
  }

  showTitleScene(notice: string | null = null): void {
    this.invalidateCpuTurnTask()
    this.releaseCurrentBridge()
    this.gameState = null
    this.selectedHandIndex = null
    this.eventLog = []
    this.resultMessage = null
    this.titleNotice = notice
    this.pendingRoundOutcome = null
    this.pendingDecision = null
    this.riichiArmed = false
    this.riichiDeclinedThisTurn = false
    this.justDrawnTile = null
    this.activeScene = 'title'
    this.replaceStageRoot(
      createTitleScene({
        notice: this.titleNotice,
        startEnabled: this.createBridge !== null,
        onStart: () => {
          this.showModeSelectScene()
        },
      })
    )
    this.renderHtmlOverlay()
  }

  showModeSelectScene(): void {
    this.invalidateCpuTurnTask()
    this.activeScene = 'mode-select'
    this.replaceStageRoot(
      createModeSelectScene({
        selectedMode: this.selectedGameMode,
        modes: this.buildGameModes(),
        usoRiichiEnabled: this.usoRiichiEnabled,
        autoDrawEnabled: this.autoDraw,
        autoSortEnabled: this.autoSort,
        // タップ即確定: 選択 + 場決めシーン遷移を 1 タップで行う
        onSelectMode: mode => {
          this.selectedGameMode = mode
          this.showDiceRollScene()
        },
        onBack: () => {
          this.showTitleScene(this.titleNotice)
        },
        onToggleUsoRiichi: enabled => {
          this.usoRiichiEnabled = enabled
          this.showModeSelectScene()
        },
        onToggleAutoDraw: enabled => {
          this.autoDraw = enabled
          this.showModeSelectScene()
        },
        onToggleAutoSort: enabled => {
          this.autoSort = enabled
          this.showModeSelectScene()
        },
      })
    )
    this.renderHtmlOverlay()
  }

  showDiceRollScene(roll?: DiceRoll): void {
    this.invalidateCpuTurnTask()
    const settledRoll = roll ?? this.rollDice()
    const humanSeat = diceRollToHumanSeat(settledRoll)
    this.selectedHumanSeat = humanSeat

    // 対局画面を先に出してから、その上にサイコロ overlay を被せる
    const started = this.startNewGame()
    if (!started) return

    this.activeScene = 'dice-roll'
    const overlay = createDiceRollScene({
      roll: settledRoll,
      humanSeat,
      onComplete: () => {
        overlay.parent?.removeChild(overlay)
        overlay.destroy({ children: true })
        this.activeScene = 'table'
        // 起家が CPU の場合 (人間が南/西/北席) は手番ループを起こさないと
        // 永遠に CPU が打たず、操作ボタンも出ないまま止まる。
        // 人間起家ならこの呼び出しは UI 再描画だけで no-op になる。
        this.advanceTurnLoop()
      },
    })
    this.app.stage.addChild(overlay)
    this.renderHtmlOverlay()
  }

  startNewGame(): boolean {
    if (!this.createBridge) {
      this.showTitleScene('対局開始に必要な bridge factory が未設定です。')
      return false
    }
    const humanSeat = this.selectedHumanSeat
    if (humanSeat === null) {
      this.showTitleScene('場決めが行われていません。')
      return false
    }

    try {
      const bridge = this.createBridge(humanSeat)
      this.titleNotice = null
      // #89: 嘘リーチ設定をブリッジに反映
      if (this.usoRiichiEnabled && typeof bridge.setUsoRiichiEnabled === 'function') {
        bridge.setUsoRiichiEnabled(true)
      }
      // #80: 理牌設定をブリッジに反映
      if (typeof bridge.setAutoSort === 'function') {
        bridge.setAutoSort(this.autoSort)
      }
      // #81: 自動ツモ設定をブリッジに反映
      if (typeof bridge.setAutoDraw === 'function') {
        bridge.setAutoDraw(this.autoDraw)
      }
      this.startGame(bridge, humanSeat)
      return true
    } catch (error) {
      const message =
        error instanceof Error
          ? `対局の初期化に失敗しました: ${error.message}`
          : '対局の初期化に失敗しました。'
      this.showTitleScene(message)
      return false
    }
  }

  startGame(bridge: WasmGameBridge, humanPlayerIndex: PlayerIndex): void {
    this.invalidateCpuTurnTask()
    if (this.bridge !== bridge) {
      this.releaseCurrentBridge()
    }
    this.bridge = bridge
    this.humanPlayerIndex = humanPlayerIndex
    this.selectedHandIndex = null
    this.eventLog = ['対局開始']
    this.resultMessage = null
    this.titleNotice = null
    this.pendingRoundOutcome = null
    this.pendingDecision = null
    this.riichiArmed = false
    this.riichiDeclinedThisTurn = false
    this.justDrawnTile = null
    this.activeScene = 'table'
    this.refreshFromBridge()

    if (this.shouldDrawHumanTile()) {
      this.drawHumanTileAndRefresh()
    } else {
      // 既に 14 枚状態 (テストで事前に手牌をセットしている等) でも、
      // カン / 立直可能ならその場でモーダルを出す。
      if (!this.maybePromptSelfKan()) {
        this.maybePromptRiichi()
      }
      this.renderTable()
    }
  }

  private refreshFromBridge(): void {
    if (!this.bridge) return
    this.gameState = createGameStateFromBridge(this.bridge, this.humanPlayerIndex)
    const humanHand = this.gameState.players[this.humanPlayerIndex].hand
    if (
      this.selectedHandIndex !== null &&
      (this.selectedHandIndex < 0 || this.selectedHandIndex >= humanHand.length)
    ) {
      this.selectedHandIndex = null
    }
    this.renderTable()
  }

  /** #81: 自動ツモ設定。false のとき shouldDrawHumanTile が false を返す。 */
  autoDraw = true
  /** #80: 自動理牌設定。false のときツモ後に sort しない。 */
  autoSort = true
  /** #79: デバッグモード。true のとき CPU 手牌を表向き表示する。 */
  debugReveal = false

  private shouldDrawHumanTile(): boolean {
    if (!this.bridge || !this.gameState) return false
    if (!this.bridge.isCurrentPlayerHuman() || this.bridge.isGameOver()) return false
    if (!this.autoDraw) return false  // #81: 手動ツモ設定
    return this.gameState.players[this.humanPlayerIndex].hand.length % 3 === 1
  }

  private appendLog(message: string): void {
    this.eventLog = [...this.eventLog.slice(-(EVENT_LOG_LIMIT - 1)), message]
  }

  private getPlayerName(playerIndex: PlayerIndex): string {
    return this.bridge?.getPlayerName(playerIndex) ?? `P${playerIndex + 1}`
  }

  /**
   * 実況ログ・ボタンラベル等の人間可読 UI 文字列。Unicode 麻雀牌 1 文字を返す。
   * 内部キーは {@link tileToCuiCode} を使う (5m と 5mr 等の差別化が必要なため別関数)。
   */
  private formatTile(tile: Tile): string {
    return tileToGlyph(tile)
  }

  /**
   * ツモ通知用。残り山牌数を出すのは「ツモ」「カン (嶺上)」等のドロー系イベントだけ。
   * 打牌ログには使わない (山番号は他家から見て手中の何枚目だったかを示唆してしまい、
   * 開示すべき情報ではない)。
   */
  private wallSummary(): string {
    const wallCount = this.bridge?.getWallCount() ?? this.gameState?.wall.length ?? 0
    return `(山${wallCount})`
  }

  private sleep(ms: number): Promise<void> {
    return new Promise(resolve => {
      window.setTimeout(resolve, ms)
    })
  }

  private invalidateCpuTurnTask(): void {
    this.cpuTurnGeneration += 1
    this.cpuTurnTask = null
  }

  private destroyBridgeOnce(bridge: WasmGameBridge | null): void {
    if (!bridge || this.destroyedBridges.has(bridge)) return
    bridge.destroy()
    this.destroyedBridges.add(bridge)
  }

  private releaseCurrentBridge(): void {
    this.destroyBridgeOnce(this.bridge)
    this.bridge = null
    this.pendingDecision = null
    this.riichiArmed = false
    this.riichiDeclinedThisTurn = false
    this.justDrawnTile = null
  }

  private isCpuTurnGenerationCurrent(generation: number): boolean {
    return generation === this.cpuTurnGeneration
  }

  /** #81: 手動ツモ。autoDraw=false のときボタン/T キーから呼ばれる。 */
  private doManualDraw(): void {
    if (!this.bridge || !this.gameState) return
    const hand = this.gameState.players[this.humanPlayerIndex].hand
    if (hand.length % 3 !== 1) return  // ツモ前でないなら何もしない
    this.drawHumanTileAndRefresh()
    this.renderHtmlOverlay()
  }

  private drawHumanTileAndRefresh(): boolean {
    if (!this.bridge) return false
    // Rust 側 `hand.rs:33-34` は push 直後に sort() するため「どれが今ツモった牌か」
    // の情報が落ちる。ツモ前後の手牌をマルチセット差分でとって右端表示用に保存する。
    const beforeHand = this.gameState
      ? this.gameState.players[this.humanPlayerIndex].hand.slice()
      : []
    const drew = this.bridge.drawTile()
    if (!drew) {
      // 引けなかった (山牌切れ等) ので、前ターンの「ツモ牌右端表示」が残ったままに
      // ならないよう明示的にクリア。confirmSelectedTile 経由でクリアされる線も
      // あるが、ガードを早期 return より前に置いておくのが安全。
      this.justDrawnTile = null
      this.maybeFinalizeRoundFromDraw()
      return false
    }
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} がツモ ${this.wallSummary()}`)
    this.refreshFromBridge()
    const afterHand = this.gameState
      ? this.gameState.players[this.humanPlayerIndex].hand
      : []
    this.justDrawnTile = diffNewlyAddedTile(beforeHand, afterHand)
    // デフォルトの打牌候補をツモ牌に合わせる (UX: 連打で即切り)。
    // 立直/カン モーダル中の誤打牌は `isHumanTurnInteractive()` で pendingDecision を
    // チェックしているので、selectedHandIndex が tsumo に向いていても安全。
    this.selectedHandIndex = this.findHandIndexOfJustDrawn()
    // 新しいツモを引いた時点で、このターンの「リーチしない」決定はリセット。
    this.riichiDeclinedThisTurn = false
    // Issue #46: 暗槓 / 加槓が可能ならカンモーダル、そうでなければリーチモーダルへ
    if (!this.maybePromptSelfKan()) {
      this.maybePromptRiichi()
    }
    this.renderTable()
    return true
  }

  private findHandIndexOfJustDrawn(): number | null {
    if (!this.gameState || !this.justDrawnTile) return null
    const hand = this.gameState.players[this.humanPlayerIndex].hand
    const key = tileToCuiCode(this.justDrawnTile)
    for (let i = hand.length - 1; i >= 0; i--) {
      if (tileToCuiCode(hand[i]) === key) return i
    }
    return null
  }

  /**
   * 自家ツモ直後で立直可能なら、ユーザーに「リーチ / リーチしない」を尋ねる。
   * 既に立直済み・宣言不可・このターンで既に断った場合は何もしない。
   */
  private maybePromptRiichi(): void {
    if (!this.bridge) return
    if (this.bridge.isGameOver()) return
    if (!this.bridge.isCurrentPlayerHuman()) return
    if (this.riichiArmed) return
    if (this.riichiDeclinedThisTurn) return
    if (this.pendingDecision) return
    if (this.bridge.isPlayerRiichi(this.humanPlayerIndex)) return
    if (!this.bridge.canRiichi()) return
    this.pendingDecision = { kind: 'riichi-prompt' }
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} がリーチ可能`)
  }

  /**
   * 自家ツモ直後で暗槓 / 加槓 (小明槓) が可能なら、ユーザに「カン / しない」を尋ねる。
   * 戻り値: true ならモーダルを立てた (= カン候補があった)、false なら何もしない。
   *
   * Issue #46: 自家ツモ後カン UI。立直済みのプレイヤーが暗槓するルール (送り槓不可等)
   * は厳密には条件付きだが、本実装では「立直中はカンを出さない」シンプル運用。
   */
  private maybePromptSelfKan(): boolean {
    if (!this.bridge) return false
    if (this.bridge.isGameOver()) return false
    if (!this.bridge.isCurrentPlayerHuman()) return false
    if (this.pendingDecision) return false
    // 立直中は (簡易ルールで) 自家カンを出さない
    if (this.bridge.isPlayerRiichi(this.humanPlayerIndex)) return false
    // mock bridge 対応: canAnkan/canShouminkan が未実装でもクラッシュしない
    const ankan =
      typeof this.bridge.canAnkan === 'function'
        ? this.bridge.canAnkan(this.humanPlayerIndex)
        : []
    const shouminkan =
      typeof this.bridge.canShouminkan === 'function'
        ? this.bridge.canShouminkan(this.humanPlayerIndex)
        : []
    if (ankan.length === 0 && shouminkan.length === 0) return false
    this.pendingDecision = { kind: 'self-kan-prompt', ankan, shouminkan }
    const parts: string[] = []
    if (ankan.length > 0) parts.push(`暗槓 ${ankan.map(t => tileToGlyph(t)).join(' ')}`)
    if (shouminkan.length > 0)
      parts.push(`加槓 ${shouminkan.map(t => tileToGlyph(t)).join(' ')}`)
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} がカン可能 (${parts.join(' / ')})`)
    return true
  }

  private maybeFinalizeRoundFromDraw(): void {
    if (!this.bridge) return
    if (this.pendingRoundOutcome) return
    if (this.bridge.isGameOver()) return
    if (this.bridge.getWallCount() !== 0) return
    this.finalizeRoundFromDraw()
  }

  private finalizeRoundFromDraw(): void {
    if (!this.bridge) return
    if (this.pendingRoundOutcome) return
    const tenpai =
      typeof this.bridge.computeTenpaiPlayers === 'function'
        ? this.bridge.computeTenpaiPlayers()
        : []
    // #89: 流局前に嘘リーチプレイヤーのログを出力（手牌公開相当）
    if (typeof this.bridge.isUsoRiichi === 'function') {
      for (let i = 0; i < 4; i++) {
        if (this.bridge.isUsoRiichi(i as PlayerIndex)) {
          this.appendLog(
            `${this.getPlayerName(i as PlayerIndex)} の嘘リーチが発覚！手牌公開 + 1000点罰符`
          )
        }
      }
    }
    this.bridge.resolveDraw(tenpai)
    this.showRoundResultIfPending()
  }

  private confirmSelectedTile(options: { riichi?: boolean } = {}): boolean {
    if (!this.bridge || !this.gameState || this.selectedHandIndex === null) return false
    if (!this.bridge.isCurrentPlayerHuman()) return false
    if (this.pendingDecision) return false

    const tile = this.gameState.players[this.humanPlayerIndex].hand[this.selectedHandIndex]
    if (!tile) return false

    // explicit riichi 指定が無くても、riichiArmed 状態 (= モーダルで「リーチ」と
    // 答えた後の打牌) ならリーチ宣言として処理する。
    const riichi = options.riichi ?? this.riichiArmed

    if (riichi && !this.bridge.declareRiichi()) {
      return false
    }
    if (riichi) {
      this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} が立直`)
    }

    const discarded = this.bridge.discardTile(tile)
    if (!discarded) return false

    this.appendLog(
      `${this.getPlayerName(this.humanPlayerIndex)} が ${this.formatTile(tile)} を打牌`
    )
    this.selectedHandIndex = null
    this.justDrawnTile = null
    // 打牌で turn が完了したのでリーチ系状態をリセット (riichiArmed は使い切り)
    this.riichiArmed = false
    this.riichiDeclinedThisTurn = false
    this.advanceTurnLoop()
    return true
  }

  private advanceTurnLoop(): void {
    if (!this.bridge) return

    this.refreshFromBridge()
    this.finalizeGameIfNeeded()
    if (!this.bridge) return

    if (this.cpuTurnDelayMs > 0) {
      const generation = this.cpuTurnGeneration
      if (!this.cpuTurnTask) {
        this.cpuTurnTask = this.runCpuTurnsAsync(generation).finally(() => {
          if (this.isCpuTurnGenerationCurrent(generation)) {
            this.cpuTurnTask = null
          }
        })
      }
      return
    }

    while (this.bridge && !this.bridge.isGameOver() && this.bridge.isCurrentPlayerCpu()) {
      const currentPlayer = this.bridge.getCurrentPlayerId() as PlayerIndex
      const playerName = this.getPlayerName(currentPlayer)
      const discardedTile = this.bridge.executeCpuTurn()
      this.refreshFromBridge()
      const parsedTile = tileFromCuiCode(discardedTile)
      if (parsedTile === null) {
        // executeCpuTurn が「山牌がありません」「打牌できません」のようなエラー文字列を
        // 返したケース。「${playerName} が 山牌がありません を打牌」のような日本語破綻
        // ログを避けるため、ツモ行は出さず、エラー行を単独で残してループ break。
        this.appendLog(`${playerName}: ${discardedTile}`)
        this.finalizeGameIfNeeded()
        return
      }
      this.appendLog(`${playerName} がツモ ${this.wallSummary()}`)
      this.appendLog(`${playerName} が ${tileToGlyph(parsedTile)} を打牌`)
      this.finalizeGameIfNeeded()
      if (this.checkMeldChancesAfterDiscard(currentPlayer)) {
        return
      }
    }

    if (!this.bridge) return

    if (this.shouldDrawHumanTile()) {
      this.drawHumanTileAndRefresh()
    }

    this.finalizeGameIfNeeded()
  }

  private async runCpuTurnsAsync(generation: number): Promise<void> {
    if (!this.bridge) return

    while (
      this.isCpuTurnGenerationCurrent(generation) &&
      this.bridge &&
      !this.bridge.isGameOver() &&
      this.bridge.isCurrentPlayerCpu()
    ) {
      const currentPlayer = this.bridge.getCurrentPlayerId() as PlayerIndex
      const playerName = this.getPlayerName(currentPlayer)
      await this.sleep(this.cpuTurnDelayMs)
      if (!this.isCpuTurnGenerationCurrent(generation) || !this.bridge) return

      const discardedTile = this.bridge.executeCpuTurn()
      this.refreshFromBridge()
      if (!this.isCpuTurnGenerationCurrent(generation) || !this.bridge) return
      const parsedTile = tileFromCuiCode(discardedTile)
      if (parsedTile === null) {
        // 同期版と同じく、エラー戻り値で打牌ログが破綻しないようガード。
        this.appendLog(`${playerName}: ${discardedTile}`)
        this.finalizeGameIfNeeded()
        return
      }
      this.appendLog(`${playerName} がツモ ${this.wallSummary()}`)
      this.appendLog(`${playerName} が ${tileToGlyph(parsedTile)} を打牌`)
      this.finalizeGameIfNeeded()
      if (this.checkMeldChancesAfterDiscard(currentPlayer)) {
        return
      }

      if (
        this.isCpuTurnGenerationCurrent(generation) &&
        this.bridge &&
        !this.bridge.isGameOver() &&
        this.bridge.isCurrentPlayerCpu()
      ) {
        await this.sleep(this.cpuTurnDelayMs)
      }
    }

    if (this.isCpuTurnGenerationCurrent(generation) && this.shouldDrawHumanTile()) {
      this.drawHumanTileAndRefresh()
    }

    if (this.isCpuTurnGenerationCurrent(generation)) {
      this.finalizeGameIfNeeded()
    }
  }

  /**
   * CPU 等が打牌した直後に、自家がロン/ポン/カン/チー宣言できるか調べ、
   * 該当があれば pendingDecision を立てて選択を促す。戻り値 true なら turn loop を中断。
   *
   * 立直中はチー/ポン/カンを (自動的に) 行わないが、ロンだけは宣言できるべき。
   * 立直中は can_pon/can_kan/can_chi が false でも安全側に弾いておく。
   */
  private checkMeldChancesAfterDiscard(discarder: PlayerIndex): boolean {
    if (!this.bridge || !this.gameState) return false
    if (this.pendingRoundOutcome) return false
    if (this.pendingDecision) return false
    if (discarder === this.humanPlayerIndex) return false
    if (this.bridge.isGameOver()) return false

    const isHumanRiichi = this.bridge.isPlayerRiichi(this.humanPlayerIndex)
    const canRon = this.bridge.canRon(this.humanPlayerIndex)
    // 立直中は鳴き禁止 (ロンは別)
    const canPon = !isHumanRiichi && this.bridge.canPon(this.humanPlayerIndex)
    const canKan = !isHumanRiichi && this.bridge.canKan(this.humanPlayerIndex)
    const canChi = !isHumanRiichi && this.bridge.canChi(this.humanPlayerIndex)

    if (!canRon && !canPon && !canKan && !canChi) return false

    this.pendingDecision = {
      kind: 'meld-call',
      from: discarder,
      tile: this.gameState.lastDiscard,
      canRon,
      canPon,
      canKan,
      canChi,
    }
    this.invalidateCpuTurnTask()
    const options: string[] = []
    if (canRon) options.push('ロン')
    if (canPon) options.push('ポン')
    if (canKan) options.push('カン')
    if (canChi) options.push('チー')
    const tile = this.gameState.lastDiscard
    this.appendLog(
      tile
        ? `${this.getPlayerName(this.humanPlayerIndex)} に ${options.join('/')} 可能 (打牌: ${this.formatTile(tile)})`
        : `${this.getPlayerName(this.humanPlayerIndex)} に ${options.join('/')} 可能`
    )
    this.renderTable()
    return true
  }

  /**
   * Issue #63: 同種牌が複数あるとき、タップ対象を「同種牌の中で最も右にある index」に
   * 寄せる。table.ts のツモ牌分離 UI (直近ツモ牌を右端に切り離して描画) と意図を揃え、
   * 「タップした牌の正体は右端のツモ分離スロットに出る」と見えるようにする。
   * 赤ドラ含めて完全一致 (`tileToCuiCode` 比較) なので、5m と 5mr は別物扱い。
   */
  private coalesceSelectionIndex(index: number): number {
    if (!this.gameState) return index
    const hand = this.gameState.players[this.humanPlayerIndex].hand
    if (index < 0 || index >= hand.length) return index
    const key = tileToCuiCode(hand[index])
    let rightmost = index
    for (let i = hand.length - 1; i > index; i--) {
      if (tileToCuiCode(hand[i]) === key) {
        rightmost = i
        break
      }
    }
    return rightmost
  }

  private handleHandTileTap(index: number): void {
    if (!this.bridge || !this.gameState) return
    if (!this.bridge.isCurrentPlayerHuman()) return

    const target = this.coalesceSelectionIndex(index)

    if (this.selectedHandIndex === target) {
      this.confirmSelectedTile()
      return
    }

    this.selectedHandIndex = target
    this.renderTable()
  }

  /**
   * 「打牌」「ツモ」「ロン」「ポン」「カン」「チー」「リーチ」「見逃し」など、現在の状況で有効な
   * 行動ボタンを構築する。HTML overlay と内部のキーボードハンドラの両方で使う。
   *
   * 状態遷移:
   * - `pendingDecision.kind === 'meld-call'`: 他家打牌後の宣言モーダル。
   *   有効な宣言ボタン (ロン/ポン/カン/チー) + 「見逃し」のみ。
   * - `pendingDecision.kind === 'riichi-prompt'`: 自家ツモ後のリーチ確認モーダル。
   *   「リーチ」「リーチしない」+ ツモ可能なら「ツモ」を出す。
   * - それ以外: 通常の自家ターン操作 (打牌・ツモ)。
   *   `riichiArmed` が true なら打牌ボタンが「立直して打牌」になる。
   */
  private buildActionButtons(): HtmlUiActionButton[] {
    if (!this.bridge || !this.gameState) return []
    if (this.bridge.isGameOver()) return []

    if (this.pendingDecision?.kind === 'meld-call') {
      return this.buildMeldCallButtons(this.pendingDecision)
    }

    if (this.pendingDecision?.kind === 'riichi-prompt') {
      return this.buildRiichiPromptButtons()
    }

    if (this.pendingDecision?.kind === 'self-kan-prompt') {
      return this.buildSelfKanPromptButtons(this.pendingDecision)
    }

    const isHumanTurn = this.bridge.isCurrentPlayerHuman()
    if (!isHumanTurn) return []

    const canTsumo = this.bridge.canTsumo(this.humanPlayerIndex)
    const hand = this.gameState.players[this.humanPlayerIndex].hand
    // 手牌が 13 枚（打牌待ち枚数でなく、ツモ前の枚数）かつ autoDraw=false のとき手動ツモ
    const needsManualDraw = !this.autoDraw && hand.length % 3 === 1
    const buttons: HtmlUiActionButton[] = []

    // #81: 手動ツモボタン（autoDraw=false のとき && 手牌がツモ前の状態）
    if (needsManualDraw) {
      buttons.push({
        key: 'manual-draw',
        label: 'ツモる',
        enabled: true,
        hotkey: 'T',
        onActivate: () => {
          this.doManualDraw()
        },
      })
      // 手動ツモ待ち中は他のボタン不要
      return buttons
    }

    if (canTsumo) {
      buttons.push({
        key: 'tsumo',
        label: 'ツモあがり',
        enabled: true,
        hotkey: 'T',
        onActivate: () => {
          this.confirmTsumo()
        },
      })
    }

    buttons.push({
      key: this.riichiArmed ? 'riichi-discard' : 'discard',
      label: this.riichiArmed ? '立直して打牌' : '打牌',
      enabled: this.selectedHandIndex !== null,
      hotkey: this.riichiArmed ? 'L' : 'D',
      onActivate: () => {
        this.confirmSelectedTile()
      },
    })

    // Issue #62: armed リーチ中は「立直やめる」を出してキャンセルできるようにする。
    // Rust 側はまだ declareRiichi を呼んでいないので TS 側の状態を落とすだけで OK。
    if (this.riichiArmed) {
      buttons.push({
        key: 'riichi-cancel',
        label: '立直やめる',
        enabled: true,
        hotkey: 'Esc',
        onActivate: () => {
          this.disarmRiichi()
        },
      })
    }

    // #80: 理牌ボタン（常に表示）
    buttons.push({
      key: 'sort-hand',
      label: '理牌',
      enabled: true,
      hotkey: 'S',
      onActivate: () => {
        this.bridge?.sortCurrentHand()
        this.refreshFromBridge()
      },
    })

    return buttons
  }

  private buildMeldCallButtons(
    pending: Extract<PendingDecision, { kind: 'meld-call' }>
  ): HtmlUiActionButton[] {
    const buttons: HtmlUiActionButton[] = []
    if (pending.canRon) {
      buttons.push({
        key: 'ron',
        label: 'ロン',
        enabled: true,
        hotkey: 'R',
        onActivate: () => {
          this.confirmRon(pending.from)
        },
      })
    }
    if (pending.canPon) {
      buttons.push({
        key: 'pon',
        label: 'ポン',
        enabled: true,
        hotkey: 'P',
        onActivate: () => {
          this.confirmPon()
        },
      })
    }
    if (pending.canKan) {
      buttons.push({
        key: 'kan',
        label: 'カン',
        enabled: true,
        hotkey: 'K',
        onActivate: () => {
          this.confirmKan()
        },
      })
    }
    if (pending.canChi) {
      buttons.push({
        key: 'chi',
        label: 'チー',
        enabled: true,
        hotkey: 'C',
        onActivate: () => {
          this.confirmChi()
        },
      })
    }
    buttons.push({
      key: 'meld-skip',
      label: '見逃し',
      enabled: true,
      hotkey: 'Esc',
      onActivate: () => {
        this.skipMeldCall()
      },
    })
    return buttons
  }

  /**
   * 自家ツモ後の暗槓 / 加槓モーダル用ボタン。
   * 暗槓と加槓が両方候補にある場合、両ボタンを並べる。複数牌候補は最初のものを採用 (簡易)。
   */
  private buildSelfKanPromptButtons(
    pending: Extract<PendingDecision, { kind: 'self-kan-prompt' }>
  ): HtmlUiActionButton[] {
    const buttons: HtmlUiActionButton[] = []
    if (pending.ankan.length > 0) {
      const tile = pending.ankan[0]
      buttons.push({
        key: 'self-ankan',
        label: `暗槓 ${tileToGlyph(tile)}`,
        enabled: true,
        hotkey: 'K',
        onActivate: () => {
          this.confirmAnkan(tile)
        },
      })
    }
    if (pending.shouminkan.length > 0) {
      const tile = pending.shouminkan[0]
      buttons.push({
        key: 'self-shouminkan',
        label: `加槓 ${tileToGlyph(tile)}`,
        enabled: true,
        hotkey: pending.ankan.length > 0 ? 'M' : 'K',
        onActivate: () => {
          this.confirmShouminkan(tile)
        },
      })
    }
    buttons.push({
      key: 'self-kan-skip',
      label: 'カンしない',
      enabled: true,
      hotkey: 'Esc',
      onActivate: () => {
        this.skipSelfKan()
      },
    })
    return buttons
  }

  private buildRiichiPromptButtons(): HtmlUiActionButton[] {
    const buttons: HtmlUiActionButton[] = []
    if (this.bridge?.canTsumo(this.humanPlayerIndex)) {
      buttons.push({
        key: 'tsumo',
        label: 'ツモあがり',
        enabled: true,
        hotkey: 'T',
        onActivate: () => {
          this.confirmTsumo()
        },
      })
    }
    buttons.push({
      key: 'riichi',
      label: 'リーチ',
      enabled: true,
      hotkey: 'L',
      onActivate: () => {
        this.armRiichi()
      },
    })
    buttons.push({
      key: 'riichi-skip',
      label: 'リーチしない',
      enabled: true,
      hotkey: 'Esc',
      onActivate: () => {
        this.declineRiichi()
      },
    })
    return buttons
  }

  private confirmTsumo(): void {
    if (!this.bridge) return
    if (!this.bridge.isCurrentPlayerHuman()) return
    if (!this.bridge.canTsumo(this.humanPlayerIndex)) return
    const summary = this.bridge.resolveWinTsumo(this.humanPlayerIndex)
    if (!summary) {
      this.appendLog('ツモ宣言失敗')
      return
    }
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} がツモ和了`)
    this.pendingDecision = null
    this.riichiArmed = false
    this.riichiDeclinedThisTurn = false
    this.justDrawnTile = null
    this.showRoundResultIfPending()
  }

  private confirmRon(fromIdx: PlayerIndex): void {
    if (!this.bridge) return
    if (this.pendingDecision?.kind !== 'meld-call') return
    if (!this.bridge.canRon(this.humanPlayerIndex)) {
      this.pendingDecision = null
      this.advanceTurnLoop()
      return
    }
    const summary = this.bridge.resolveWinRon(this.humanPlayerIndex, fromIdx)
    this.pendingDecision = null
    if (!summary) {
      this.appendLog('ロン宣言失敗（和了形不成立）')
      this.advanceTurnLoop()
      return
    }
    this.showRoundResultIfPending()
  }

  /**
   * ポン宣言。成功なら手番は自家に移り、即座に打牌待ち状態にする (justDrawnTile は無し)。
   */
  private confirmPon(): void {
    if (!this.bridge) return
    if (this.pendingDecision?.kind !== 'meld-call') return
    if (!this.bridge.canPon(this.humanPlayerIndex)) {
      this.skipMeldCall()
      return
    }
    const ok = this.bridge.doPon(this.humanPlayerIndex)
    this.pendingDecision = null
    if (!ok) {
      this.appendLog('ポン宣言失敗')
      this.advanceTurnLoop()
      return
    }
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} がポン`)
    this.justDrawnTile = null
    this.selectedHandIndex = null
    // 鳴き成功で手番が自家に移ったので、CPU 連鎖の旧世代が打牌しないよう
    // 世代カウンタを bump (cpuTurnDelayMs>0 で task が走っているケースの保険)。
    this.invalidateCpuTurnTask()
    this.refreshFromBridge()
    // do_pon 後、Rust 側で current_player が humanPlayerIndex に移っている。
    // 手番が自家なので普通に「打牌」UI が出る。advanceTurnLoop を呼ばない (まだ打牌前)。
  }

  /**
   * 明槓 (他家の打牌に対するカン) 宣言。
   * 明槓は嶺上ツモ 1 枚 + 槓ドラ 1 枚追加を Rust 側 (`game.rs::do_kan`) で実行する。
   * ここでは do_kan 前後で自家の手牌を比較し、嶺上から増えた 1 枚を抽出して
   * `justDrawnTile` に反映する。これによりカン直後も「ツモ牌右端分離」UX が維持される。
   * (do_kan で同名牌 3 枚は副露へ移るため `hand` から消える。嶺上牌だけが after 側に
   *  新規追加されるので diffNewlyAddedTile で取れる。)
   */
  private confirmKan(): void {
    if (!this.bridge) return
    if (this.pendingDecision?.kind !== 'meld-call') return
    if (!this.bridge.canKan(this.humanPlayerIndex)) {
      this.skipMeldCall()
      return
    }
    const beforeHand = this.gameState
      ? this.gameState.players[this.humanPlayerIndex].hand.slice()
      : []
    const ok = this.bridge.doKan(this.humanPlayerIndex)
    this.pendingDecision = null
    if (!ok) {
      this.appendLog('カン宣言失敗')
      this.advanceTurnLoop()
      return
    }
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} がカン`)
    this.selectedHandIndex = null
    this.invalidateCpuTurnTask()
    this.refreshFromBridge()
    const afterHand = this.gameState
      ? this.gameState.players[this.humanPlayerIndex].hand
      : []
    // 嶺上ツモ牌を抽出 (見つからなければ null)。明槓は槓ドラも追加されるが、
    // それは `getDoraIndicators()` 側で表示されるため UI 側で追加処理は不要。
    this.justDrawnTile = diffNewlyAddedTile(beforeHand, afterHand)
  }

  /**
   * チー宣言。pattern (0/1/2) は最初に成立するものを採用 (UI は今回未実装)。
   * 適切なパターン選択 UI は follow-up で対応。
   */
  private confirmChi(): void {
    if (!this.bridge) return
    if (this.pendingDecision?.kind !== 'meld-call') return
    if (!this.bridge.canChi(this.humanPlayerIndex)) {
      this.skipMeldCall()
      return
    }
    let success = false
    for (const pattern of [0, 1, 2]) {
      if (this.bridge.doChi(this.humanPlayerIndex, pattern)) {
        success = true
        break
      }
    }
    this.pendingDecision = null
    if (!success) {
      this.appendLog('チー宣言失敗')
      this.advanceTurnLoop()
      return
    }
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} がチー`)
    this.justDrawnTile = null
    this.selectedHandIndex = null
    this.invalidateCpuTurnTask()
    this.refreshFromBridge()
  }

  /**
   * 自家暗槓を確定する (#46)。
   * Rust 側 `doAnkan` 内で 4 枚副露への移動 + 嶺上ツモ + 槓ドラ追加 + 嶺上開花フラグ。
   * 嶺上ツモを `justDrawnTile` に反映して右端表示する。
   * 暗槓後はそのまま手番に居続けるので advanceTurnLoop は呼ばない。
   */
  private confirmAnkan(tile: Tile): void {
    if (!this.bridge) return
    if (this.pendingDecision?.kind !== 'self-kan-prompt') return
    if (typeof this.bridge.doAnkan !== 'function') {
      this.skipSelfKan()
      return
    }
    const beforeHand = this.gameState
      ? this.gameState.players[this.humanPlayerIndex].hand.slice()
      : []
    const ok = this.bridge.doAnkan(this.humanPlayerIndex, tile)
    this.pendingDecision = null
    if (!ok) {
      this.appendLog('暗槓宣言失敗')
      return
    }
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} が暗槓 ${tileToGlyph(tile)}`)
    this.selectedHandIndex = null
    this.refreshFromBridge()
    const afterHand = this.gameState
      ? this.gameState.players[this.humanPlayerIndex].hand
      : []
    this.justDrawnTile = diffNewlyAddedTile(beforeHand, afterHand)
    // 暗槓後も自家手番継続。ツモ済み (嶺上) 状態なので、続けて打牌待ち or
    // 連続カン (4 枚目を新たに引いた場合) → 再度 maybePromptSelfKan を試す
    if (!this.maybePromptSelfKan()) {
      this.maybePromptRiichi()
    }
    this.renderTable()
  }

  /**
   * 自家加槓を確定する (#46)。
   * Rust 側 2 段階 API:
   *  1. `startShouminkan` で pending_chankan を立て、槍槓ロン候補を取得
   *  2. 候補がいる場合は本実装では「未対応」としてキャンセル + 通常打牌に戻す
   *     (CPU の槍槓ロン宣言は follow-up Issue #50 fast path 範囲外)
   *  3. 候補ゼロなら `completeShouminkan` で完了
   */
  private confirmShouminkan(tile: Tile): void {
    if (!this.bridge) return
    if (this.pendingDecision?.kind !== 'self-kan-prompt') return
    if (typeof this.bridge.startShouminkan !== 'function') {
      this.skipSelfKan()
      return
    }
    const start = this.bridge.startShouminkan(this.humanPlayerIndex, tile)
    this.pendingDecision = null
    if (!start.ok) {
      this.appendLog('加槓宣言失敗')
      return
    }

    // #76: 槍槓候補に CPU がいれば自動ロン宣言
    if (start.candidates.length > 0) {
      const cpuCandidates = start.candidates.filter(idx => idx !== this.humanPlayerIndex)
      if (cpuCandidates.length > 0) {
        // CPU が槍槓ロンを宣言（複数いる場合は先着1人）
        const winnerIdx = cpuCandidates[0]
        const summary = typeof this.bridge.resolveWinChankan === 'function'
          ? this.bridge.resolveWinChankan(winnerIdx, this.humanPlayerIndex)
          : null
        if (summary) {
          this.appendLog(
            `${this.getPlayerName(winnerIdx)} が槍槓ロン ${tileToGlyph(tile)}`
          )
          this.refreshFromBridge()
          this.showRoundResultIfPending()
          return
        }
      }
      // 人間だけが候補 → 後続で pending を立てて選ばせる（現状は見逃し扱い・将来実装）
      this.appendLog(
        `加槓 ${tileToGlyph(tile)}: 人間プレイヤーが槍槓可能（本実装では見逃し扱い）`
      )
    }

    const beforeHand = this.gameState
      ? this.gameState.players[this.humanPlayerIndex].hand.slice()
      : []
    const ok =
      typeof this.bridge.completeShouminkan === 'function'
        ? this.bridge.completeShouminkan(this.humanPlayerIndex, tile)
        : false
    if (!ok) {
      // 完了失敗時は pending_chankan を確実にキャンセルしておく
      if (typeof this.bridge.cancelShouminkan === 'function') {
        this.bridge.cancelShouminkan()
      }
      this.appendLog('加槓完了失敗')
      return
    }
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} が加槓 ${tileToGlyph(tile)}`)
    this.selectedHandIndex = null
    this.refreshFromBridge()
    const afterHand = this.gameState
      ? this.gameState.players[this.humanPlayerIndex].hand
      : []
    this.justDrawnTile = diffNewlyAddedTile(beforeHand, afterHand)
    if (!this.maybePromptSelfKan()) {
      this.maybePromptRiichi()
    }
    this.renderTable()
  }

  /** 自家ツモ後カンモーダルで「カンしない」を選んだ。リーチモーダルへ進める。 */
  private skipSelfKan(): void {
    if (this.pendingDecision?.kind !== 'self-kan-prompt') return
    this.pendingDecision = null
    this.maybePromptRiichi()
    this.renderTable()
  }

  /**
   * 鳴き (ロン/ポン/カン/チー) のどれもしないで通常の turn loop に戻る。
   *
   * Issue #56: ロン可能だったのに見逃した場合は WASM 側にフリテン通知 (`skipRon`) を
   * 送り、同巡フリテン / 立直後永続フリテンを発動させる。`canRon=false` の鳴き見逃し
   * (ポン/カン/チーだけだったケース) ではフリテン化しない。
   */
  private skipMeldCall(): void {
    const wasRonAvailable =
      this.pendingDecision?.kind === 'meld-call' && this.pendingDecision.canRon
    this.pendingDecision = null
    if (wasRonAvailable && this.bridge) {
      this.bridge.skipRon(this.humanPlayerIndex)
    }
    this.appendLog('見逃し')
    this.advanceTurnLoop()
  }

  /**
   * 「リーチ」確認モーダルで「リーチ」を選んだ。次の打牌は立直として処理される。
   * モーダル自体は閉じ、ユーザーは普通に牌選択して打牌する。
   */
  private armRiichi(): void {
    if (this.pendingDecision?.kind !== 'riichi-prompt') return
    this.pendingDecision = null
    this.riichiArmed = true
    this.appendLog('リーチを選択。捨てる牌を選んでください')
    this.renderTable()
  }

  /** 「リーチしない」を選んだ。このターンは再プロンプトしない。 */
  private declineRiichi(): void {
    if (this.pendingDecision?.kind !== 'riichi-prompt') return
    this.pendingDecision = null
    this.riichiDeclinedThisTurn = true
    this.renderTable()
  }

  /**
   * Issue #62: armed リーチ (= モーダルで「リーチ」を選んだが、まだ打牌していない状態)
   * をキャンセルして通常の打牌 UI に戻す。Rust 側はまだ declareRiichi を呼んでいないので
   * TS 側の `riichiArmed` / `selectedHandIndex` を落とすだけで充分。
   * declareRiichi 済み (= 立直成立後) の取り消しは別物で、本メソッドは扱わない。
   */
  private disarmRiichi(): void {
    if (!this.riichiArmed) return
    this.riichiArmed = false
    this.selectedHandIndex = null
    this.appendLog('リーチをキャンセル')
    this.renderTable()
  }

  // ============================================================================
  // キーボードショートカット
  // ============================================================================

  private isHumanTurnInteractive(): boolean {
    return (
      this.bridge !== null &&
      this.bridge.isCurrentPlayerHuman() &&
      !this.bridge.isGameOver() &&
      this.pendingDecision === null &&
      this.activeScene === 'table'
    )
  }

  private handleHotkeySelect(index: number): void {
    if (!this.isHumanTurnInteractive() || !this.gameState) return
    const hand = this.gameState.players[this.humanPlayerIndex].hand
    if (index < 0 || index >= hand.length) return
    this.handleHandTileTap(index)
  }

  private handleHotkeyShift(delta: -1 | 1): void {
    if (!this.isHumanTurnInteractive() || !this.gameState) return
    const hand = this.gameState.players[this.humanPlayerIndex].hand
    if (hand.length === 0) return
    const current = this.selectedHandIndex ?? (delta > 0 ? -1 : hand.length)
    const next = ((current + delta) % hand.length + hand.length) % hand.length
    this.selectedHandIndex = next
    this.renderTable()
  }

  private handleHotkeyDiscard(): void {
    if (!this.isHumanTurnInteractive() || this.selectedHandIndex === null) return
    this.confirmSelectedTile()
  }

  private handleHotkeyTsumo(): void {
    if (this.activeScene !== 'table') return
    if (!this.bridge?.isCurrentPlayerHuman()) return
    // #81: 手動ツモ待ち状態（autoDraw=false かつ手牌がツモ前）
    if (!this.autoDraw && this.gameState) {
      const hand = this.gameState.players[this.humanPlayerIndex].hand
      if (hand.length % 3 === 1) {
        this.doManualDraw()
        return
      }
    }
    if (!this.bridge.canTsumo(this.humanPlayerIndex)) return
    this.confirmTsumo()
  }

  private handleHotkeyRon(): void {
    if (this.pendingDecision?.kind !== 'meld-call') return
    if (!this.pendingDecision.canRon) return
    this.confirmRon(this.pendingDecision.from)
  }

  /**
   * L キー: 状況によって意味が変わる。
   * - リーチ確認モーダル中: 「リーチ」を選ぶ
   * - 通常 + riichiArmed: 「立直して打牌」(selectedHandIndex 必要)
   * - 通常 + canRiichi だがまだ確認中でない: 何もしない (モーダルが出ているはず)
   */
  private handleHotkeyRiichiDiscard(): void {
    if (this.pendingDecision?.kind === 'riichi-prompt') {
      this.armRiichi()
      return
    }
    if (!this.isHumanTurnInteractive() || this.selectedHandIndex === null) return
    if (!this.riichiArmed) return
    this.confirmSelectedTile()
  }

  private handleHotkeyCancel(): void {
    if (this.pendingDecision?.kind === 'meld-call') {
      this.skipMeldCall()
      return
    }
    if (this.pendingDecision?.kind === 'riichi-prompt') {
      this.declineRiichi()
      return
    }
    if (this.pendingDecision?.kind === 'self-kan-prompt') {
      this.skipSelfKan()
      return
    }
    // Issue #62: armed リーチ中なら Esc で armed 状態を解除
    if (this.riichiArmed) {
      this.disarmRiichi()
      return
    }
  }

  private handleHotkeyConfirm(): void {
    // 卓: 「打牌」を確定 (選択中の牌があれば)
    if (this.activeScene === 'table' && this.isHumanTurnInteractive()) {
      this.handleHotkeyDiscard()
    }
  }

  /** #80: S キーで手牌を即時ソートする。 */
  private handleHotkeySortHand(): void {
    if (this.activeScene !== 'table') return
    if (!this.bridge?.isCurrentPlayerHuman()) return
    this.bridge.sortCurrentHand()
    this.refreshFromBridge()
  }

  // ============================================================================
  // 描画
  // ============================================================================

  private renderTable(): void {
    if (!this.gameState) return
    this.activeScene = 'table'
    const isInteractive =
      this.bridge !== null &&
      this.bridge.isCurrentPlayerHuman() &&
      !this.bridge.isGameOver() &&
      this.gameState.currentTurn === this.humanPlayerIndex &&
      this.pendingDecision === null

    const table = createTableScene(this.gameState, {
      humanPlayerIndex: this.humanPlayerIndex,
      selectedHandIndex: this.selectedHandIndex,
      interactiveHandPlayerId: isInteractive ? this.humanPlayerIndex : null,
      justDrawnTile: this.justDrawnTile,
      highlightLastDiscard: this.pendingDecision?.kind === 'meld-call',
      onHandTileTap: index => {
        this.handleHandTileTap(index)
      },
    })
    this.replaceStageRoot(table)
    this.renderHtmlOverlay()
  }

  /**
   * HTML overlay (#ui-side) を最新状態で描画する。
   * 卓以外のシーンでも、最低限「タイトル中」「モード選択中」等のラベルとログを出す。
   */
  private renderHtmlOverlay(): void {
    if (!this.htmlUiRoot) return
    const actions = this.activeScene === 'table' ? this.buildActionButtons() : []
    const hint = this.computeHint()
    const visibleLog = this.eventLog.slice(-EVENT_LOG_VISIBLE_COUNT)
    // #79: デバッグモードのとき CPU 手牌文字列を収集
    const cpuHandStrings: Record<number, string> | undefined =
      this.debugReveal && this.bridge && this.gameState
        ? Object.fromEntries(
            this.gameState.players
              .map((_, i) => [i, this.bridge!.getPlayerHandString(i as PlayerIndex)])
              .filter(([i]) => i !== this.humanPlayerIndex)
          )
        : undefined
    const state: HtmlUiState = {
      game: this.activeScene === 'table' ? this.gameState : null,
      humanPlayerIndex: this.humanPlayerIndex,
      eventLog: visibleLog,
      actions,
      hint,
      debugReveal: this.debugReveal,
      cpuHandStrings,
    }
    renderHtmlUi(this.htmlUiRoot, state)
  }

  private computeHint(): string {
    if (this.pendingDecision?.kind === 'meld-call') {
      const opts: string[] = []
      if (this.pendingDecision.canRon) opts.push('ロン[R]')
      if (this.pendingDecision.canPon) opts.push('ポン[P]')
      if (this.pendingDecision.canKan) opts.push('カン[K]')
      if (this.pendingDecision.canChi) opts.push('チー[C]')
      opts.push('見逃し[Esc]')
      return opts.join(' / ')
    }
    if (this.pendingDecision?.kind === 'riichi-prompt') {
      return 'リーチ[L] / リーチしない[Esc]'
    }
    if (this.pendingDecision?.kind === 'self-kan-prompt') {
      const opts: string[] = []
      if (this.pendingDecision.ankan.length > 0) opts.push('暗槓[K]')
      if (this.pendingDecision.shouminkan.length > 0) {
        opts.push(this.pendingDecision.ankan.length > 0 ? '加槓[M]' : '加槓[K]')
      }
      opts.push('カンしない[Esc]')
      return opts.join(' / ')
    }
    if (this.activeScene !== 'table') {
      if (this.activeScene === 'title') return '対局開始でモード選択へ'
      if (this.activeScene === 'mode-select') return '東風戦/半荘戦を選び「次へ」'
      if (this.activeScene === 'dice-roll') return 'サイコロで起家を決定中'
      if (this.activeScene === 'round-result') return '次局へ / タイトルへ'
      if (this.activeScene === 'result') return '再戦 / タイトルへ'
      return ''
    }
    if (!this.bridge) return ''
    if (this.bridge.isCurrentPlayerHuman()) {
      if (this.riichiArmed) {
        return '立直確定。捨てる牌を選んで「立直して打牌」 / Esc で取り消し'
      }
      if (this.selectedHandIndex === null) {
        return '手牌の数字キー (1-9) か牌をタップで選択。 ←/→ で移動'
      }
      return '同じ牌タップ・D・Enter で打牌。T ツモ'
    }
    return 'CPU の手番'
  }

  private buildGameModes(): GameModeOption[] {
    return [
      {
        key: 'tonpuusen',
        title: '東風戦',
        description: '東場のみの短期戦',
        enabled: true,
      },
      {
        key: 'hanchan',
        title: '半荘戦',
        description: '東南両場を打つ標準ルール',
        enabled: true,
      },
    ]
  }

  private replaceStageRoot(root: Container): void {
    const previousChildren = this.app.stage.removeChildren()
    previousChildren.forEach(child => {
      child.destroy({ children: true })
    })
    this.app.stage.addChild(root)
  }

  private buildResultEntries(gameState: GameState): ResultEntry[] {
    return [...gameState.players]
      .sort((a, b) => b.score - a.score || a.id - b.id)
      .map((player, index) => ({
        rank: index + 1,
        playerId: player.id,
        name: player.name,
        score: player.score,
      }))
  }

  private showResultScene(): void {
    if (!this.gameState || !this.resultMessage) return

    const finalState = this.gameState
    const reason = this.resultMessage
    const entries = this.buildResultEntries(finalState)

    this.invalidateCpuTurnTask()
    this.releaseCurrentBridge()
    this.gameState = finalState
    this.selectedHandIndex = null
    this.activeScene = 'result'

    this.replaceStageRoot(
      createResultScene({
        reason,
        entries,
        detailPlaceholder: '現 API では未取得',
        onRematch: () => {
          this.showModeSelectScene()
        },
        onBackToTitle: () => {
          this.showTitleScene()
        },
      })
    )
    this.renderHtmlOverlay()
  }

  private finalizeGameIfNeeded(): void {
    if (!this.bridge || !this.gameState) return
    if (this.pendingRoundOutcome) return

    if (!this.bridge.isGameOver() && this.bridge.getWallCount() === 0) {
      this.finalizeRoundFromDraw()
      return
    }

    if (!this.bridge.isGameOver()) return

    this.computeAndAppendResultMessage()
    this.showResultScene()
  }

  private computeAndAppendResultMessage(): void {
    if (this.resultMessage) return
    if (!this.gameState) return
    const bankruptPlayer = this.gameState.players.find(player => player.score <= 0)
    if (bankruptPlayer) {
      this.resultMessage = `${bankruptPlayer.name} が飛んで終局`
    } else if (this.bridge && this.bridge.getWallCount() === 0) {
      this.resultMessage = '山牌が尽きて終局'
    } else {
      this.resultMessage = '対局終了'
    }
    this.appendLog(this.resultMessage)
  }

  private showRoundResultIfPending(): void {
    if (!this.bridge) return
    const json = this.bridge.getLastOutcomeJson()
    const outcome = parseRoundOutcome(json)
    if (!outcome) return
    this.pendingRoundOutcome = outcome
    this.invalidateCpuTurnTask()
    this.activeScene = 'round-result'

    if (outcome.kind === 'win') {
      const w = outcome.data
      const winnerName = this.getPlayerName(w.winner)
      const tag =
        w.winType === 'tsumo'
          ? 'ツモ'
          : `ロン (放銃: ${this.getPlayerName((w.from ?? 0) as PlayerIndex)})`
      this.appendLog(`${winnerName} が ${tag} / ${w.han}飜 ${w.fu}符 ${w.totalPoints}点`)
    } else {
      this.appendLog('流局')
    }

    const bridge = this.bridge
    this.replaceStageRoot(
      createRoundResultScene({
        outcome,
        getPlayerName: idx => this.getPlayerName(idx),
        onNext: () => {
          this.advanceToNextRound(bridge)
        },
      })
    )
    this.renderHtmlOverlay()
  }

  private advanceToNextRound(bridge: WasmGameBridge): void {
    if (this.bridge !== bridge) return
    this.pendingRoundOutcome = null
    this.pendingDecision = null
    this.riichiArmed = false
    this.riichiDeclinedThisTurn = false
    this.justDrawnTile = null
    const cont = bridge.nextRound()
    if (!cont) {
      this.refreshFromBridge()
      this.computeAndAppendResultMessage()
      this.showResultScene()
      return
    }
    this.appendLog(`次局: ${bridge.getRound()}局 ${bridge.getHonba()}本場`)
    this.activeScene = 'table'
    this.refreshFromBridge()
    if (this.shouldDrawHumanTile()) {
      this.drawHumanTileAndRefresh()
    }
    this.advanceTurnLoop()
  }
}
