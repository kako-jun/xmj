import type { GameState, PlayerIndex, PlayerState, Tile } from './types'
import { tileFromCuiCode } from './types'
import { WasmGameBridge } from './wasm'

const PLAYER_LINE_RE = /^([ >])(?:(親)\s*)?(.+?) \((\d+)点\):\s*(.*)$/
const ROUND_LINE_RE = /^Round: (\d+) \| Wall: (\d+) tiles$/
const DORA_LINE_RE = /^Dora indicators:\s*(.*)$/

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
  isRiichi: boolean
): PlayerState => ({
  id,
  name,
  hand,
  discards,
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

  for (let i = 2; i < lines.length; i++) {
    const playerLine = lines[i]
    const match = PLAYER_LINE_RE.exec(playerLine)
    if (!match) continue

    const marker = match[1]
    const name = match[3]
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
    round,
  }
}

export const createGameStateFromBridge = (
  bridge: WasmGameBridge,
  humanPlayerIndex: PlayerIndex = 0
): GameState => {
  const base = parseFormattedGameState(bridge.getGameStateJson(), humanPlayerIndex)
  const players = base.players.map((player, idx) => ({
    ...player,
    score: bridge.getPlayerScore(idx),
    name: bridge.getPlayerName(idx),
    discards: parseTileList(bridge.getPlayerDiscards(idx)),
    isRiichi: bridge.isPlayerRiichi(idx),
  })) as GameState['players']

  players[humanPlayerIndex] = {
    ...players[humanPlayerIndex],
    hand: parseTileList(bridge.getCurrentHandString()),
    isCPU: false,
  }

  return {
    ...base,
    currentTurn: bridge.getCurrentPlayerId() as PlayerIndex,
    wall: Array.from({ length: bridge.getWallCount() }, () => ({ suit: 'man', value: 1 })),
    doraIndicators: parseTileList(bridge.getDoraIndicators()),
    players,
  }
}
