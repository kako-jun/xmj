// シナリオ駆動 UI ドライバ (Issue #66)
//
// 役割:
//   - App を Pixi 無し (`stage` だけのモック Application) で動かす
//   - WasmGameBridge と同じ shape を持つ MockBridge を vitest 環境で組み立てる
//   - 「現在画面に出ているボタンのラベル一覧」「pendingDecision」「log」などを
//     1 メソッドで取れる Snapshot を提供する
//   - 「リーチ」「ロン」「打牌」等のボタンをラベルで叩ける
//
// 設計方針:
//   - 既存 `App.test.ts` の createBridgeMock パターンを抽出して再利用可能にする
//   - 既存 `App.test.ts` を壊さないため、本ファイルは App / WasmGameBridge には
//     一切手を入れない。テスト側から App と DOM だけを介して観測する。
//   - Pixi の `stage` は jsdom 環境でも Container だけは生成できるので、それを
//     fakeApp として渡す。実描画は `App` 内部の `replaceStageRoot` に任せて構わない。
//
// TODO(#66 follow-up):
//   - rust 側の Scenario と TS 側の MockBridge を「同じ局面定義」で組み立てたい場合は、
//     Rust → JSON シリアライズ経由で MockBridge を埋める層を別 Issue で作る。
//     現状は TS 側の MockBridge を直接書く想定。

import { Application, Container } from 'pixi.js'
import { App, type PendingDecision } from './App'
import type { PlayerIndex, RoundWinSummary, Tile } from './types'
import type { WasmGameBridge } from './wasm'

/**
 * `WasmGameBridge` と同じメソッドセットを持つテスト用 mock。
 *
 * 全メソッドにデフォルト挙動を入れ、テスト側は `overrides` で必要な所だけ差し替える。
 * 既存 `App.test.ts` の `createBridgeMock` と同じ思想で、shape は WasmGameBridge と
 * 互換。返り値は `as unknown as WasmGameBridge` で App に渡せる。
 */
export type MockBridge = WasmGameBridge

/**
 * デフォルトの mock 用ゲーム状態文字列。
 * Rust 側 `Game::get_game_state_string()` の出力フォーマットに合わせる
 * (bridgeState.ts の正規表現が読める形)。
 */
const DEFAULT_GAME_STATE_STRING = `Round: 1 | Wall: 69 tiles
Dora indicators: 5p
>親 あなた (25000点): 1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to
  河: 9m 1p
   CPU 南 (25000点): 1p 1p 2p 2p 3p 3p 4s 5s 6s na na ht cn
  河: 7s
   CPU 西 (25000点): 4m 5m 6m 7m 8m 9m 3p 4p 5p 6p 7p 8p pe
   CPU 北 (25000点): 1s 1s 2s 2s 3s 3s 4m 4m 5m 5m 6m 6m sa
`

/**
 * `WasmGameBridge` 互換の mock を作る。テスト側は必要なメソッドだけ overrides で差し替える。
 *
 * 既存 `App.test.ts::createBridgeMock` の export 版。
 */
export const createMockBridge = (
  overrides: Partial<WasmGameBridge> = {}
): WasmGameBridge =>
  ({
    getGameStateJson: () => DEFAULT_GAME_STATE_STRING,
    getPlayerScore: () => 25000,
    getPlayerName: (idx: number) => ['あなた', 'CPU 南', 'CPU 西', 'CPU 北'][idx] ?? `P${idx + 1}`,
    getPlayerDiscards: () => '',
    isPlayerRiichi: () => false,
    getCurrentHandString: () => '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to hk',
    getCurrentPlayerId: () => 0,
    getWallCount: () => 69,
    getDoraIndicators: () => '5p',
    isCurrentPlayerHuman: () => true,
    isCurrentPlayerCpu: () => false,
    isGameOver: () => false,
    drawTile: () => true,
    discardTile: (_tile: Tile) => true,
    executeCpuTurn: () => '5m',
    canRiichi: () => false,
    declareRiichi: () => false,
    canTsumo: () => false,
    canRon: () => false,
    canPon: () => false,
    canKan: () => false,
    canChi: () => false,
    doPon: () => false,
    doKan: () => false,
    doChi: () => false,
    // Issue #46: 暗槓 / 加槓 API。デフォルト候補なし。
    canAnkan: () => [] as Tile[],
    canShouminkan: () => [] as Tile[],
    doAnkan: () => false,
    startShouminkan: () => ({ ok: false, candidates: [] }),
    completeShouminkan: () => false,
    cancelShouminkan: () => undefined,
    resolveWinChankan: () => null as RoundWinSummary | null,
    getLastDiscarder: () => undefined,
    resolveWinTsumo: () => null as RoundWinSummary | null,
    resolveWinRon: () => null as RoundWinSummary | null,
    computeTenpaiPlayers: () => [],
    destroy: () => undefined,
    ...overrides,
  }) as unknown as WasmGameBridge

/**
 * 現在の `App` 状態とその DOM を観測するスナップショット。
 *
 * - `pendingDecision`: モーダル状態 (App のフィールドをそのまま転送)
 * - `log`: 最近のイベントログ (App.eventLog のコピー)
 * - `visibleButtons`: DOM 上で表示されている action button のラベル一覧
 * - `justDrawnTile` / `selectedHandIndex` / `humanHand` / `riichiArmed`: App 内部状態の転写
 */
export interface DriverSnapshot {
  pendingDecision: PendingDecision | null
  log: string[]
  visibleButtons: string[]
  visibleButtonKeys: string[]
  justDrawnTile: Tile | null
  selectedHandIndex: number | null
  humanHand: Tile[]
  riichiArmed: boolean
}

/**
 * `scenarioDriver` の戻り値。テストはここから App / snapshot / クリック動作を扱う。
 */
export interface ScenarioDriver {
  /** ドライブ対象の `App`。直接フィールド読み書きしたいテスト用に公開する。 */
  app: App
  /**
   * `App` が描画している側面の現状スナップショット。
   * 呼ぶたび最新の DOM とフィールドから組み立てるので、操作後に毎回呼び直して構わない。
   */
  snapshot(): DriverSnapshot
  /**
   * 表示中の action button をラベルで探してクリックする。
   * 一致するボタンが無ければ Error。disabled の場合も Error にする方が
   * テスト失敗時の原因特定がしやすいので、disabled だったらその旨をエラーに乗せる。
   */
  clickButton(labelOrKey: string): void
  /** 自家手牌のインデックスを選択する (App.selectedHandIndex に反映)。 */
  selectHandTile(index: number): void
  /** クリーンアップ。jsdom の document.body を空にし、App.destroy() を呼ぶ。 */
  cleanup(): void
}

/**
 * `data-ui="ui-side"` 等の HTML 構造を組み立てる。jsdom 環境前提。
 *
 * 既存 `App.test.ts::UI_SIDE_HTML` の共通化。テストごとに `document.body.innerHTML` を
 * 書き換える運用。
 */
export const setupScenarioDom = (): HTMLElement => {
  document.body.innerHTML = `
    <aside id="ui-side">
      <span data-ui="round"></span>
      <span data-ui="honba"></span>
      <span data-ui="wall"></span>
      <span data-ui="dora"></span>
      <div data-ui="scores"></div>
      <div data-ui="actions"></div>
      <div data-ui="hint"></div>
      <div data-ui="log"></div>
    </aside>
  `
  const root = document.getElementById('ui-side')
  if (!root) throw new Error('ui-side root not found after innerHTML assignment')
  return root
}

/**
 * シナリオドライバを作る。
 *
 * 副作用:
 *   - jsdom の document に `<aside id="ui-side">` を書き込む
 *   - Pixi `Container` を 1 つ生成し、`fakeApp = { stage }` として App に渡す
 *   - 渡された `bridge` で App.startGame を呼び、即「対局中」状態にする
 *
 * @param options.bridge App.startGame に渡す MockBridge
 * @param options.humanPlayerIndex 人間プレイヤーの座席 index (default 0)
 */
export const createScenarioDriver = (options: {
  bridge: MockBridge
  humanPlayerIndex?: PlayerIndex
}): ScenarioDriver => {
  setupScenarioDom()
  const stage = new Container()
  const fakeApp = { stage } as unknown as Application
  const app = new App(fakeApp)

  const humanSeat = options.humanPlayerIndex ?? 0
  app.startGame(options.bridge, humanSeat)

  const findActionButtons = (): HTMLButtonElement[] =>
    Array.from(document.querySelectorAll<HTMLButtonElement>('button[data-action-key]'))

  const snapshot = (): DriverSnapshot => {
    const buttons = findActionButtons()
    const visibleButtons = buttons.map(btn => {
      // 最初の <span> がラベル (htmlUi.ts:166-168 の構造に依存)。
      // ホットキー spans は className='hotkey' なので filter で除外。
      const labelSpan = Array.from(btn.children).find(
        (el): el is HTMLElement =>
          el instanceof HTMLElement && el.className !== 'hotkey'
      )
      return labelSpan?.textContent?.trim() ?? ''
    })
    const visibleButtonKeys = buttons.map(btn => btn.dataset.actionKey ?? '')

    const humanHand = app.gameState?.players[app.humanPlayerIndex].hand ?? []

    return {
      pendingDecision: app.pendingDecision,
      log: [...app.eventLog],
      visibleButtons,
      visibleButtonKeys,
      justDrawnTile: app.justDrawnTile,
      selectedHandIndex: app.selectedHandIndex,
      humanHand: [...humanHand],
      riichiArmed: app.riichiArmed,
    }
  }

  const clickButton = (labelOrKey: string): void => {
    const buttons = findActionButtons()
    const found = buttons.find(btn => {
      if (btn.dataset.actionKey === labelOrKey) return true
      const labelSpan = Array.from(btn.children).find(
        (el): el is HTMLElement =>
          el instanceof HTMLElement && el.className !== 'hotkey'
      )
      return labelSpan?.textContent?.trim() === labelOrKey
    })
    if (!found) {
      const labels = buttons.map(b => `[${b.dataset.actionKey}]`).join(', ')
      throw new Error(`No action button matches "${labelOrKey}". Visible: ${labels || '(none)'}`)
    }
    if (found.disabled) {
      throw new Error(`Action button "${labelOrKey}" is disabled`)
    }
    found.click()
  }

  const selectHandTile = (index: number): void => {
    app.selectedHandIndex = index
    // App.renderTable() を直接呼ばなくても DOM 観測には影響しないが、
    // ボタン enabled 状態を見るには再描画したい。App には public な
    // renderTable は無いので、bridge を介した再 refresh は避けて、
    // 「次の clickButton 前提のテスト」では本関数の後にもう一度
    // pendingDecision を読みなおすこと。
    // 注: 実プロジェクトで selectedHandIndex を経由する操作 (打牌) には、
    // handleHandTileTap 経路を将来追加してよい。
  }

  const cleanup = (): void => {
    app.destroy()
    document.body.innerHTML = ''
  }

  return { app, snapshot, clickButton, selectHandTile, cleanup }
}
