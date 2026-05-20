// HTML オーバーレイ UI。
//
// 役割:
//   - Pixi の卓 (table.ts) には文字を一切出さず、ここで点数・局・本場・山・ドラ・
//     操作ボタン・実況ログをすべて HTML として描く。スマホ縦は卓下、PC 横は卓右に
//     回り込むレイアウトは index.html の CSS Grid 側が担う。
//   - キーボードショートカット (1-9 = 手牌選択 / D = 打牌 / T = ツモ / R = ロン /
//     L = 立直して打牌 / Esc = 見逃し or 戻る / Enter = メイン操作) も統合する。
//     キー名は CLI / TUI と揃え、画面を見ずに同じ操作で遊べることを目指す。
//
// テスト方針: jsdom 環境で `renderHtmlUi` を呼び、data-ui="xxx" の DOM 状態と
// onClick / keydown の発火を検証する (htmlUi.test.ts)。

import type { GameState, PlayerIndex, PlayerState, Tile } from './types'
import { tileToCuiCode } from './types'

const PLAYER_WIND = ['東', '南', '西', '北'] as const

export interface HtmlUiActionButton {
  /** ボタン識別子。'discard' | 'tsumo' | 'ron' | 'ron-skip' | 'riichi-discard' 等 */
  key: string
  /** 画面に出すラベル ('打牌' / 'ツモ' / 'ロン' / '立直して打牌' / '見逃し') */
  label: string
  /** 有効/無効 (disabled 表示) */
  enabled: boolean
  /** クリック / キーボードショートカット押下時のハンドラ */
  onActivate: () => void
  /** 表示する hotkey ヒント (例: 'D'、'T'、'R'、'L'、'Esc') */
  hotkey?: string
}

export interface HtmlUiState {
  /** 局情報。null なら卓画面以外のシーン (タイトル等) として全部空表示にする */
  game: GameState | null
  /** 人間プレイヤーの座席 index。score 表で「あなた」を強調するため。 */
  humanPlayerIndex: PlayerIndex
  /** 実況ログ。最新が末尾。 */
  eventLog: string[]
  /** 行動ボタン。空配列なら「選択肢なし」とだけ表示。 */
  actions: HtmlUiActionButton[]
  /** ヒント表示 (例: 「手牌の数字キーで牌を選び、D で打牌」)。 */
  hint?: string | null
}

const formatTileText = (tile: Tile): string => tileToCuiCode(tile)

const setText = (el: HTMLElement | null, text: string): void => {
  if (!el) return
  if (el.textContent !== text) {
    el.textContent = text
  }
}

const renderStatus = (root: HTMLElement, state: HtmlUiState): void => {
  const round = root.querySelector<HTMLElement>('[data-ui="round"]')
  const honba = root.querySelector<HTMLElement>('[data-ui="honba"]')
  const wall = root.querySelector<HTMLElement>('[data-ui="wall"]')
  const dora = root.querySelector<HTMLElement>('[data-ui="dora"]')
  const g = state.game
  if (!g) {
    setText(round, '—')
    setText(honba, '')
    setText(wall, '')
    if (dora) dora.replaceChildren()
    return
  }
  setText(round, `東${g.round}局`)
  setText(honba, g.honba > 0 ? `${g.honba}本場` : '')
  setText(wall, `山 ${g.wall.length}`)
  if (dora) {
    dora.replaceChildren(
      ...g.doraIndicators.map(tile => {
        const span = document.createElement('span')
        span.className = 'dora-tile'
        span.textContent = formatTileText(tile)
        return span
      })
    )
  }
}

const renderScores = (root: HTMLElement, state: HtmlUiState): void => {
  const scoresEl = root.querySelector<HTMLElement>('[data-ui="scores"]')
  if (!scoresEl) return
  const g = state.game
  if (!g) {
    scoresEl.replaceChildren()
    return
  }
  scoresEl.replaceChildren(
    ...g.players.map((player: PlayerState) => {
      const row = document.createElement('div')
      row.className = 'score-row'
      if (g.currentTurn === player.id) row.classList.add('is-turn')
      if (player.score <= 0) row.classList.add('is-bankrupt')
      if (player.id === state.humanPlayerIndex) row.dataset.self = '1'
      const wind = document.createElement('span')
      wind.className = 'wind'
      wind.textContent = PLAYER_WIND[player.id]
      const name = document.createElement('span')
      name.className = 'name'
      name.textContent =
        player.id === state.humanPlayerIndex ? `${player.name} (あなた)` : player.name
      const pts = document.createElement('span')
      pts.className = 'points'
      pts.textContent = player.score.toLocaleString()
      row.appendChild(wind)
      row.appendChild(name)
      row.appendChild(pts)
      if (player.isRiichi) {
        const r = document.createElement('span')
        r.className = 'riichi'
        r.textContent = '立直'
        row.appendChild(r)
      }
      return row
    })
  )
}

const renderActions = (root: HTMLElement, state: HtmlUiState): void => {
  const actionsEl = root.querySelector<HTMLElement>('[data-ui="actions"]')
  if (!actionsEl) return
  const hintEl = root.querySelector<HTMLElement>('[data-ui="hint"]')
  setText(hintEl, state.hint ?? '')
  if (state.actions.length === 0) {
    actionsEl.replaceChildren()
    return
  }
  actionsEl.replaceChildren(
    ...state.actions.map(action => {
      const btn = document.createElement('button')
      btn.type = 'button'
      btn.className = 'action-btn'
      btn.dataset.actionKey = action.key
      btn.disabled = !action.enabled
      const labelSpan = document.createElement('span')
      labelSpan.textContent = action.label
      btn.appendChild(labelSpan)
      if (action.hotkey) {
        const hk = document.createElement('span')
        hk.className = 'hotkey'
        hk.textContent = `[${action.hotkey}]`
        btn.appendChild(hk)
      }
      btn.addEventListener('click', () => {
        if (!action.enabled) return
        action.onActivate()
      })
      return btn
    })
  )
}

const renderLog = (root: HTMLElement, state: HtmlUiState): void => {
  const logEl = root.querySelector<HTMLElement>('[data-ui="log"]')
  if (!logEl) return
  logEl.replaceChildren(
    ...state.eventLog.map(entry => {
      const row = document.createElement('div')
      row.className = 'log-entry'
      row.textContent = entry
      return row
    })
  )
  // 最新行が常に見えるよう下端に自動スクロール
  logEl.scrollTop = logEl.scrollHeight
}

/**
 * HTML オーバーレイの全部分を再描画する。
 * 呼び出し側は state が変わるたび毎回呼んで構わない (差分更新は textContent 比較で軽量化)。
 */
export const renderHtmlUi = (root: HTMLElement, state: HtmlUiState): void => {
  renderStatus(root, state)
  renderScores(root, state)
  renderActions(root, state)
  renderLog(root, state)
}

/**
 * キーボードショートカット用の意味論キー。
 *   - 'select-1' .. 'select-9' (1-9 数字キー → 自家手牌 1-indexed の選択)
 *   - 'select-10' .. 'select-14' (Q W E R T → 10-14 番目の選択用)
 *   - 'discard' (D / Space)
 *   - 'tsumo' (T)
 *   - 'ron' (R)
 *   - 'riichi-discard' (L)
 *   - 'cancel' (Esc / 見逃し)
 *   - 'confirm' (Enter / 主操作)
 *   - 'back-tile' (左矢印 / B → 牌選択を1つ戻す)
 *   - 'next-tile' (右矢印 / N → 牌選択を1つ進める)
 */
export type HotkeyIntent =
  | { kind: 'select'; index: number }
  | { kind: 'discard' }
  | { kind: 'tsumo' }
  | { kind: 'ron' }
  | { kind: 'riichi-discard' }
  | { kind: 'cancel' }
  | { kind: 'confirm' }
  | { kind: 'back-tile' }
  | { kind: 'next-tile' }

/**
 * KeyboardEvent を HotkeyIntent に変換する。
 * 修飾キー (Ctrl/Meta/Alt) が押されている場合は意味を持たせない (ブラウザ操作優先)。
 * IME 入力中 (event.isComposing) も無視する。
 */
export const keyEventToIntent = (event: KeyboardEvent): HotkeyIntent | null => {
  if (event.ctrlKey || event.metaKey || event.altKey) return null
  if (event.isComposing) return null
  const key = event.key
  // 数字 1-9: 1-indexed の手牌選択
  if (/^[1-9]$/.test(key)) return { kind: 'select', index: Number(key) - 1 }
  // 14 牌目までは QWERT で拾う
  const extraSelect: Record<string, number> = { q: 9, w: 10, e: 11 }
  const lower = key.toLowerCase()
  if (extraSelect[lower] !== undefined) return { kind: 'select', index: extraSelect[lower] }
  switch (lower) {
    case 'd':
    case ' ':
      return { kind: 'discard' }
    case 't':
      return { kind: 'tsumo' }
    case 'r':
      return { kind: 'ron' }
    case 'l':
      return { kind: 'riichi-discard' }
    case 'enter':
      return { kind: 'confirm' }
    case 'escape':
    case 'esc':
      return { kind: 'cancel' }
    case 'arrowleft':
    case 'b':
      return { kind: 'back-tile' }
    case 'arrowright':
    case 'n':
      return { kind: 'next-tile' }
    default:
      return null
  }
}

export interface KeyboardBindingOptions {
  /**
   * 1-indexed 手牌位置の選択を試みる。範囲外なら無視。
   * 同じ index が既に選択済みなら確定打牌として扱うかは呼び出し側の自由。
   */
  onSelect?: (index: number) => void
  /** 「打牌」操作 (D / Space) */
  onDiscard?: () => void
  onTsumo?: () => void
  onRon?: () => void
  onRiichiDiscard?: () => void
  onCancel?: () => void
  onConfirm?: () => void
  onBackTile?: () => void
  onNextTile?: () => void
}

/**
 * window レベルで keydown を捕まえ、ショートカット → ハンドラに振り分ける。
 * 戻り値の関数を呼ぶと解除できる。
 */
export const installKeyboardShortcuts = (options: KeyboardBindingOptions): (() => void) => {
  const handler = (event: KeyboardEvent): void => {
    const intent = keyEventToIntent(event)
    if (!intent) return
    // ボタン上で Enter / Space を押した場合はブラウザのデフォルト発火に任せる。
    const target = event.target as HTMLElement | null
    if (target && target.tagName === 'BUTTON') {
      return
    }
    switch (intent.kind) {
      case 'select':
        options.onSelect?.(intent.index)
        event.preventDefault()
        break
      case 'discard':
        options.onDiscard?.()
        event.preventDefault()
        break
      case 'tsumo':
        options.onTsumo?.()
        event.preventDefault()
        break
      case 'ron':
        options.onRon?.()
        event.preventDefault()
        break
      case 'riichi-discard':
        options.onRiichiDiscard?.()
        event.preventDefault()
        break
      case 'cancel':
        options.onCancel?.()
        event.preventDefault()
        break
      case 'confirm':
        options.onConfirm?.()
        event.preventDefault()
        break
      case 'back-tile':
        options.onBackTile?.()
        event.preventDefault()
        break
      case 'next-tile':
        options.onNextTile?.()
        event.preventDefault()
        break
    }
  }
  window.addEventListener('keydown', handler)
  return () => {
    window.removeEventListener('keydown', handler)
  }
}
