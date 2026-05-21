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

  it('全 34 種類が例外なく生成できる (面 + Unicode 文字 = 2 子)', () => {
    for (const tile of enumerateAllTiles()) {
      const c = createTileGraphics(tile)
      expect(c).toBeInstanceOf(Container)
      // 新実装: 全牌が「角丸面 + Unicode 麻雀タイル文字 1 つ」の 2 子構成
      expect(c.children).toHaveLength(2)
    }
  })

  it('数牌は Unicode 麻雀タイル文字で描画される (1m=🀇, 5p=🀝, 9s=🀘)', () => {
    expect(
      (createTileGraphics({ suit: 'man', value: 1 }).children.find(
        (ch): ch is Text => ch instanceof Text
      ) as Text).text
    ).toBe('\u{1F007}') // 🀇
    expect(
      (createTileGraphics({ suit: 'pin', value: 5 }).children.find(
        (ch): ch is Text => ch instanceof Text
      ) as Text).text
    ).toBe('\u{1F01D}') // 🀝
    expect(
      (createTileGraphics({ suit: 'sou', value: 9 }).children.find(
        (ch): ch is Text => ch instanceof Text
      ) as Text).text
    ).toBe('\u{1F018}') // 🀘
  })

  it('風牌は U+1F000..U+1F003 (東南西北)', () => {
    const expected = ['\u{1F000}', '\u{1F001}', '\u{1F002}', '\u{1F003}']
    expected.forEach((glyph, i) => {
      const c = createTileGraphics({ suit: 'wind', value: i + 1 })
      const text = c.children.find((ch): ch is Text => ch instanceof Text)
      expect(text?.text).toBe(glyph)
    })
  })

  it('三元牌は白(🀆)發(🀅)中(🀄) (内部 value=1,2,3)', () => {
    const expected = ['\u{1F006}', '\u{1F005}', '\u{1F004}']
    expected.forEach((glyph, i) => {
      const c = createTileGraphics({ suit: 'dragon', value: i + 1 })
      const text = c.children.find((ch): ch is Text => ch instanceof Text)
      expect(text?.text).toBe(glyph)
    })
  })

  it('赤ドラは文字色が TILE.redTextColor', () => {
    const red: Tile = { suit: 'man', value: 5, isRed: true }
    const c = createTileGraphics(red)
    expect(c.label).toBe('5mr')
    const text = c.children.find((ch): ch is Text => ch instanceof Text)
    expect(text?.style.fill).toBe(TILE.redTextColor)
  })

  it('赤ドラでない通常牌は文字色が TILE.textColor (スート差し色は Unicode 絵柄に委譲)', () => {
    const c = createTileGraphics({ suit: 'sou', value: 3 })
    const text = c.children.find((ch): ch is Text => ch instanceof Text)
    expect(text?.style.fill).toBe(TILE.textColor)
  })
})

describe('createTileBackGraphics', () => {
  it("label が 'back'", () => {
    expect(createTileBackGraphics().label).toBe('back')
  })

  it('面 + Unicode 裏牌文字 (🀫) = 2 子', () => {
    const c = createTileBackGraphics()
    expect(c.children).toHaveLength(2)
    const text = c.children.find((ch): ch is Text => ch instanceof Text)
    expect(text?.text).toBe('\u{1F02B}')
  })
})
