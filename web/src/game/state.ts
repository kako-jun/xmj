// GameState 初期化ユーティリティ (Issue #3)
//
// initWithState は Partial<GameState> を受け取ってデフォルトとマージし、
// 必ず完全な GameState を返す。Wasm 起動直後や、テスト・リスタート時に使う。

import type { GameState, PlayerIndex, PlayerState } from './types'

const DEFAULT_PLAYER_NAMES: readonly [string, string, string, string] = [
  'あなた',
  'CPU 南',
  'CPU 西',
  'CPU 北',
]

export const createEmptyPlayer = (
  id: PlayerIndex,
  name: string,
  isCPU: boolean
): PlayerState => ({
  id,
  name,
  hand: [],
  discards: [],
  score: 25000,
  isCPU,
  isRiichi: false,
})

export const createInitialGameState = (): GameState => ({
  phase: 'title',
  players: [
    createEmptyPlayer(0, DEFAULT_PLAYER_NAMES[0], false),
    createEmptyPlayer(1, DEFAULT_PLAYER_NAMES[1], true),
    createEmptyPlayer(2, DEFAULT_PLAYER_NAMES[2], true),
    createEmptyPlayer(3, DEFAULT_PLAYER_NAMES[3], true),
  ],
  currentTurn: 0,
  wall: [],
  doraIndicators: [],
  round: 1,
})

/**
 * 部分指定された GameState を完全な GameState にマージする。
 * players は配列ごと差し替え (4 要素未満で渡されたらデフォルトで埋め戻す)。
 */
export const initWithState = (partial: Partial<GameState> = {}): GameState => {
  const base = createInitialGameState()
  const merged: GameState = {
    phase: partial.phase ?? base.phase,
    players: base.players,
    currentTurn: partial.currentTurn ?? base.currentTurn,
    wall: partial.wall ?? base.wall,
    doraIndicators: partial.doraIndicators ?? base.doraIndicators,
    round: partial.round ?? base.round,
  }

  if (partial.players) {
    // 4 人未満で渡されたら base から補う。多すぎたら 4 で切る。
    const ps = partial.players
    merged.players = [
      ps[0] ?? base.players[0],
      ps[1] ?? base.players[1],
      ps[2] ?? base.players[2],
      ps[3] ?? base.players[3],
    ]
  }
  return merged
}
