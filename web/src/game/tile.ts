// 牌の PixiJS 表現 (Issue #4)
//
// createTileGraphics(tile) は牌 1 枚を Container として返す。
// 内部は Graphics (枠 + 面) + Text (記号) の入れ子。後の Issue で
// 選択中 / ホバー / 打牌済み状態を切り替えやすいよう、Container.label に
// CUI コードを設定する。

import { Container, Graphics, Text, TextStyle } from 'pixi.js'
import { TILE } from './constants'
import type { Tile, Suit } from './types'
import { tileToCuiCode } from './types'

// 牌記号テーブル。
//
// 数牌は数字 + 系統。簡素な視認性のため最初はテキスト表示にする (m/p/s)。
// 字牌は漢字 1 文字。
const WIND_KANJI = ['東', '南', '西', '北'] as const
const DRAGON_KANJI = ['白', '發', '中'] as const

const suitMark: Record<Suit, string> = {
  man: '萬',
  pin: '筒',
  sou: '索',
  wind: '',
  dragon: '',
}

const suitTextColor = (suit: Suit, isRed?: boolean): number => {
  if (isRed) return TILE.redTextColor
  switch (suit) {
    case 'sou':
      return TILE.souColor
    case 'pin':
      return TILE.pinColor
    default:
      return TILE.textColor
  }
}

/**
 * 牌 1 枚分の表示テキスト (上下 2 行のうち上段)。
 *   - 数牌: 数字
 *   - 風牌: 漢字 (東南西北)
 *   - 三元: 漢字 (白發中)
 */
const topGlyph = (tile: Tile): string => {
  switch (tile.suit) {
    case 'man':
    case 'pin':
    case 'sou':
      return String(tile.value)
    case 'wind':
      return WIND_KANJI[tile.value - 1] ?? '?'
    case 'dragon':
      return DRAGON_KANJI[tile.value - 1] ?? '?'
  }
}

/**
 * 牌 1 枚を PIXI.Container で生成する。
 *
 * - 角丸長方形の白い面 (TILE.faceColor)
 * - 縁取り (TILE.edgeColor)
 * - 上段: 数字 or 漢字
 * - 下段: 系統マーク (萬/筒/索)、字牌は空
 * - 赤ドラ (isRed=true) は文字が赤
 *
 * container.label には CUI コード (e.g. "5mr") をセットし、テストや
 * デバッグから参照できるようにする。
 */
export const createTileGraphics = (tile: Tile): Container => {
  const container = new Container()
  container.label = tileToCuiCode(tile)

  // 面
  const face = new Graphics()
  face
    .roundRect(0, 0, TILE.width, TILE.height, TILE.cornerRadius)
    .fill({ color: TILE.faceColor })
    .stroke({ color: TILE.edgeColor, width: 2 })
  container.addChild(face)

  const textColor = suitTextColor(tile.suit, tile.isRed)

  // 上段グリフ (大きめ)
  const topStyle = new TextStyle({
    fontFamily: 'sans-serif',
    fontSize: tile.suit === 'wind' || tile.suit === 'dragon' ? 28 : 32,
    fontWeight: 'bold',
    fill: textColor,
  })
  const top = new Text({ text: topGlyph(tile), style: topStyle })
  top.anchor.set(0.5)
  top.x = TILE.width / 2
  top.y = TILE.height * 0.36
  container.addChild(top)

  // 下段マーク (萬/筒/索)。字牌では非表示。
  const mark = suitMark[tile.suit]
  if (mark) {
    const bottomStyle = new TextStyle({
      fontFamily: 'sans-serif',
      fontSize: 18,
      fill: textColor,
    })
    const bottom = new Text({ text: mark, style: bottomStyle })
    bottom.anchor.set(0.5)
    bottom.x = TILE.width / 2
    bottom.y = TILE.height * 0.72
    container.addChild(bottom)
  }

  return container
}

/**
 * 裏向き (伏せ牌) の表示。河で他家から見える形と、開始前の山の表示に使う。
 * 青系の単色 + 細い縁取り。
 */
export const createTileBackGraphics = (): Container => {
  const container = new Container()
  container.label = 'back'

  const back = new Graphics()
  back
    .roundRect(0, 0, TILE.width, TILE.height, TILE.cornerRadius)
    .fill({ color: TILE.backColor })
    .stroke({ color: 0x0a1e3a, width: 2 })
  container.addChild(back)

  // 中央に小さい菱形 (背中の意匠)
  const ornament = new Graphics()
  const cx = TILE.width / 2
  const cy = TILE.height / 2
  const r = 8
  ornament
    .moveTo(cx, cy - r)
    .lineTo(cx + r, cy)
    .lineTo(cx, cy + r)
    .lineTo(cx - r, cy)
    .closePath()
    .fill({ color: 0xfaf3e0, alpha: 0.6 })
  container.addChild(ornament)

  return container
}

/**
 * 全 34 種類の牌をテスト・デバッグ用に列挙する。
 * (赤ドラを含まない素の集合)
 */
export const enumerateAllTiles = (): Tile[] => {
  const tiles: Tile[] = []
  const numberSuits = ['man', 'pin', 'sou'] as const satisfies readonly Suit[]
  for (const suit of numberSuits) {
    for (let v = 1; v <= 9; v++) tiles.push({ suit, value: v })
  }
  for (let v = 1; v <= 4; v++) tiles.push({ suit: 'wind', value: v })
  for (let v = 1; v <= 3; v++) tiles.push({ suit: 'dragon', value: v })
  return tiles
}
