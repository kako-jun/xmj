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

const PLAYER_WIND = ['東', '南', '西', '北'] as const

/**
 * 牌を Unicode 麻雀文字 (U+1F000 群) に変換する。CUI コード "1m" 等を
 * そのまま出すと「m って何？」になるので、ブラウザ版では絵文字牌で見せる。
 *
 * mapping:
 *   - 萬子 1-9 → 🀇🀈🀉🀊🀋🀌🀍🀎🀏 (U+1F007 ..)
 *   - 筒子 1-9 → 🀙🀚🀛🀜🀝🀞🀟🀠🀡 (U+1F019 ..)
 *   - 索子 1-9 → 🀐🀑🀒🀓🀔🀕🀖🀗🀘 (U+1F010 ..)
 *   - 風: 東南西北 → 🀀🀁🀂🀃 (U+1F000 ..)
 *   - 三元: 白發中 → 🀆🀅🀄 (順序注意: 白=U+1F006, 發=U+1F005, 中=U+1F004)
 */
const tileToGlyph = (tile: Tile): string => {
  const cp = (n: number): string => String.fromCodePoint(n)
  switch (tile.suit) {
    case 'man':
      return cp(0x1f007 + (tile.value - 1))
    case 'pin':
      return cp(0x1f019 + (tile.value - 1))
    case 'sou':
      return cp(0x1f010 + (tile.value - 1))
    case 'wind':
      return cp(0x1f000 + (tile.value - 1))
    case 'dragon': {
      // 白=1→🀆 (1F006), 發=2→🀅 (1F005), 中=3→🀄 (1F004)
      const map = [0x1f006, 0x1f005, 0x1f004]
      return cp(map[tile.value - 1] ?? 0x1f004)
    }
  }
}

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

const formatTileText = (tile: Tile): string => tileToGlyph(tile)

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
      // CPU 名 ("CPU 南" 等) には既に風が含まれているため .wind を空にして二重表記を回避。
      // 人間プレイヤー (name="あなた" 等) では風を表示する。
      const wk = PLAYER_WIND[player.id]
      const nameContainsWind = player.name.includes(wk)
      wind.textContent = nameContainsWind ? '' : wk
      const name = document.createElement('span')
      name.className = 'name'
      // 人間でも name が "あなた" のままで十分なので "(あなた)" は付けない。
      name.textContent = player.name
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

// MMORPG のチャット欄風: 行ごとに [発信者] と本文を分けて、発信者ごとに色分け。
// 発信者の検出は行頭の "東家 が" / "あなた が" / "CPU 南 が" などのパターンで行う。
const SPEAKER_PATTERNS: Array<{ tag: string; regex: RegExp; klass: string }> = [
  { tag: '東家', regex: /^(?:CPU\s+東|東家)/, klass: 'spk-east' },
  { tag: '南家', regex: /^(?:CPU\s+南|南家)/, klass: 'spk-south' },
  { tag: '西家', regex: /^(?:CPU\s+西|西家)/, klass: 'spk-west' },
  { tag: '北家', regex: /^(?:CPU\s+北|北家)/, klass: 'spk-north' },
  { tag: '自分', regex: /^あなた/, klass: 'spk-self' },
]

const renderLog = (root: HTMLElement, state: HtmlUiState): void => {
  const logEl = root.querySelector<HTMLElement>('[data-ui="log"]')
  if (!logEl) return
  logEl.replaceChildren(
    ...state.eventLog.map(entry => {
      const row = document.createElement('div')
      row.className = 'log-entry'
      const matched = SPEAKER_PATTERNS.find(p => p.regex.test(entry))
      if (matched) {
        row.classList.add(matched.klass)
        const tag = document.createElement('span')
        tag.className = 'log-tag'
        tag.textContent = matched.tag
        const body = document.createElement('span')
        body.className = 'log-body'
        body.textContent = entry.replace(/^[^\s]+\s*(が)?\s*/, '')
        row.appendChild(tag)
        row.appendChild(body)
      } else {
        row.classList.add('log-system')
        row.textContent = entry
      }
      return row
    })
  )
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
  // 10-12 牌目は Q W E で拾う (R/T はロン/ツモと衝突するため使用不可)
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
    // 直前にボタンをクリックすると focus が残り、続けて `1` / `D` / `T` を押しても
    // 全部無視されてしまうので、Enter / Space (= 'confirm' or 'discard' で Space を拾う)
    // だけブラウザ既定の発火に任せ、それ以外の意味論キーは focus 場所に関わらず通す。
    const target = event.target as HTMLElement | null
    const isButtonFocused = target?.tagName === 'BUTTON'
    const isBrowserDefaultKey =
      (event.key === 'Enter' || event.key === ' ' || event.key === 'Spacebar')
    if (isButtonFocused && isBrowserDefaultKey) {
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
