import type { GameState, MeldGroup, PlayerIndex, PlayerState, Tile } from './types'
import { tileFromCuiCode } from './types'
import { WasmGameBridge } from './wasm'

const PLAYER_LINE_RE = /^([ >])(?:(親)\s*)?(.+?) \((-?\d+)点\):\s*(.*)$/
const ROUND_LINE_RE = /^Round: (\d+) \| Wall: (\d+) tiles$/
const DORA_LINE_RE = /^Dora indicators:\s*(.*)$/
const LAST_DISCARD_LINE_RE = /^Last discard:\s*(.+)$/

const parseTileList = (raw: string): Tile[] =>
  raw
    .trim()
    .split(/\s+/)
    .map(code => tileFromCuiCode(code))
    .filter((tile): tile is Tile => tile !== null)

const createPlayerState = (
  id: PlayerIndex,
  name: string,
  score: number,
  hand: Tile[],
  discards: Tile[],
  isCPU: boolean,
  isRiichi: boolean,
  melds: MeldGroup[] = []
): PlayerState => ({
  id,
  name,
  hand,
  discards,
  melds,
  score,
  isCPU,
  isRiichi,
})

/**
 * Rust 側 Game::get_game_state_string() を UI 用 GameState に変換する。
 * 相手手牌は文字列に含まれる実牌を取り込むが、描画時には裏向きで扱う。
 */
export const parseFormattedGameState = (
  raw: string,
  humanPlayerIndex: PlayerIndex = 0
): GameState => {
  const lines = raw
    .split('\n')
    .map(line => line.trimEnd())
    .filter(Boolean)

  const roundMatch = ROUND_LINE_RE.exec(lines[0] ?? '')
  const doraMatch = DORA_LINE_RE.exec(lines[1] ?? '')
  if (!roundMatch || !doraMatch) {
    throw new Error(`不正なゲーム状態文字列です: ${raw}`)
  }

  const round = Number(roundMatch[1])
  const wallCount = Number(roundMatch[2])
  const doraIndicators = parseTileList(doraMatch[1] ?? '')

  const players: PlayerState[] = []
  let currentTurn: PlayerIndex = 0
  let lastDiscard: Tile | null = null

  for (let i = 2; i < lines.length; i++) {
    const playerLine = lines[i]
    const lastDiscardMatch = LAST_DISCARD_LINE_RE.exec(playerLine)
    if (lastDiscardMatch) {
      lastDiscard = tileFromCuiCode(lastDiscardMatch[1]?.trim() ?? '')
      continue
    }
    const match = PLAYER_LINE_RE.exec(playerLine)
    if (!match) continue

    const marker = match[1]
    const name = match[3].trim()
    const score = Number(match[4])
    const hand = parseTileList(match[5] ?? '')
    const discardsLine = lines[i + 1]?.trim()
    const discards =
      discardsLine && discardsLine.startsWith('河: ')
        ? parseTileList(discardsLine.slice(3))
        : []

    const id = players.length as PlayerIndex
    if (marker === '>') currentTurn = id

    players.push(
      createPlayerState(
        id,
        name,
        score,
        hand,
        discards,
        id !== humanPlayerIndex,
        false
      )
    )

    if (discardsLine?.startsWith('河: ')) i += 1
  }

  if (players.length !== 4) {
    throw new Error(`プレイヤー数の解析に失敗しました: ${players.length}`)
  }

  return {
    phase: 'game',
    players: players as GameState['players'],
    currentTurn,
    wall: Array.from({ length: wallCount }, () => ({ suit: 'man', value: 1 as const })),
    doraIndicators,
    lastDiscard,
    round,
    honba: 0,
    dealer: 0,
    riichiSticks: 0,
  }
}

export const createGameStateFromBridge = (
  bridge: WasmGameBridge,
  humanPlayerIndex: PlayerIndex = 0
): GameState => {
  const base = parseFormattedGameState(bridge.getGameStateJson(), humanPlayerIndex)
  const currentPlayerId = bridge.getCurrentPlayerId() as PlayerIndex
  // #83: 副露を bridge から取得する。getPlayerMelds 未実装の旧 bridge mock では空。
  const getMelds = (idx: number): MeldGroup[] => {
    if (typeof bridge.getPlayerMelds === 'function') {
      return bridge.getPlayerMelds(idx as PlayerIndex)
    }
    return []
  }
  const players = base.players.map((player, idx) => ({
    ...player,
    score: bridge.getPlayerScore(idx),
    name: bridge.getPlayerName(idx),
    discards: parseTileList(bridge.getPlayerDiscards(idx)),
    isRiichi: bridge.isPlayerRiichi(idx),
    melds: getMelds(idx),
  })) as GameState['players']

  if (currentPlayerId === humanPlayerIndex) {
    players[humanPlayerIndex] = {
      ...players[humanPlayerIndex],
      hand: parseTileList(bridge.getCurrentHandString()),
      isCPU: false,
    }
  }

  // round-loop bridge (Issue #27) で追加された getter を取得。
  // 旧 bridge mock (テスト等) では undefined になり得るので default を当てる。
  // テスト互換のためのフォールバックであり、本番 bridge はこれらを必ず実装する。
  const round = typeof bridge.getRound === 'function' ? bridge.getRound() : base.round
  const honba = typeof bridge.getHonba === 'function' ? bridge.getHonba() : 0
  const dealer =
    typeof bridge.getDealer === 'function' ? (bridge.getDealer() as PlayerIndex) : 0
  const riichiSticks =
    typeof bridge.getRiichiSticks === 'function' ? bridge.getRiichiSticks() : 0

  return {
    ...base,
    phase: bridge.isGameOver() ? 'over' : base.phase,
    currentTurn: currentPlayerId,
    wall: Array.from({ length: bridge.getWallCount() }, () => ({ suit: 'man', value: 1 })),
    doraIndicators: parseTileList(bridge.getDoraIndicators()),
    players,
    round,
    honba,
    dealer,
    riichiSticks,
  }
}
