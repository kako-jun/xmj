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

/** 対局モード。麻雀の基本ルールに従って 東風戦 / 半荘戦 を選ぶ。 */
export type GameMode = 'tonpuusen' | 'hanchan'

/** モード選択カードの表示用構造。App と modeSelectScene 間で共有する。 */
export interface GameModeOption {
  key: GameMode
  title: string
  description: string
  enabled: boolean
}

/**
 * 場決め (起家決定) のサイコロ結果。
 * 2 個のサイコロを振り、合計 (2-12) から人間プレイヤーの起家を決める。
 * 合計の最小値が 2 なので `(sum - 2) mod 4` で 0-3 に正規化する。
 * 対応表: 2/6/10 → 東(0), 3/7/11 → 南(1), 4/8/12 → 西(2), 5/9 → 北(3)
 */
export interface DiceRoll {
  d1: number
  d2: number
}

export const diceRollToHumanSeat = (roll: DiceRoll): PlayerIndex => {
  const sum = roll.d1 + roll.d2
  return ((sum - 2) % 4) as PlayerIndex
}


/**
 * 副露 (鳴き面子) の種類 (#83 副露表示)。
 * - chi: 順子 (チー)
 * - pon: 刻子 (ポン)
 * - ankan: 暗槓 (自家 4 枚で槓)
 * - minkan: 大明槓 (他家の打牌で槓)
 * - kakan: 加槓 (既存 Pon → Kan に昇格)
 */
export type MeldKind = 'chi' | 'pon' | 'ankan' | 'minkan' | 'kakan'

/**
 * 副露 1 組分の表示用データ (#83 副露表示)。
 * Pixi 側でこのデータをもとに、横並び + claimed 牌を 90 度回転して描画する。
 */
export interface MeldGroup {
  kind: MeldKind
  tiles: Tile[]
  /**
   * 鳴き元の自プレイヤー相対 offset。`(from_player - player_idx + 4) % 4`。
   * - 1: 下家 (右) から
   * - 2: 対面 (上) から
   * - 3: 上家 (左) から
   * - 0: 自家 (通常は加槓のみ、claimed の鳴き元は元 Pon と同じ向き)
   * - null: 暗槓 (鳴き元なし)
   */
  fromOffset: 0 | 1 | 2 | 3 | null
  /** tiles の何番目が他家から取った牌か。暗槓は null。 */
  claimedIndex: number | null
}

/**
 * プレイヤー 1 人の状態。手牌・河・点数を持つ。
 * isCPU で CPU/人間を区別 (Wasm 側の human_player_index に対応)。
 */
export interface PlayerState {
  id: PlayerIndex
  name: string
  hand: Tile[]
  discards: Tile[]
  /** 副露 (鳴き) の面子配列。鳴きが無ければ空 (#83)。 */
  melds: MeldGroup[]
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
  lastDiscard: Tile | null
  /** 何局目か (1: 東1局, 2: 東2局, ...)。 */
  round: number
  /** 本場 (連荘・流局でインクリメント)。Rust core `Game::honba` と一致。 */
  honba: number
  /** 親の座席 index。Rust core `Game::dealer` と一致。 */
  dealer: PlayerIndex
  /** 供託リーチ棒の本数。Rust core `Game::riichi_sticks` と一致。 */
  riichiSticks: number
}

// ============================================================================
// 局結果 (Issue #27 round loop)
// ============================================================================

/**
 * 和了局の結果サマリ。Rust 側 `getLastOutcomeJson()` の `kind=="win"` 形に対応。
 * yaku は現状 Rust 側 `format!("{:?}", Yaku)` の Debug 表記をそのまま受ける
 * （表示用ラベリングは別 Issue でやる）。
 */
export interface RoundWinSummary {
  winner: PlayerIndex
  winType: 'tsumo' | 'ron'
  from?: PlayerIndex
  han: number
  fu: number
  totalPoints: number
  yaku: string[]
}

/** 流局の結果サマリ。聴牌者の座席 index 一覧。 */
export interface RoundDrawSummary {
  tenpaiPlayers: PlayerIndex[]
}

/** #55 特殊（途中）流局の結果。abortiveKind は Rust の AbortiveDrawKind の Debug 文字列。 */
export interface RoundAbortiveSummary {
  abortiveKind: string // "SuufonRenda" | "SuuchaRiichi" | "SuukanSanra" | "KyuushuKyuuhai" | "SanchaaHou"
}

/** 局結果の判別共用体。UI の中間結果シーンが switch で分岐する。 */
export type RoundOutcome =
  | { kind: 'win'; data: RoundWinSummary }
  | { kind: 'draw'; data: RoundDrawSummary }
  | { kind: 'abortive'; data: RoundAbortiveSummary }

/**
 * Rust 側の生 JSON を `RoundOutcome` に変換する。
 * 空文字（未確定）や JSON エラーは null を返す。
 */
export const parseRoundOutcome = (json: string): RoundOutcome | null => {
  if (!json) return null
  let parsed: unknown
  try {
    parsed = JSON.parse(json)
  } catch {
    return null
  }
  if (!parsed || typeof parsed !== 'object') return null
  const obj = parsed as Record<string, unknown>
  if (obj.kind === 'win') {
    const winner = obj.winner as number
    const winType = obj.winType as 'tsumo' | 'ron'
    const from = typeof obj.from === 'number' ? (obj.from as PlayerIndex) : undefined
    const yaku = Array.isArray(obj.yaku) ? (obj.yaku as string[]) : []
    return {
      kind: 'win',
      data: {
        winner: winner as PlayerIndex,
        winType,
        ...(from !== undefined ? { from } : {}),
        han: (obj.han as number) ?? 0,
        fu: (obj.fu as number) ?? 0,
        totalPoints: (obj.totalPoints as number) ?? 0,
        yaku,
      },
    }
  }
  if (obj.kind === 'draw') {
    const arr = Array.isArray(obj.tenpaiPlayers) ? (obj.tenpaiPlayers as number[]) : []
    return {
      kind: 'draw',
      data: { tenpaiPlayers: arr.map(n => n as PlayerIndex) },
    }
  }
  if (obj.kind === 'abortive') {
    // #55 特殊（途中）流局
    return {
      kind: 'abortive',
      data: { abortiveKind: typeof obj.abortiveKind === 'string' ? obj.abortiveKind : 'Abortive' },
    }
  }
  return null
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

/**
 * 牌を Unicode 麻雀牌 (U+1F000-1F02B) 1 文字に変換する。
 * ログ表示・ボタンラベル等の人間可読 UI 文字列で使う。
 *
 * **赤ドラは区別しない** (5m と 5mr どちらも 🀋 = U+1F00B)。ハッシュキー・等価比較には
 * {@link tileToCuiCode} を使うこと。
 *
 * mapping:
 *   - 萬子 1-9 → 🀇🀈🀉🀊🀋🀌🀍🀎🀏 (U+1F007 ..)
 *   - 筒子 1-9 → 🀙🀚🀛🀜🀝🀞🀟🀠🀡 (U+1F019 ..)
 *   - 索子 1-9 → 🀐🀑🀒🀓🀔🀕🀖🀗🀘 (U+1F010 ..)
 *   - 風 東南西北 → 🀀🀁🀂🀃 (U+1F000 ..)
 *   - 三元 白發中 → 🀆🀅🀄 (順序注意: 白=U+1F006, 發=U+1F005, 中=U+1F004)
 */
export const tileToGlyph = (tile: Tile): string => {
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
      const map = [0x1f006, 0x1f005, 0x1f004]
      return cp(map[tile.value - 1] ?? 0x1f004)
    }
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
