// 牌グラフィックス生成のテスト (Issue #4)
//
// jsdom + PixiJS v8 では WebGL レンダラは起動できないが、
// Container / Graphics / Text オブジェクトの生成と階層構造は
// レンダラ非依存で検証できる。

import { describe, it, expect } from 'vitest'
import { Container, Text } from 'pixi.js'
import {
  createTileGraphics,
  createTileBackGraphics,
  enumerateAllTiles,
} from './tile'
import type { Tile } from './types'
import { TILE } from './constants'

describe('enumerateAllTiles', () => {
  it('全 34 種類 (1-9mps + 風4 + 三元3)', () => {
    const tiles = enumerateAllTiles()
    expect(tiles).toHaveLength(34)
    expect(tiles.filter(t => t.suit === 'man')).toHaveLength(9)
    expect(tiles.filter(t => t.suit === 'pin')).toHaveLength(9)
    expect(tiles.filter(t => t.suit === 'sou')).toHaveLength(9)
    expect(tiles.filter(t => t.suit === 'wind')).toHaveLength(4)
    expect(tiles.filter(t => t.suit === 'dragon')).toHaveLength(3)
  })
})

describe('createTileGraphics', () => {
  it('Container を返す', () => {
    const c = createTileGraphics({ suit: 'man', value: 1 })
    expect(c).toBeInstanceOf(Container)
  })

  it('label に CUI コードが入る', () => {
    expect(createTileGraphics({ suit: 'man', value: 1 }).label).toBe('1m')
    expect(createTileGraphics({ suit: 'pin', value: 5 }).label).toBe('5p')
    expect(createTileGraphics({ suit: 'sou', value: 9 }).label).toBe('9s')
    expect(createTileGraphics({ suit: 'wind', value: 1 }).label).toBe('to')
    expect(createTileGraphics({ suit: 'wind', value: 4 }).label).toBe('pe')
    expect(createTileGraphics({ suit: 'dragon', value: 1 }).label).toBe('hk')
    expect(createTileGraphics({ suit: 'dragon', value: 3 }).label).toBe('cn')
  })

  it('全 34 種類が例外なく生成できる', () => {
    for (const tile of enumerateAllTiles()) {
      const c = createTileGraphics(tile)
      expect(c).toBeInstanceOf(Container)
      // 数牌は面 + 上段 + 下段 = 3 子、字牌は下段なしで 2 子
      const expectedChildren =
        tile.suit === 'wind' || tile.suit === 'dragon' ? 2 : 3
      expect(c.children).toHaveLength(expectedChildren)
    }
  })

  it('数牌は上段に数字、下段にスート漢字', () => {
    const c = createTileGraphics({ suit: 'man', value: 7 })
    const texts = c.children.filter((ch): ch is Text => ch instanceof Text)
    expect(texts).toHaveLength(2)
    expect(texts[0].text).toBe('7')
    expect(texts[1].text).toBe('萬')
  })

  it('風牌は東南西北の漢字', () => {
    const winds = [1, 2, 3, 4] as const
    const expected = ['東', '南', '西', '北']
    winds.forEach((v, i) => {
      const c = createTileGraphics({ suit: 'wind', value: v })
      const text = c.children.find((ch): ch is Text => ch instanceof Text)
      expect(text?.text).toBe(expected[i])
    })
  })

  it('三元牌は白發中', () => {
    const expected = ['白', '發', '中']
    expected.forEach((kanji, i) => {
      const c = createTileGraphics({ suit: 'dragon', value: i + 1 })
      const text = c.children.find((ch): ch is Text => ch instanceof Text)
      expect(text?.text).toBe(kanji)
    })
  })

  it('赤ドラは文字が赤 (TILE.redTextColor)', () => {
    const red: Tile = { suit: 'man', value: 5, isRed: true }
    const c = createTileGraphics(red)
    expect(c.label).toBe('5mr')
    const text = c.children.find((ch): ch is Text => ch instanceof Text)
    // PIXI v8 の TextStyle.fill は number / string / FillStyle を取りうるが、
    // 単色 number を渡しているので number で返る。
    expect(text?.style.fill).toBe(TILE.redTextColor)
  })

  it('赤ドラでない 5m は通常色 (黒系)', () => {
    const c = createTileGraphics({ suit: 'man', value: 5 })
    const text = c.children.find((ch): ch is Text => ch instanceof Text)
    expect(text?.style.fill).toBe(TILE.textColor)
  })

  it('索子は緑、筒子は青の文字色', () => {
    const sou = createTileGraphics({ suit: 'sou', value: 3 })
    const souText = sou.children.find((ch): ch is Text => ch instanceof Text)
    expect(souText?.style.fill).toBe(TILE.souColor)

    const pin = createTileGraphics({ suit: 'pin', value: 3 })
    const pinText = pin.children.find((ch): ch is Text => ch instanceof Text)
    expect(pinText?.style.fill).toBe(TILE.pinColor)
  })
})

describe('createTileBackGraphics', () => {
  it("label が 'back'", () => {
    expect(createTileBackGraphics().label).toBe('back')
  })

  it('面 + 装飾 = 2 子', () => {
    const c = createTileBackGraphics()
    expect(c.children).toHaveLength(2)
  })
})
