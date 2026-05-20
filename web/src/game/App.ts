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
import { diceRollToHumanSeat, tileToCuiCode } from './types'
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

const defaultRollDice = (): DiceRoll => ({
  d1: 1 + Math.floor(Math.random() * 6),
  d2: 1 + Math.floor(Math.random() * 6),
})

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
  pendingRonChance: { from: PlayerIndex } | null = null
  selectedGameMode: GameMode = 'tonpuusen'
  selectedHumanSeat: PlayerIndex | null = null
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
    this.pendingRonChance = null
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
    this.pendingRonChance = null
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
        onSelectMode: mode => {
          this.selectedGameMode = mode
          this.showModeSelectScene()
        },
        onConfirm: () => {
          this.showDiceRollScene()
        },
        onBack: () => {
          this.showTitleScene(this.titleNotice)
        },
      })
    )
    this.renderHtmlOverlay()
  }

  showDiceRollScene(roll?: DiceRoll): void {
    this.invalidateCpuTurnTask()
    this.activeScene = 'dice-roll'
    const settledRoll = roll ?? this.rollDice()
    const humanSeat = diceRollToHumanSeat(settledRoll)
    this.selectedHumanSeat = humanSeat

    this.replaceStageRoot(
      createDiceRollScene({
        roll: settledRoll,
        humanSeat,
        onComplete: () => {
          this.startNewGame()
        },
      })
    )
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
    this.pendingRonChance = null
    this.activeScene = 'table'
    this.refreshFromBridge()

    if (this.shouldDrawHumanTile()) {
      this.drawHumanTileAndRefresh()
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

  private shouldDrawHumanTile(): boolean {
    if (!this.bridge || !this.gameState) return false
    if (!this.bridge.isCurrentPlayerHuman() || this.bridge.isGameOver()) return false
    return this.gameState.players[this.humanPlayerIndex].hand.length % 3 === 1
  }

  private appendLog(message: string): void {
    this.eventLog = [...this.eventLog.slice(-(EVENT_LOG_LIMIT - 1)), message]
  }

  private getPlayerName(playerIndex: PlayerIndex): string {
    return this.bridge?.getPlayerName(playerIndex) ?? `P${playerIndex + 1}`
  }

  private formatTile(tile: Tile): string {
    return tileToCuiCode(tile)
  }

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
    this.pendingRonChance = null
  }

  private isCpuTurnGenerationCurrent(generation: number): boolean {
    return generation === this.cpuTurnGeneration
  }

  private drawHumanTileAndRefresh(): boolean {
    if (!this.bridge) return false
    const drew = this.bridge.drawTile()
    if (!drew) {
      this.maybeFinalizeRoundFromDraw()
      return false
    }
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} がツモ ${this.wallSummary()}`)
    this.refreshFromBridge()
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
    this.bridge.resolveDraw(tenpai)
    this.showRoundResultIfPending()
  }

  private confirmSelectedTile(options: { riichi: boolean } = { riichi: false }): boolean {
    if (!this.bridge || !this.gameState || this.selectedHandIndex === null) return false
    if (!this.bridge.isCurrentPlayerHuman()) return false

    const tile = this.gameState.players[this.humanPlayerIndex].hand[this.selectedHandIndex]
    if (!tile) return false

    if (options.riichi && !this.bridge.declareRiichi()) {
      return false
    }
    if (options.riichi) {
      this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} が立直`)
    }

    const discarded = this.bridge.discardTile(tile)
    if (!discarded) return false

    this.appendLog(
      `${this.getPlayerName(this.humanPlayerIndex)} が ${this.formatTile(tile)} を打牌 ${this.wallSummary()}`
    )
    this.selectedHandIndex = null
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
      this.appendLog(`${playerName} がツモ ${this.wallSummary()}`)
      this.appendLog(`${playerName} が ${discardedTile} を打牌 ${this.wallSummary()}`)
      this.finalizeGameIfNeeded()
      if (this.checkRonChanceAfterDiscard(currentPlayer)) {
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
      this.appendLog(`${playerName} がツモ ${this.wallSummary()}`)
      this.appendLog(`${playerName} が ${discardedTile} を打牌 ${this.wallSummary()}`)
      this.finalizeGameIfNeeded()
      if (this.checkRonChanceAfterDiscard(currentPlayer)) {
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

  private checkRonChanceAfterDiscard(discarder: PlayerIndex): boolean {
    if (!this.bridge) return false
    if (this.pendingRoundOutcome) return false
    if (this.pendingRonChance) return false
    if (discarder === this.humanPlayerIndex) return false
    if (this.bridge.isGameOver()) return false
    if (!this.bridge.canRon(this.humanPlayerIndex)) return false
    this.pendingRonChance = { from: discarder }
    this.invalidateCpuTurnTask()
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} にロン可能`)
    this.renderTable()
    return true
  }

  private handleHandTileTap(index: number): void {
    if (!this.bridge || !this.gameState) return
    if (!this.bridge.isCurrentPlayerHuman()) return

    if (this.selectedHandIndex === index) {
      this.confirmSelectedTile()
      return
    }

    this.selectedHandIndex = index
    this.renderTable()
  }

  /**
   * 「打牌」「ツモ」「ロン」「立直して打牌」「見逃し」など、現在の状況で有効な
   * 行動ボタンを構築する。HTML overlay と内部のキーボードハンドラの両方で使う。
   */
  private buildActionButtons(): HtmlUiActionButton[] {
    if (!this.bridge || !this.gameState) return []
    if (this.bridge.isGameOver()) return []

    if (this.pendingRonChance) {
      const { from } = this.pendingRonChance
      return [
        {
          key: 'ron',
          label: 'ロン',
          enabled: true,
          hotkey: 'R',
          onActivate: () => {
            this.confirmRon(from)
          },
        },
        {
          key: 'ron-skip',
          label: '見逃し',
          enabled: true,
          hotkey: 'Esc',
          onActivate: () => {
            this.skipRon()
          },
        },
      ]
    }

    const isHumanTurn = this.bridge.isCurrentPlayerHuman()
    const canTsumo = isHumanTurn && this.bridge.canTsumo(this.humanPlayerIndex)
    const shouldUseRiichiConfirm =
      isHumanTurn && this.selectedHandIndex !== null && this.bridge.canRiichi()

    const buttons: HtmlUiActionButton[] = [
      {
        key: shouldUseRiichiConfirm ? 'riichi-discard' : 'discard',
        label: shouldUseRiichiConfirm ? '立直して打牌' : '打牌',
        enabled: isHumanTurn && this.selectedHandIndex !== null,
        hotkey: shouldUseRiichiConfirm ? 'L' : 'D',
        onActivate: () => {
          this.confirmSelectedTile({ riichi: shouldUseRiichiConfirm })
        },
      },
    ]

    if (canTsumo) {
      buttons.unshift({
        key: 'tsumo',
        label: 'ツモ',
        enabled: true,
        hotkey: 'T',
        onActivate: () => {
          this.confirmTsumo()
        },
      })
    }

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
    this.showRoundResultIfPending()
  }

  private confirmRon(fromIdx: PlayerIndex): void {
    if (!this.bridge) return
    if (!this.pendingRonChance) return
    if (!this.bridge.canRon(this.humanPlayerIndex)) {
      this.pendingRonChance = null
      this.advanceTurnLoop()
      return
    }
    const summary = this.bridge.resolveWinRon(this.humanPlayerIndex, fromIdx)
    this.pendingRonChance = null
    if (!summary) {
      this.appendLog('ロン宣言失敗（和了形不成立）')
      this.advanceTurnLoop()
      return
    }
    this.showRoundResultIfPending()
  }

  private skipRon(): void {
    this.pendingRonChance = null
    this.appendLog('ロン見逃し')
    this.advanceTurnLoop()
  }

  // ============================================================================
  // キーボードショートカット
  // ============================================================================

  private isHumanTurnInteractive(): boolean {
    return (
      this.bridge !== null &&
      this.bridge.isCurrentPlayerHuman() &&
      !this.bridge.isGameOver() &&
      !this.pendingRonChance &&
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
    if (!this.bridge.canTsumo(this.humanPlayerIndex)) return
    this.confirmTsumo()
  }

  private handleHotkeyRon(): void {
    if (!this.pendingRonChance) return
    this.confirmRon(this.pendingRonChance.from)
  }

  private handleHotkeyRiichiDiscard(): void {
    if (!this.isHumanTurnInteractive() || this.selectedHandIndex === null) return
    if (!this.bridge?.canRiichi()) return
    this.confirmSelectedTile({ riichi: true })
  }

  private handleHotkeyCancel(): void {
    if (this.pendingRonChance) {
      this.skipRon()
    }
  }

  private handleHotkeyConfirm(): void {
    // 卓: 「打牌」を確定 (選択中の牌があれば)
    if (this.activeScene === 'table' && this.isHumanTurnInteractive()) {
      this.handleHotkeyDiscard()
    }
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
      !this.pendingRonChance

    const table = createTableScene(this.gameState, {
      humanPlayerIndex: this.humanPlayerIndex,
      selectedHandIndex: this.selectedHandIndex,
      interactiveHandPlayerId: isInteractive ? this.humanPlayerIndex : null,
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
    const state: HtmlUiState = {
      game: this.activeScene === 'table' ? this.gameState : null,
      humanPlayerIndex: this.humanPlayerIndex,
      eventLog: visibleLog,
      actions,
      hint,
    }
    renderHtmlUi(this.htmlUiRoot, state)
  }

  private computeHint(): string {
    if (this.pendingRonChance) {
      return 'ロン: R / 見逃し: Esc'
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
      if (this.selectedHandIndex === null) {
        return '手牌の数字キー (1-9) か牌をタップで選択。 ←/→ で移動'
      }
      return '同じ牌タップ・D・Enter で打牌。T ツモ / L 立直して打牌'
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
        enabled: false,
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
        onBackToTitle: () => {
          this.showTitleScene()
        },
      })
    )
    this.renderHtmlOverlay()
  }

  private advanceToNextRound(bridge: WasmGameBridge): void {
    if (this.bridge !== bridge) return
    this.pendingRoundOutcome = null
    this.pendingRonChance = null
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
