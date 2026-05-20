import { Application, Container, Graphics } from 'pixi.js'
import {
  STAGE_WIDTH,
  STAGE_HEIGHT,
  TABLE_BG_COLOR,
  EVENT_LOG_LIMIT,
} from './constants'
import { createTableScene } from './table'
import type { TableActionButton } from './table'
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

interface AppOptions {
  cpuTurnDelayMs?: number
  createBridge?: ((humanSeat: PlayerIndex) => WasmGameBridge) | null
  /** 場決め用サイコロの乱数源。テストで決定的に注入できる。 */
  rollDice?: () => DiceRoll
}

const defaultRollDice = (): DiceRoll => ({
  d1: 1 + Math.floor(Math.random() * 6),
  d2: 1 + Math.floor(Math.random() * 6),
})

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
  selectedGameMode: GameMode = 'tonpuusen'
  /** 場決めで決まった人間プレイヤーの席。null なら未確定。 */
  selectedHumanSeat: PlayerIndex | null = null
  private cpuTurnDelayMs: number
  private createBridge: ((humanSeat: PlayerIndex) => WasmGameBridge) | null
  private rollDice: () => DiceRoll
  private cpuTurnTask: Promise<void> | null = null
  private cpuTurnGeneration = 0
  private destroyedBridges = new WeakSet<WasmGameBridge>()

  constructor(app: Application, options: AppOptions = {}) {
    this.app = app
    this.cpuTurnDelayMs = options.cpuTurnDelayMs ?? 0
    this.createBridge = options.createBridge ?? null
    this.rollDice = options.rollDice ?? defaultRollDice
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
    this.releaseCurrentBridge()
    this.gameState = gameState
    this.selectedHandIndex = null
    this.eventLog = []
    this.resultMessage = null
    this.titleNotice = null
    this.pendingRoundOutcome = null
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
    this.replaceStageRoot(
      createTitleScene({
        notice: this.titleNotice,
        startEnabled: this.createBridge !== null,
        onStart: () => {
          this.showModeSelectScene()
        },
      })
    )
  }

  /**
   * モード選択 (東風戦 / 半荘戦) シーン。半荘戦は現状無効。
   */
  showModeSelectScene(): void {
    this.invalidateCpuTurnTask()
    this.replaceStageRoot(
      createModeSelectScene({
        selectedMode: this.selectedGameMode,
        modes: this.buildGameModes(),
        // モード切替で scene を丸ごと再構築している。カードが 2 枚なので毎回 destroy
        // & 再生成しても十分軽い。カードが増える場合は selected フラグだけ差分更新する
        // 設計に切り替えること。
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
  }

  /**
   * 場決めシーン。
   * - 引数なしで呼ぶと内部の rollDice() で席を決める。
   * - テストやアニメ完了後の置き換え用に、明示の DiceRoll を渡せる。
   */
  showDiceRollScene(roll?: DiceRoll): void {
    this.invalidateCpuTurnTask()
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
  }

  private isCpuTurnGenerationCurrent(generation: number): boolean {
    return generation === this.cpuTurnGeneration
  }

  private drawHumanTileAndRefresh(): boolean {
    if (!this.bridge) return false
    const drew = this.bridge.drawTile()
    if (!drew) {
      // 山牌が尽きていれば中間結果シーンに集約する (M2: 競合ガード)。
      this.maybeFinalizeRoundFromDraw()
      return false
    }
    this.appendLog(`${this.getPlayerName(this.humanPlayerIndex)} がツモ ${this.wallSummary()}`)
    this.refreshFromBridge()
    return true
  }

  /**
   * `bridge.drawTile()` が失敗した場面で「山牌切れ & 未確定」を検出したら
   * `finalizeRoundFromDraw` に流す。`pendingRoundOutcome` が既にセットされていたら
   * 何もしない (多重ガード)。
   */
  private maybeFinalizeRoundFromDraw(): void {
    if (!this.bridge) return
    if (this.pendingRoundOutcome) return
    if (this.bridge.isGameOver()) return
    if (this.bridge.getWallCount() !== 0) return
    this.finalizeRoundFromDraw()
  }

  /**
   * 流局 (山牌切れ) の確定処理を 1 箇所に集約する。
   * テンパイ者の自動算出 → `resolveDraw` → 中間結果シーンの流れを担う。
   */
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
    const isInteractive =
      this.bridge !== null &&
      this.bridge.isCurrentPlayerHuman() &&
      !this.bridge.isGameOver() &&
      this.gameState.currentTurn === this.humanPlayerIndex

    const table = createTableScene(this.gameState, {
      humanPlayerIndex: this.humanPlayerIndex,
      selectedHandIndex: this.selectedHandIndex,
      interactiveHandPlayerId: isInteractive ? this.humanPlayerIndex : null,
      onHandTileTap: index => {
        this.handleHandTileTap(index)
      },
      actionButtons: this.buildActionButtons(),
      eventLog: this.eventLog,
    })
    this.replaceStageRoot(table)
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
    // 同点時の正式な順位規則は Rust core 側の API 整備後に再検討する。
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

    this.replaceStageRoot(
      createResultScene({
        reason,
        entries,
        detailPlaceholder: '現 API では未取得',
        onRematch: () => {
          // 再戦はモード選択からやり直す (場決めもサイコロからもう一度)。
          // モードは前回選択を引き継ぐので一度押すだけで再開できる。
          this.showModeSelectScene()
        },
        onBackToTitle: () => {
          this.showTitleScene()
        },
      })
    )
  }

  private finalizeGameIfNeeded(): void {
    if (!this.bridge || !this.gameState) return

    // 中間結果シーンを既に表示している（次局ボタン待ち）ならスキップ
    if (this.pendingRoundOutcome) return

    // 山牌切れだが対局はまだ続く可能性 → resolveDraw → 中間結果シーン
    if (!this.bridge.isGameOver() && this.bridge.getWallCount() === 0) {
      this.finalizeRoundFromDraw()
      return
    }

    if (!this.bridge.isGameOver()) return

    this.computeAndAppendResultMessage()
    this.showResultScene()
  }

  /**
   * 終局時の表示メッセージを `resultMessage` にセットし、eventLog にも積む。
   * 飛び (score <= 0) を最優先、続いて山牌切れ、いずれでもなければ汎用文言。
   * 一度セット済みなら何もしない。
   */
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

  /**
   * `bridge.getLastOutcomeJson()` を読み、結果があれば中間結果シーンを表示する。
   * 結果が読めない / パース失敗のときは何もしない（呼び出し側で finalizeGameIfNeeded
   * 経由のフォールバックが走る）。
   */
  private showRoundResultIfPending(): void {
    if (!this.bridge) return
    const json = this.bridge.getLastOutcomeJson()
    const outcome = parseRoundOutcome(json)
    if (!outcome) return
    this.pendingRoundOutcome = outcome
    this.invalidateCpuTurnTask()

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
  }

  /**
   * 中間結果シーンの「次局へ」処理。
   * `bridge.nextRound()` の戻り値で続行 / 終局を分岐する。
   */
  private advanceToNextRound(bridge: WasmGameBridge): void {
    if (this.bridge !== bridge) return
    this.pendingRoundOutcome = null
    const cont = bridge.nextRound()
    if (!cont) {
      // 対局終了 → 通常の結果画面へ
      // 先に gameState を最新化してから飛び判定/順位確定を行う (S6)。
      this.refreshFromBridge()
      this.computeAndAppendResultMessage()
      this.showResultScene()
      return
    }
    this.appendLog(`次局: ${bridge.getRound()}局 ${bridge.getHonba()}本場`)
    this.refreshFromBridge()
    if (this.shouldDrawHumanTile()) {
      this.drawHumanTileAndRefresh()
    }
    this.advanceTurnLoop()
  }
}
