// 牌文字列変換のラウンドトリップ確認 (Issue #3)

import { describe, it, expect } from 'vitest'
import { tileToCuiCode, tileFromCuiCode, parseRoundOutcome, type Tile } from './types'

describe('tileToCuiCode / tileFromCuiCode', () => {
  it('数牌 1-9 のラウンドトリップ (萬・筒・索)', () => {
    const suits = ['man', 'pin', 'sou'] as const
    for (const suit of suits) {
      for (let v = 1; v <= 9; v++) {
        const t: Tile = { suit, value: v }
        const code = tileToCuiCode(t)
        const back = tileFromCuiCode(code)
        expect(back).toEqual(t)
      }
    }
  })

  it('赤ドラを正しく扱う (5mr / 5pr / 5sr)', () => {
    const red: Tile = { suit: 'man', value: 5, isRed: true }
    expect(tileToCuiCode(red)).toBe('5mr')
    expect(tileFromCuiCode('5mr')).toEqual(red)
  })

  it('風牌 to/na/sa/pe', () => {
    expect(tileToCuiCode({ suit: 'wind', value: 1 })).toBe('to')
    expect(tileToCuiCode({ suit: 'wind', value: 4 })).toBe('pe')
    expect(tileFromCuiCode('to')).toEqual({ suit: 'wind', value: 1 })
    expect(tileFromCuiCode('pe')).toEqual({ suit: 'wind', value: 4 })
  })

  it('三元牌 hk/ht/cn', () => {
    expect(tileToCuiCode({ suit: 'dragon', value: 1 })).toBe('hk')
    expect(tileToCuiCode({ suit: 'dragon', value: 3 })).toBe('cn')
    expect(tileFromCuiCode('hk')).toEqual({ suit: 'dragon', value: 1 })
    expect(tileFromCuiCode('cn')).toEqual({ suit: 'dragon', value: 3 })
  })

  it('不正な文字列は null', () => {
    expect(tileFromCuiCode('99m')).toBeNull()
    expect(tileFromCuiCode('xx')).toBeNull()
    expect(tileFromCuiCode('')).toBeNull()
    expect(tileFromCuiCode('0m')).toBeNull()
    expect(tileFromCuiCode('1z')).toBeNull()
  })

  it('字牌の value が範囲外なら "?" を返す (防御フォールバック)', () => {
    expect(tileToCuiCode({ suit: 'wind', value: 0 })).toBe('?')
    expect(tileToCuiCode({ suit: 'wind', value: 5 })).toBe('?')
    expect(tileToCuiCode({ suit: 'dragon', value: 4 })).toBe('?')
    // '?' を再パースしても null
    expect(tileFromCuiCode('?')).toBeNull()
  })
})

describe('parseRoundOutcome (Issue #27)', () => {
  it('空文字 / 不正 JSON は null', () => {
    expect(parseRoundOutcome('')).toBeNull()
    expect(parseRoundOutcome('not-json')).toBeNull()
    expect(parseRoundOutcome('{"kind":"unknown"}')).toBeNull()
  })

  it('win (ツモ) を整形する', () => {
    const out = parseRoundOutcome(
      JSON.stringify({
        kind: 'win',
        winner: 1,
        winType: 'tsumo',
        han: 2,
        fu: 30,
        totalPoints: 2000,
        yaku: ['Riichi'],
      })
    )
    expect(out?.kind).toBe('win')
    if (out?.kind !== 'win') throw new Error('unreachable')
    expect(out.data.winner).toBe(1)
    expect(out.data.winType).toBe('tsumo')
    expect(out.data.from).toBeUndefined()
    expect(out.data.totalPoints).toBe(2000)
    expect(out.data.yaku).toEqual(['Riichi'])
  })

  it('win (ロン) は from を保持', () => {
    const out = parseRoundOutcome(
      JSON.stringify({
        kind: 'win',
        winner: 0,
        winType: 'ron',
        from: 2,
        han: 1,
        fu: 30,
        totalPoints: 1000,
        yaku: [],
      })
    )
    if (out?.kind !== 'win') throw new Error('unreachable')
    expect(out.data.from).toBe(2)
    expect(out.data.yaku).toEqual([])
  })

  it('draw は tenpaiPlayers を抽出', () => {
    const out = parseRoundOutcome(
      JSON.stringify({ kind: 'draw', tenpaiPlayers: [0, 3] })
    )
    if (out?.kind !== 'draw') throw new Error('unreachable')
    expect(out.data.tenpaiPlayers).toEqual([0, 3])
  })
})
