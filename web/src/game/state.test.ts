// initWithState の挙動確認 (Issue #3)

import { describe, it, expect } from 'vitest'
import { initWithState, createInitialGameState, createEmptyPlayer } from './state'
import type { Tile } from './types'

describe('createInitialGameState', () => {
  it('4 人プレイヤー、25000 点持ち、東 1 局でタイトル状態', () => {
    const s = createInitialGameState()
    expect(s.phase).toBe('title')
    expect(s.round).toBe(1)
    expect(s.currentTurn).toBe(0)
    expect(s.players).toHaveLength(4)
    expect(s.players[0].name).toBe('あなた')
    expect(s.players[0].isCPU).toBe(false)
    expect(s.players[1].isCPU).toBe(true)
    expect(s.players[2].isCPU).toBe(true)
    expect(s.players[3].isCPU).toBe(true)
    s.players.forEach(p => expect(p.score).toBe(25000))
  })
})

describe('initWithState', () => {
  it('引数なしでデフォルト GameState を返す', () => {
    const s = initWithState()
    expect(s.phase).toBe('title')
    expect(s.players).toHaveLength(4)
  })

  it('phase だけ部分指定すると他はデフォルトのまま', () => {
    const s = initWithState({ phase: 'game' })
    expect(s.phase).toBe('game')
    expect(s.round).toBe(1)
    expect(s.currentTurn).toBe(0)
  })

  it('round と currentTurn を上書きできる', () => {
    const s = initWithState({ round: 3, currentTurn: 2 })
    expect(s.round).toBe(3)
    expect(s.currentTurn).toBe(2)
  })

  it('players をフルに渡せば差し替わる', () => {
    const custom = [
      createEmptyPlayer(0, 'A', false),
      createEmptyPlayer(1, 'B', true),
      createEmptyPlayer(2, 'C', true),
      createEmptyPlayer(3, 'D', true),
    ]
    const s = initWithState({ players: custom as typeof s.players })
    expect(s.players[0].name).toBe('A')
    expect(s.players[3].name).toBe('D')
  })

  it('wall と doraIndicators を上書きできる', () => {
    const dora: Tile[] = [{ suit: 'man', value: 5 }]
    const wall: Tile[] = Array.from({ length: 70 }, (_, i) => ({
      suit: 'pin' as const,
      value: (i % 9) + 1,
    }))
    const s = initWithState({ wall, doraIndicators: dora })
    expect(s.wall).toHaveLength(70)
    expect(s.doraIndicators).toEqual(dora)
  })
})
