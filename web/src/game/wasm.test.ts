// Wasm ラッパのテスト (Issue #3)
//
// 実際の wasm-bindgen 生成物は jsdom で動かしづらいため、
// vi.mock で pkg を差し替えて wrapper の呼び出しを確認する。

import { describe, it, expect, vi, beforeEach } from 'vitest'
import {
  WasmGameBridge,
  __resetWasmForTest,
  __setWasmModuleForTest,
} from './wasm'
import { SHIBARI_BLOCKED } from './types'
import type { Tile } from './types'

// ---- モック WasmGame ----
class MockWasmGame {
  static newHybridArgs: [string, number] | null = null
  static constructorArgs: string[] | null = null
  /** 最後に discardTile に渡された CUI コード。WasmGameBridge.discardTile の検証用。 */
  static lastDiscardArg: string | null = null
  /** resolveDraw に渡された Uint32Array の中身 (テスト検証用) */
  static lastResolveDrawArg: number[] | null = null
  /** resolveWinTsumo に渡された winner_idx */
  static lastResolveTsumoArg: number | null = null
  /** resolveWinRon に渡された [winner_idx, from_idx] */
  static lastResolveRonArgs: [number, number] | null = null
  /** canTsumo / canRon に渡された player_idx */
  static lastCanTsumoArg: number | null = null
  static lastCanRonArg: number | null = null

  drawCalled = 0

  static newHybrid(name: string, position: number): MockWasmGame {
    MockWasmGame.newHybridArgs = [name, position]
    return new MockWasmGame()
  }

  constructor(names?: string[]) {
    if (names) MockWasmGame.constructorArgs = names
  }

  drawTile(): boolean {
    this.drawCalled += 1
    return true
  }
  discardTile(code: string): boolean {
    MockWasmGame.lastDiscardArg = code
    return true
  }
  executeCpuTurn(): string {
    return '5m'
  }
  getGameState(): string {
    return '{"phase":"game"}'
  }
  getCurrentHand(): string {
    return '1m 2m 3m'
  }
  getCurrentPlayerId(): number {
    return 1
  }
  isCurrentPlayerHuman(): boolean {
    return false
  }
  isCurrentPlayerCpu(): boolean {
    return true
  }
  isGameOver(): boolean {
    return false
  }
  getWallCount(): number {
    return 70
  }
  getShanten(): number {
    return 2
  }
  getPlayerScore(_idx: number): number {
    return 25000
  }
  getPlayerName(_idx: number): string {
    return 'mock'
  }
  getDoraIndicators(): string {
    return '5m'
  }
  getPlayerDiscards(_idx: number): string {
    return ''
  }
  canRiichi(): boolean {
    return false
  }
  declareRiichi(): boolean {
    return false
  }
  isPlayerRiichi(_idx: number): boolean {
    return false
  }
  canTsumo(idx: number): boolean {
    MockWasmGame.lastCanTsumoArg = idx
    return idx === 0
  }
  canRon(idx: number): boolean {
    MockWasmGame.lastCanRonArg = idx
    return idx === 2
  }
  getLastDiscarder(): number | undefined {
    return 3
  }
  free(): void {
    /* noop */
  }

  // ---- Round loop (Issue #27) ----
  resolveDraw(arr: Uint32Array): void {
    MockWasmGame.lastResolveDrawArg = Array.from(arr)
  }
  resolveWinTsumo(winnerIdx: number): string {
    MockWasmGame.lastResolveTsumoArg = winnerIdx
    return JSON.stringify({
      han: 3,
      fu: 30,
      totalPoints: 3900,
      yaku: ['Riichi', 'Pinfu', 'Tsumo'],
    })
  }
  resolveWinRon(winnerIdx: number, fromIdx: number): string {
    MockWasmGame.lastResolveRonArgs = [winnerIdx, fromIdx]
    return JSON.stringify({
      han: 2,
      fu: 40,
      totalPoints: 2600,
      yaku: ['Pinfu', 'Tanyao'],
    })
  }
  nextRound(): boolean {
    return true
  }
  getRound(): number {
    return 2
  }
  getHonba(): number {
    return 1
  }
  getDealer(): number {
    return 0
  }
  getRiichiSticks(): number {
    return 1
  }
  getLastOutcomeJson(): string {
    return JSON.stringify({
      kind: 'win',
      winner: 0,
      winType: 'tsumo',
      han: 3,
      fu: 30,
      totalPoints: 3900,
      yaku: ['Riichi'],
    })
  }
}

const fakeModule = {
  default: vi.fn(async () => undefined),
  WasmGame: MockWasmGame as unknown as typeof import('../../pkg/xmj_core.js').WasmGame,
  version: () => '0.1.0',
  gameName: () => '邪雀 Xtreme Mahjong',
} as unknown as typeof import('../../pkg/xmj_core.js')

describe('WasmGameBridge', () => {
  beforeEach(() => {
    __resetWasmForTest()
    MockWasmGame.newHybridArgs = null
    MockWasmGame.constructorArgs = null
    MockWasmGame.lastDiscardArg = null
    MockWasmGame.lastResolveDrawArg = null
    MockWasmGame.lastResolveTsumoArg = null
    MockWasmGame.lastResolveRonArgs = null
    MockWasmGame.lastCanTsumoArg = null
    MockWasmGame.lastCanRonArg = null
    __setWasmModuleForTest(fakeModule)
  })

  it('createHybrid で WasmGame.newHybrid に引数が渡る', () => {
    const bridge = WasmGameBridge.createHybrid('かこじゅん', 0)
    expect(MockWasmGame.newHybridArgs).toEqual(['かこじゅん', 0])
    expect(bridge.getCurrentPlayerId()).toBe(1)
  })

  it('createAllHuman で名前配列が渡る', () => {
    WasmGameBridge.createAllHuman(['A', 'B', 'C', 'D'])
    expect(MockWasmGame.constructorArgs).toEqual(['A', 'B', 'C', 'D'])
  })

  it('discardTile は Tile を CUI 文字列に変換して渡す', () => {
    const bridge = WasmGameBridge.createHybrid('me', 0)

    // 赤ドラの 5m は "5mr"
    const red: Tile = { suit: 'man', value: 5, isRed: true }
    expect(bridge.discardTile(red)).toBe(true)
    expect(MockWasmGame.lastDiscardArg).toBe('5mr')

    // 風牌 (東) は "to"
    bridge.discardTile({ suit: 'wind', value: 1 })
    expect(MockWasmGame.lastDiscardArg).toBe('to')

    // 三元 (中) は "cn"
    bridge.discardTile({ suit: 'dragon', value: 3 })
    expect(MockWasmGame.lastDiscardArg).toBe('cn')

    // 通常数牌 (3p) は "3p"
    bridge.discardTile({ suit: 'pin', value: 3 })
    expect(MockWasmGame.lastDiscardArg).toBe('3p')
  })

  it('drawTile / executeCpuTurn / 状態取得 API が呼べる', () => {
    const bridge = WasmGameBridge.createHybrid('me', 0)
    expect(bridge.drawTile()).toBe(true)
    expect(bridge.executeCpuTurn()).toBe('5m')
    expect(bridge.getGameStateJson()).toBe('{"phase":"game"}')
    expect(bridge.getWallCount()).toBe(70)
    expect(bridge.getShanten()).toBe(2)
    expect(bridge.isCurrentPlayerCpu()).toBe(true)
    expect(bridge.isCurrentPlayerHuman()).toBe(false)
    expect(bridge.isGameOver()).toBe(false)
  })

  it('Wasm 未初期化なら create 系は例外', () => {
    __setWasmModuleForTest(null)
    expect(() => WasmGameBridge.createHybrid('me', 0)).toThrow(/初期化/)
    expect(() => WasmGameBridge.createAllHuman(['a', 'b', 'c', 'd'])).toThrow(
      /初期化/
    )
  })

  // ---- Round loop (Issue #27) ----

  it('resolveDraw は number[] を Uint32Array にして Rust に渡す', () => {
    const bridge = WasmGameBridge.createHybrid('me', 0)
    bridge.resolveDraw([0, 2])
    expect(MockWasmGame.lastResolveDrawArg).toEqual([0, 2])
  })

  it('resolveWinTsumo は ScoringResult JSON を RoundWinSummary に整形する', () => {
    const bridge = WasmGameBridge.createHybrid('me', 0)
    const summary = bridge.resolveWinTsumo(1)
    expect(MockWasmGame.lastResolveTsumoArg).toBe(1)
    expect(summary).not.toBeNull()
    expect(summary).not.toBe(SHIBARI_BLOCKED)
    if (summary === null || summary === SHIBARI_BLOCKED) throw new Error('unreachable')
    expect(summary.winner).toBe(1)
    expect(summary?.winType).toBe('tsumo')
    expect(summary?.from).toBeUndefined()
    expect(summary?.han).toBe(3)
    expect(summary?.fu).toBe(30)
    expect(summary?.totalPoints).toBe(3900)
    expect(summary?.yaku).toEqual(['Riichi', 'Pinfu', 'Tsumo'])
  })

  it('resolveWinRon は from を埋めた RoundWinSummary を返す', () => {
    const bridge = WasmGameBridge.createHybrid('me', 0)
    const summary = bridge.resolveWinRon(0, 2)
    expect(MockWasmGame.lastResolveRonArgs).toEqual([0, 2])
    if (summary === null || summary === SHIBARI_BLOCKED) throw new Error('unreachable')
    expect(summary.winType).toBe('ron')
    expect(summary.from).toBe(2)
    expect(summary.totalPoints).toBe(2600)
  })

  it('#143 本場縛りブロックのセンチネル JSON は SHIBARI_BLOCKED を返す (ツモ)', () => {
    // wasm が {"shibariBlocked":true} を返したら、null (和了形不成立) と区別して
    // センチネルを返す。これが #143 の TS 側の核心分岐。
    class ShibariGame extends MockWasmGame {
      static override newHybrid(_n: string, _p: number): ShibariGame {
        return new ShibariGame()
      }
      override resolveWinTsumo(): string {
        return JSON.stringify({ shibariBlocked: true })
      }
    }
    __setWasmModuleForTest({
      ...fakeModule,
      WasmGame: ShibariGame as unknown as typeof import('../../pkg/xmj_core.js').WasmGame,
    } as typeof import('../../pkg/xmj_core.js'))
    const bridge = WasmGameBridge.createHybrid('me', 0)
    const summary = bridge.resolveWinTsumo(0)
    expect(summary).toBe(SHIBARI_BLOCKED)
    // 空文字 (和了形不成立) パスとは異なり null ではない
    expect(summary).not.toBeNull()
  })

  it('#143 本場縛りブロックのセンチネルはロンでも返る', () => {
    class ShibariGame extends MockWasmGame {
      static override newHybrid(_n: string, _p: number): ShibariGame {
        return new ShibariGame()
      }
      override resolveWinRon(): string {
        return JSON.stringify({ shibariBlocked: true })
      }
    }
    __setWasmModuleForTest({
      ...fakeModule,
      WasmGame: ShibariGame as unknown as typeof import('../../pkg/xmj_core.js').WasmGame,
    } as typeof import('../../pkg/xmj_core.js'))
    const bridge = WasmGameBridge.createHybrid('me', 0)
    expect(bridge.resolveWinRon(0, 1)).toBe(SHIBARI_BLOCKED)
  })

  it('#143 shibariBlocked が true 以外 (false) ならセンチネルにせず通常サマリとして整形する', () => {
    // 厳密等価 (=== true) の境界。shibariBlocked:false を持つ正規サマリは
    // センチネル扱いせず RoundWinSummary に整形する (誤センチネル化しない)。
    class FalseFlagGame extends MockWasmGame {
      static override newHybrid(_n: string, _p: number): FalseFlagGame {
        return new FalseFlagGame()
      }
      override resolveWinTsumo(): string {
        return JSON.stringify({
          shibariBlocked: false,
          han: 4,
          fu: 30,
          totalPoints: 8000,
          yaku: ['Haitei'],
        })
      }
    }
    __setWasmModuleForTest({
      ...fakeModule,
      WasmGame: FalseFlagGame as unknown as typeof import('../../pkg/xmj_core.js').WasmGame,
    } as typeof import('../../pkg/xmj_core.js'))
    const bridge = WasmGameBridge.createHybrid('me', 0)
    const summary = bridge.resolveWinTsumo(0)
    expect(summary).not.toBe(SHIBARI_BLOCKED)
    if (summary === null || summary === SHIBARI_BLOCKED) throw new Error('unreachable')
    expect(summary.han).toBe(4)
    expect(summary.totalPoints).toBe(8000)
  })

  it('#143 槍槓ロンの本場縛りブロックもセンチネルを返す', () => {
    class ChankanGame extends MockWasmGame {
      static override newHybrid(_n: string, _p: number): ChankanGame {
        return new ChankanGame()
      }
      resolveWinChankan(): string {
        return JSON.stringify({ shibariBlocked: true })
      }
    }
    __setWasmModuleForTest({
      ...fakeModule,
      WasmGame: ChankanGame as unknown as typeof import('../../pkg/xmj_core.js').WasmGame,
    } as typeof import('../../pkg/xmj_core.js'))
    const bridge = WasmGameBridge.createHybrid('me', 0)
    expect(bridge.resolveWinChankan(1, 0)).toBe(SHIBARI_BLOCKED)
  })

  it('和了形でないとき (空文字) は null を返す', () => {
    class EmptyGame extends MockWasmGame {
      static override newHybrid(_n: string, _p: number): EmptyGame {
        return new EmptyGame()
      }
      override resolveWinTsumo(): string {
        return ''
      }
      override resolveWinRon(): string {
        return ''
      }
    }
    __setWasmModuleForTest({
      ...fakeModule,
      WasmGame: EmptyGame as unknown as typeof import('../../pkg/xmj_core.js').WasmGame,
    } as typeof import('../../pkg/xmj_core.js'))
    const bridge = WasmGameBridge.createHybrid('me', 0)
    expect(bridge.resolveWinTsumo(0)).toBeNull()
    expect(bridge.resolveWinRon(0, 1)).toBeNull()
  })

  // ---- Tsumo / Ron 宣言 (Issue #35) ----

  it('canTsumo は引数省略時は現在のプレイヤー idx を Rust に渡す', () => {
    const bridge = WasmGameBridge.createHybrid('me', 0)
    // mock の getCurrentPlayerId() は 1 を返す → canTsumo() は false
    expect(bridge.canTsumo()).toBe(false)
    expect(MockWasmGame.lastCanTsumoArg).toBe(1)
    // 明示 idx は素通し
    expect(bridge.canTsumo(0 as const)).toBe(true)
    expect(MockWasmGame.lastCanTsumoArg).toBe(0)
  })

  it('canRon は指定 idx を Rust に渡す', () => {
    const bridge = WasmGameBridge.createHybrid('me', 0)
    expect(bridge.canRon(2 as const)).toBe(true)
    expect(MockWasmGame.lastCanRonArg).toBe(2)
    expect(bridge.canRon(0 as const)).toBe(false)
  })

  it('getLastDiscarder は number を PlayerIndex として返す', () => {
    const bridge = WasmGameBridge.createHybrid('me', 0)
    expect(bridge.getLastDiscarder()).toBe(3)
  })

  it('round 系 getter / nextRound / getLastOutcomeJson が透過する', () => {
    const bridge = WasmGameBridge.createHybrid('me', 0)
    expect(bridge.getRound()).toBe(2)
    expect(bridge.getHonba()).toBe(1)
    expect(bridge.getDealer()).toBe(0)
    expect(bridge.getRiichiSticks()).toBe(1)
    expect(bridge.nextRound()).toBe(true)
    const json = bridge.getLastOutcomeJson()
    expect(json).toContain('"kind":"win"')
  })
})
