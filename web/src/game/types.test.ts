// 牌文字列変換のラウンドトリップ確認 (Issue #3)

import { describe, it, expect } from 'vitest'
import { tileToCuiCode, tileFromCuiCode, type Tile } from './types'

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
  })
})
