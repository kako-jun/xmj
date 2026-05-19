import { describe, expect, it } from 'vitest'
import { createGameStateFromBridge, parseFormattedGameState } from './bridgeState'
import type { Tile } from './types'

const sampleState = `Round: 1 | Wall: 69 tiles
Dora indicators: 5p
>親 あなた (25000点): 1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to hk
  河: 9m 1p
   CPU 南 (25000点): 1p 1p 2p 2p 3p 3p 4s 5s 6s na na ht cn
  河: 7s
   CPU 西 (25000点): 4m 5m 6m 7m 8m 9m 3p 4p 5p 6p 7p 8p pe
   CPU 北 (25000点): 1s 1s 2s 2s 3s 3s 4m 4m 5m 5m 6m 6m sa
`

describe('parseFormattedGameState', () => {
  it('整形文字列から初期卓の GameState を復元する', () => {
    const state = parseFormattedGameState(sampleState, 0)
    expect(state.phase).toBe('game')
    expect(state.round).toBe(1)
    expect(state.currentTurn).toBe(0)
    expect(state.wall).toHaveLength(69)
    expect(state.doraIndicators).toEqual([{ suit: 'pin', value: 5 }])
    expect(state.players[0].hand).toHaveLength(14)
    expect(state.players[0].hand[4]).toEqual({ suit: 'man', value: 5, isRed: true })
    expect(state.players[1].name).toBe('CPU 南')
    expect(state.players[1].hand).toHaveLength(13)
    expect(state.players[1].isCPU).toBe(true)
    expect(state.players[0].discards).toEqual([
      { suit: 'man', value: 9 },
      { suit: 'pin', value: 1 },
    ])
    expect(state.players[1].discards).toEqual([{ suit: 'sou', value: 7 }])
  })

  it('Round 行が欠けた文字列は失敗する', () => {
    const invalidState = sampleState
      .split('\n')
      .filter(line => !line.startsWith('Round: '))
      .join('\n')

    expect(() => parseFormattedGameState(invalidState, 0)).toThrow(/不正なゲーム状態文字列/)
  })

  it('Dora indicators 行が欠けた文字列は失敗する', () => {
    const invalidState = sampleState
      .split('\n')
      .filter(line => !line.startsWith('Dora indicators:'))
      .join('\n')

    expect(() => parseFormattedGameState(invalidState, 0)).toThrow(/不正なゲーム状態文字列/)
  })
})

describe('createGameStateFromBridge', () => {
  it('bridge の getter を優先して卓状態を整える', () => {
    const bridge = {
      getGameStateJson: () => sampleState,
      getPlayerScore: (idx: number) => 25000 + idx * 100,
      getPlayerName: (idx: number) => ['あなた', 'CPU 南', 'CPU 西', 'CPU 北'][idx],
      getPlayerDiscards: (idx: number) => (idx === 0 ? '9m 1p' : ''),
      isPlayerRiichi: (idx: number) => idx === 2,
      getCurrentHandString: () => '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to hk',
      getCurrentPlayerId: () => 0,
      getWallCount: () => 68,
      getDoraIndicators: () => '5p 9s',
    } as const

    const state = createGameStateFromBridge(
      bridge as unknown as import('./wasm').WasmGameBridge,
      0
    )

    expect(state.wall).toHaveLength(68)
    expect(state.players[0].score).toBe(25000)
    expect(state.players[3].score).toBe(25300)
    expect(state.players[0].discards).toEqual([
      { suit: 'man', value: 9 },
      { suit: 'pin', value: 1 },
    ] satisfies Tile[])
    expect(state.players[2].isRiichi).toBe(true)
    expect(state.doraIndicators).toEqual([
      { suit: 'pin', value: 5 },
      { suit: 'sou', value: 9 },
    ])
  })

  it('humanPlayerIndex が手番でない間は getCurrentHandString() で人間席を上書きしない', () => {
    const bridge = {
      getGameStateJson: () => sampleState,
      getPlayerScore: () => 25000,
      getPlayerName: (idx: number) => ['CPU 東', 'あなた', 'CPU 西', 'CPU 北'][idx],
      getPlayerDiscards: () => '',
      isPlayerRiichi: () => false,
      getCurrentHandString: () => '9s 9s 9s 8s 8s 8s 7s 7s 7s 6s 6s 6s ht',
      getCurrentPlayerId: () => 0,
      getWallCount: () => 68,
      getDoraIndicators: () => '5p',
    } as const

    const state = createGameStateFromBridge(
      bridge as unknown as import('./wasm').WasmGameBridge,
      1
    )

    expect(state.currentTurn).toBe(0)
    expect(state.players[1].name).toBe('あなた')
    expect(state.players[1].hand).toEqual([
      { suit: 'pin', value: 1 },
      { suit: 'pin', value: 1 },
      { suit: 'pin', value: 2 },
      { suit: 'pin', value: 2 },
      { suit: 'pin', value: 3 },
      { suit: 'pin', value: 3 },
      { suit: 'sou', value: 4 },
      { suit: 'sou', value: 5 },
      { suit: 'sou', value: 6 },
      { suit: 'wind', value: 2 },
      { suit: 'wind', value: 2 },
      { suit: 'dragon', value: 2 },
      { suit: 'dragon', value: 3 },
    ])
  })

  it('humanPlayerIndex が手番のときだけ getCurrentHandString() で人間席を上書きする', () => {
    const bridge = {
      getGameStateJson: () => sampleState,
      getPlayerScore: () => 25000,
      getPlayerName: (idx: number) => ['CPU 東', 'あなた', 'CPU 西', 'CPU 北'][idx],
      getPlayerDiscards: () => '',
      isPlayerRiichi: () => false,
      getCurrentHandString: () => '9s 9s 9s 8s 8s 8s 7s 7s 7s 6s 6s 6s ht',
      getCurrentPlayerId: () => 1,
      getWallCount: () => 68,
      getDoraIndicators: () => '5p',
    } as const

    const state = createGameStateFromBridge(
      bridge as unknown as import('./wasm').WasmGameBridge,
      1
    )

    expect(state.currentTurn).toBe(1)
    expect(state.players[1].hand).toEqual([
      { suit: 'sou', value: 9 },
      { suit: 'sou', value: 9 },
      { suit: 'sou', value: 9 },
      { suit: 'sou', value: 8 },
      { suit: 'sou', value: 8 },
      { suit: 'sou', value: 8 },
      { suit: 'sou', value: 7 },
      { suit: 'sou', value: 7 },
      { suit: 'sou', value: 7 },
      { suit: 'sou', value: 6 },
      { suit: 'sou', value: 6 },
      { suit: 'sou', value: 6 },
      { suit: 'dragon', value: 2 },
    ])
  })
})
