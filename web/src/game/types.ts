// 麻雀ドメイン型 (Issue #3)
//
// 設計方針:
//   - Rust 側 (`src/tile.rs` / `src/game.rs` / `src/player.rs`) の純粋データを
//     TS 側で写経した形。Wasm から取れる文字列・JSON との変換は state.ts /
//     wasm.ts でまとめる。
//   - Phase '誠京/鷲巣/闇麻' 等の特殊ルールは後続 Issue で拡張する。
//     ここでは「通常麻雀」を回す最小集合に絞る。

/** 牌の種類。数牌3系統 (萬・筒・索) + 字牌 2 系統 (風・三元)。 */
export type Suit = 'man' | 'pin' | 'sou' | 'wind' | 'dragon'

/**
 * 牌 1 枚。value の意味は suit ごとに異なる:
 *   - 'man' / 'pin' / 'sou': 1-9 (数牌)
 *   - 'wind':   1=東 / 2=南 / 3=西 / 4=北
 *   - 'dragon': 1=白 / 2=發 / 3=中
 * isRed は 5m/5p/5s に対応する赤ドラフラグ。
 */
export interface Tile {
  suit: Suit
  value: number
  isRed?: boolean
}

/** ゲームのフェーズ。タイトル / 対局中 / 結果。 */
export type GamePhase = 'title' | 'game' | 'over'

/** プレイヤー位置 (東家=0, 南=1, 西=2, 北=3)。 */
export type PlayerIndex = 0 | 1 | 2 | 3

/**
 * プレイヤー 1 人の状態。手牌・河・点数を持つ。
 * isCPU で CPU/人間を区別 (Wasm 側の human_player_index に対応)。
 */
export interface PlayerState {
  id: PlayerIndex
  name: string
  hand: Tile[]
  discards: Tile[]
  score: number
  isCPU: boolean
  isRiichi: boolean
}

/**
 * 局全体の状態。Wasm の WasmGame と 1:1 対応するように設計し、
 * UI 側はこれを single source of truth として扱う。
 */
export interface GameState {
  phase: GamePhase
  players: [PlayerState, PlayerState, PlayerState, PlayerState]
  currentTurn: PlayerIndex
  /** 山牌の残り。Tile[] 自体は UI でほぼ使わないが残数表示のために長さだけ参照する。 */
  wall: Tile[]
  doraIndicators: Tile[]
  /** 何局目か (1: 東1局, 2: 東2局, ...)。 */
  round: number
}

// ============================================================================
// 牌の文字列表現 (Rust 側 tile.rs と合わせる)
// ============================================================================

/**
 * Rust 側 Tile::to_string() / Tile::from_string() に対応する CUI 表記。
 *   - 数牌: "1m" "5p" "9s" (赤ドラは "5mr")
 *   - 字牌: "to"(東) "na"(南) "sa"(西) "pe"(北) "hk"(白) "ht"(發) "cn"(中)
 */
export const WIND_CODES = ['to', 'na', 'sa', 'pe'] as const
export const DRAGON_CODES = ['hk', 'ht', 'cn'] as const

export const tileToCuiCode = (tile: Tile): string => {
  switch (tile.suit) {
    case 'man':
    case 'pin':
    case 'sou': {
      const suitChar = tile.suit === 'man' ? 'm' : tile.suit === 'pin' ? 'p' : 's'
      return `${tile.value}${suitChar}${tile.isRed ? 'r' : ''}`
    }
    case 'wind':
      return WIND_CODES[tile.value - 1] ?? '?'
    case 'dragon':
      return DRAGON_CODES[tile.value - 1] ?? '?'
  }
}

export const tileFromCuiCode = (code: string): Tile | null => {
  // 数牌
  const numMatch = /^([1-9])([mps])(r?)$/.exec(code)
  if (numMatch) {
    const [, value, suitChar, red] = numMatch
    const suit: Suit = suitChar === 'm' ? 'man' : suitChar === 'p' ? 'pin' : 'sou'
    const tile: Tile = { suit, value: Number(value) }
    if (red) tile.isRed = true
    return tile
  }
  const windIdx = WIND_CODES.indexOf(code as (typeof WIND_CODES)[number])
  if (windIdx >= 0) return { suit: 'wind', value: windIdx + 1 }
  const dragonIdx = DRAGON_CODES.indexOf(code as (typeof DRAGON_CODES)[number])
  if (dragonIdx >= 0) return { suit: 'dragon', value: dragonIdx + 1 }
  return null
}
