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
import { tileToGlyph } from './types'
import { PLAYER_WIND_BY_ID, seatWindForPlayerId } from './seatColors'

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
  /** #79: デバッグモード。true のとき CPU 手牌を表示する。 */
  debugReveal?: boolean
  /** #79: CPU 手牌文字列。playerIndex → 手牌文字列のマップ。 */
  cpuHandStrings?: Record<number, string>
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
  // 場風: round 1-4 = 東場、5-8 = 南場 (半荘のみ後者に到達)。
  // 9 局以上は通常の麻雀ルール外なので "?" でフォールバック (西場/北場まで対応する
  // 拡張ルールは現状未対応 — 想定外データを黙って 南 と表示しない)。
  const bafuu = g.round >= 9 ? '?' : g.round >= 5 ? '南' : '東'
  const localRound = ((g.round - 1) % 4) + 1
  setText(round, `${bafuu}${localRound}局`)
  setText(honba, `${g.honba}本場`)
  setText(wall, `山 ${g.wall.length}`)
  // 親プレイヤー名と場風を表示
  const oya = root.querySelector<HTMLElement>('[data-ui="oya"]')
  if (oya) {
    const oyaPlayer = g.players[g.dealer]
    setText(oya, oyaPlayer ? `親: ${oyaPlayer.name}` : '')
  }
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
      // 風固定の席色 (東=赤 / 南=黄 / 西=青 / 北=緑) を CSS で当てるため、
      // 風を data-wind に出す。data-self は「人間プレイヤーの行」識別用に残すが、
      // 黄色ラインなど色面の装飾には使わない (色は風で固定)。
      if (player.id === state.humanPlayerIndex) row.dataset.self = '1'
      row.dataset.wind = seatWindForPlayerId(player.id)
      const wind = document.createElement('span')
      wind.className = 'wind'
      // CPU 名 ("CPU 東/南/西/北") は既に風が含まれているので .wind を空にして二重表記を回避。
      // 人間プレイヤー (name="あなた" 等) では風を表示する。
      // **構造的判定**: 「CPU + 風漢字」の厳密パターンだけ抑止 (人間が "東田さん" 等の風漢字を含む名前を
      // 付けても風表示が消えないようにする — name.includes(wk) の素朴判定は誤動作する)。
      const wk = PLAYER_WIND[player.id]
      const cpuWindPattern = new RegExp(`^CPU\\s*${wk}$`)
      const isCpuWindName = cpuWindPattern.test(player.name)
      wind.textContent = isCpuWindName ? '' : wk
      const name = document.createElement('span')
      name.className = 'name'
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
  // 旧仕様の #ui-hint は廃止 (ボタンの label と hotkey で十分)
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
// 発信者検出: 各パターンは「話者名にマッチする capturing prefix」をひとまとまりで持つ。
// body は entry からその prefix と、続く「 が 」 (任意) を一気に剥がしたもの。
//
// 色は風固定 (東=赤 / 南=黄 / 西=青 / 北=緑)。「あなた」(人間プレイヤー) は
// 自身の風に応じて spk-east 等を当てる (= 旧 spk-self は廃止、南風と人間が同じ
// 黄色 になるのはむしろ「席色の対応付け」として正しい)。
const FIXED_SPEAKER_PATTERNS: Array<{ tag: string; prefix: RegExp; klass: string }> = [
  { tag: '東家', prefix: /^(?:CPU\s+東|東家)\s*(?:が\s*)?/, klass: 'spk-east' },
  { tag: '南家', prefix: /^(?:CPU\s+南|南家)\s*(?:が\s*)?/, klass: 'spk-south' },
  { tag: '西家', prefix: /^(?:CPU\s+西|西家)\s*(?:が\s*)?/, klass: 'spk-west' },
  { tag: '北家', prefix: /^(?:CPU\s+北|北家)\s*(?:が\s*)?/, klass: 'spk-north' },
]

const speakerPatternsFor = (
  humanIndex: PlayerIndex
): Array<{ tag: string; prefix: RegExp; klass: string }> => {
  // 「あなた」は人間プレイヤーの風由来クラスで色付け (席色を風で固定するため)。
  const humanWind = PLAYER_WIND_BY_ID[humanIndex]
  const humanKlass = `spk-${humanWind}`
  return [
    ...FIXED_SPEAKER_PATTERNS,
    { tag: '自分', prefix: /^あなた\s*(?:が\s*)?/, klass: humanKlass },
  ]
}

const renderLog = (root: HTMLElement, state: HtmlUiState): void => {
  const logEl = root.querySelector<HTMLElement>('[data-ui="log"]')
  if (!logEl) return
  const patterns = speakerPatternsFor(state.humanPlayerIndex)
  logEl.replaceChildren(
    ...state.eventLog.map(entry => {
      const row = document.createElement('div')
      row.className = 'log-entry'
      const matched = patterns.find(p => p.prefix.test(entry))
      if (matched) {
        row.classList.add(matched.klass)
        const tag = document.createElement('span')
        tag.className = 'log-tag'
        tag.textContent = matched.tag
        const body = document.createElement('span')
        body.className = 'log-body'
        const stripped = entry.replace(matched.prefix, '')
        body.textContent = stripped.length > 0 ? stripped : entry
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
  // #79: デバッグモード — CPU 手牌を右サイドバーに表示
  if (state.debugReveal && state.cpuHandStrings) {
    let debugEl = root.querySelector<HTMLElement>('#debug-cpu-hands')
    if (!debugEl) {
      debugEl = document.createElement('div')
      debugEl.id = 'debug-cpu-hands'
      debugEl.style.cssText =
        'margin-top:12px;padding:8px;background:#222;color:#0f0;font-family:monospace;font-size:12px;border-radius:4px;'
      root.appendChild(debugEl)
    }
    const lines = Object.entries(state.cpuHandStrings)
      .map(([idx, hand]) => `P${Number(idx) + 1}: ${hand}`)
      .join('\n')
    if (debugEl.textContent !== lines) debugEl.textContent = lines
  }
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
  | { kind: 'sort-hand' }  // #80: 理牌

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
    case 's':
      return { kind: 'sort-hand' }  // #80: 理牌
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
  /** #80: 理牌 (S キー) */
  onSortHand?: () => void
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
      case 'sort-hand':
        options.onSortHand?.()
        event.preventDefault()
        break
    }
  }
  window.addEventListener('keydown', handler)
  return () => {
    window.removeEventListener('keydown', handler)
  }
}
