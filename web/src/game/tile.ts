// 牌の PixiJS 表現
//
// Unicode 麻雀牌 (U+1F000 〜 U+1F021) を 1 文字 Text として描画する。
// 牌の絵柄は Unicode 文字自体に含まれるため、自前で枠 + 漢字 + 系統マークを
// 組み立てる必要が無い。サイズ調整・色付け (赤ドラ) は TextStyle で行う。
//
// container.label には CUI コード (e.g. "5mr") をセットし、テストや
// デバッグから参照できるようにする (htmlUi / table から DOM/Pixi 検索する際の鍵)。

import { Container, Graphics, Text, TextStyle } from 'pixi.js'
import { TILE } from './constants'
import type { Tile, Suit } from './types'
import { tileToCuiCode } from './types'

/**
 * 牌 1 枚に対応する Unicode 麻雀タイル文字を返す。
 *
 * Unicode ブロック "Mahjong Tiles" (U+1F000–U+1F02B):
 *   - 風: 東南西北 → U+1F000, U+1F001, U+1F002, U+1F003
 *   - 三元: 中發白 → U+1F004 (中), U+1F005 (發), U+1F006 (白)
 *   - 萬子 1-9 → U+1F007 .. U+1F00F
 *   - 索子 1-9 → U+1F010 .. U+1F018
 *   - 筒子 1-9 → U+1F019 .. U+1F021
 *   - 裏: 🀫 U+1F02B
 *
 * NOTE: 三元の Unicode 順は 中→發→白 で、内部 value (1=白, 2=發, 3=中) と逆。
 */
const tileToUnicodeChar = (tile: Tile): string => {
  const cp = (n: number): string => String.fromCodePoint(n)
  switch (tile.suit) {
    case 'man':
      return cp(0x1f007 + (tile.value - 1))
    case 'sou':
      return cp(0x1f010 + (tile.value - 1))
    case 'pin':
      return cp(0x1f019 + (tile.value - 1))
    case 'wind':
      return cp(0x1f000 + (tile.value - 1))
    case 'dragon': {
      // value: 1=白(U+1F006), 2=發(U+1F005), 3=中(U+1F004)
      const map = [0x1f006, 0x1f005, 0x1f004]
      return cp(map[tile.value - 1] ?? 0x1f004)
    }
  }
}

/**
 * Unicode 麻雀牌の描画フォント。
 *
 * `web/public/fonts/noto-sans-symbols2-mahjong.woff2` (Noto Sans Symbols 2 の
 * U+1F000-1F02B subset, 12KB) を `@font-face: XmjMahjong` として index.html で
 * 登録済み。**fallback は持たない** — どの OS でも完全に同じ glyph で描画する
 * ことが目的なので、未ロード時は font-display: block で「フォントが揃うまで
 * 何も描画しない」方針 (main.ts の `document.fonts.ready` で待つ)。
 */
const TILE_FONT_FAMILY = 'XmjMahjong'

/**
 * 牌 1 枚 (表向き) を PIXI.Container で生成する。
 *
 * - 視覚は **Unicode 麻雀牌 1 文字のみ**。背景塗りも外枠 stroke も持たない。
 *   牌の絵柄と枠線は Unicode 文字 (🀇 等) が自前で担う。
 * - 透明 (alpha 0) の Graphics rect を hit-area / 選択 glow の座標基準として
 *   1 枚仕込む。fontFamily は mono symbol フォントを優先、VS-15 で text presentation
 *   を強制して、Apple Color Emoji 等のカラー絵文字に乗っ取られないようにする。
 * - 系統色は `fill` で適用 (mono glyph なのでそのまま色が乗る)。赤ドラは最優先。
 */
export const createTileGraphics = (tile: Tile): Container => {
  const container = new Container()
  container.label = tileToCuiCode(tile)

  // 透明な hit-area / 選択 glow の座標基準。視覚的には何も出ない (alpha 0)。
  // 「文字＝牌」方針: Unicode 牌が自前で枠と絵柄を持つので、cream 背景は描かず
  // 緑フェルトに文字を直接置く。
  const hitArea = new Graphics()
  hitArea.rect(0, 0, TILE.width, TILE.height).fill({ color: 0x000000, alpha: 0 })
  container.addChild(hitArea)

  // Unicode 麻雀タイル文字 1 つ
  // 系統ごとに色分け: 索子=緑 / 筒子=青 / 萬子=ダークレッド / 字牌=黒 / 赤ドラ=赤 (最優先)
  const suitColor = (() => {
    if (tile.isRed) return TILE.redTextColor
    switch (tile.suit) {
      case 'sou':
        return TILE.souColor
      case 'pin':
        return TILE.pinColor
      case 'man':
        return TILE.manColor
      default:
        return TILE.textColor
    }
  })()
  const style = new TextStyle({
    fontFamily: TILE_FONT_FAMILY,
    // bbox (40×56) をなるべく埋めるサイズ。font の em に内余白があるので height
    // より少し大きめを取り、glyph 全体が見える範囲で詰める。
    fontSize: 60,
    fill: suitColor,
  })
  const glyph = new Text({ text: tileToUnicodeChar(tile), style })
  glyph.anchor.set(0.5)
  glyph.x = TILE.width / 2
  glyph.y = TILE.height / 2
  container.addChild(glyph)

  return container
}

/**
 * 裏向き (伏せ牌) の表示。河で他家から見える形と、開始前の山の表示に使う。
 *
 * Unicode 🀫 (U+1F02B) "Mahjong Tile Back" を 1 文字描く。背景は青系で
 * 「裏向き」を強調 (絵文字フォントによっては U+1F02B が描けないシステム
 * もあるため、青背景があれば最悪 "裏" と分かる)。
 */
export const createTileBackGraphics = (): Container => {
  const container = new Container()
  container.label = 'back'

  // 表向き牌と同じ方針: 透明 hit-area + Unicode 🀫 を mono 強制で 1 文字描く。
  // 🀫 が自前で枠 + 竹の裏面パターン (ハッチング) を持つので、自前描画はしない。
  const hitArea = new Graphics()
  hitArea.rect(0, 0, TILE.width, TILE.height).fill({ color: 0x000000, alpha: 0 })
  container.addChild(hitArea)

  const style = new TextStyle({
    fontFamily: TILE_FONT_FAMILY,
    fontSize: 60,
    fill: TILE.backColor,
  })
  const glyph = new Text({ text: '\u{1F02B}', style })
  glyph.anchor.set(0.5)
  glyph.x = TILE.width / 2
  glyph.y = TILE.height / 2
  container.addChild(glyph)

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
