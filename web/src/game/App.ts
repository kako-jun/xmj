import { Application, Graphics } from 'pixi.js'
import {
  STAGE_WIDTH,
  STAGE_HEIGHT,
  TABLE_BG_COLOR,
  EVENT_LOG_LIMIT,
} from './constants'
import { createTableScene } from './table'
import type { TableActionButton } from './table'
import type { GameState, PlayerIndex, Tile } from './types'
import { createGameStateFromBridge } from './bridgeState'
import { tileToCuiCode } from './types'
import { WasmGameBridge } from './wasm'

interface AppOptions {
  cpuTurnDelayMs?: number
}

export class App {
  app: Application
  bridge: WasmGameBridge | null = null
  humanPlayerIndex: PlayerIndex = 0
  gameState: GameState | null = null
  selectedHandIndex: number | null = null
  eventLog: string[] = []
  resultMessage: string | null = null
  private cpuTurnDelayMs: number
  private cpuTurnTask: Promise<void> | null = null
  private cpuTurnGeneration = 0

  constructor(app: Application, options: AppOptions = {}) {
    this.app = app
    this.cpuTurnDelayMs = options.cpuTurnDelayMs ?? 0
  }

  /**
   * Wasm 卓の生成に失敗したときのフォールバックとして、単色の卓背景だけを描画する。
   */
  showTableBackground(): void {
    const bg = new Graphics()
    bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: TABLE_BG_COLOR })
    this.app.stage.addChild(bg)
  }

  /**
   * bridge を使わず、与えられた GameState をそのまま静的表示する。
   * スモークテストや初期描画確認用であり、ターン進行 UI の起点には使わない。
   */
  showInitialTable(gameState: GameState): void {
    this.invalidateCpuTurnTask()
    this.bridge = null
    this.gameState = gameState
    this.selectedHandIndex = null
    this.eventLog = []
    this.resultMessage = null
    this.renderTable()
  }

  startGame(bridge: WasmGameBridge, humanPlayerIndex: PlayerIndex = 0): void {
    this.invalidateCpuTurnTask()
    this.bridge = bridge
    this.humanPlayerIndex = humanPlayerIndex
    this.selectedHandIndex = null
    this.eventLog = ['対局開始']
    this.resultMessage = null
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

  private isCpuTurnGenerationCurrent(generation: number): boolean {
    return generation === this.cpuTurnGeneration
  }

  private drawHumanTileAndRefresh(): boolean {
    if (!this.bridge) return false
    const drew = this.bridge.drawTile()
    if (!drew) return false
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} がツモ ${this.wallSummary()}`)
    this.refreshFromBridge()
    return true
  }

  private finalizeGameIfNeeded(): void {
    if (!this.bridge || !this.gameState || !this.bridge.isGameOver()) return

    if (this.resultMessage) return

    const bankruptPlayer = this.gameState.players.find(player => player.score <= 0)
    if (bankruptPlayer) {
      this.resultMessage = `${bankruptPlayer.name} が飛んで終局`
    } else if (this.bridge.getWallCount() === 0) {
      this.resultMessage = '山牌が尽きて終局'
    } else {
      this.resultMessage = '対局終了'
    }
    this.appendLog(this.resultMessage)
    this.renderTable()
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

    while (!this.bridge.isGameOver() && this.bridge.isCurrentPlayerCpu()) {
      const currentPlayer = this.bridge.getCurrentPlayerId() as PlayerIndex
      const playerName = this.getPlayerName(currentPlayer)
      const discardedTile = this.bridge.executeCpuTurn()
      this.refreshFromBridge()
      this.appendLog(`${playerName} がツモ ${this.wallSummary()}`)
      this.appendLog(`${playerName} が ${discardedTile} を打牌 ${this.wallSummary()}`)
      this.finalizeGameIfNeeded()
    }

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
      this.appendLog(`${playerName} が思考中`)
      this.renderTable()
      await this.sleep(this.cpuTurnDelayMs)
      if (!this.isCpuTurnGenerationCurrent(generation) || !this.bridge) return

      const discardedTile = this.bridge.executeCpuTurn()
      this.refreshFromBridge()
      if (!this.isCpuTurnGenerationCurrent(generation) || !this.bridge) return
      this.appendLog(`${playerName} がツモ ${this.wallSummary()}`)
      this.appendLog(`${playerName} が ${discardedTile} を打牌 ${this.wallSummary()}`)
      this.finalizeGameIfNeeded()

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

  private buildActionButtons(): TableActionButton[] {
    if (!this.bridge || !this.gameState) return []
    if (this.bridge.isGameOver()) return []

    const shouldUseRiichiConfirm =
      this.bridge.isCurrentPlayerHuman() &&
      this.selectedHandIndex !== null &&
      this.bridge.canRiichi()

    return [
      {
        key: shouldUseRiichiConfirm ? 'riichi-discard' : 'discard',
        label: shouldUseRiichiConfirm ? '立直して打牌' : '打牌',
        enabled: this.bridge.isCurrentPlayerHuman() && this.selectedHandIndex !== null,
        onTap: () => {
          this.confirmSelectedTile({ riichi: shouldUseRiichiConfirm })
        },
      },
    ]
  }

  private renderTable(): void {
    if (!this.gameState) return
    const previousChildren = this.app.stage.removeChildren()
    previousChildren.forEach(child => {
      child.destroy({ children: true })
    })

    const isInteractive =
      this.bridge !== null &&
      this.bridge.isCurrentPlayerHuman() &&
      !this.bridge.isGameOver() &&
      this.gameState.currentTurn === this.humanPlayerIndex

    const table = createTableScene(this.gameState, {
      selectedHandIndex: this.selectedHandIndex,
      interactiveHandPlayerId: isInteractive ? this.humanPlayerIndex : null,
      onHandTileTap: index => {
        this.handleHandTileTap(index)
      },
      actionButtons: this.buildActionButtons(),
      eventLog: this.eventLog,
      resultMessage: this.resultMessage,
    })
    this.app.stage.addChild(table)
  }
}
