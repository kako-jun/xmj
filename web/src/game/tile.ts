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
/**
 * glyph を bbox いっぱいに拡大する scale を計算する (アスペクト維持)。
 * Pixi の Text は fontFamily / size から自然サイズが決まるので、`text.width` を
 * 測ってから TILE.width に揃える倍率を返す。jsdom 等で canvas が無い (= 測定不可)
 * 環境では 1 を返してテストを通す。
 */
const computeFitScale = (text: Text): number => {
  try {
    const w = text.width
    if (!w || w <= 0 || !Number.isFinite(w)) return 1
    return TILE.width / w
  } catch {
    return 1
  }
}

export interface TileGraphicsOptions {
  /**
   * 選択中の牌。`true` のとき、面色を白からハイライト色 (黄) に変える (#98)。
   * 旧仕様の「外側の glow 枠」描画はやめて、面そのものを着色するのが意図 (kako-jun 指示)。
   */
  selected?: boolean
}

export const createTileGraphics = (tile: Tile, options: TileGraphicsOptions = {}): Container => {
  const container = new Container()
  container.label = tileToCuiCode(tile)

  // 角丸面 = 「ユニコード文字の枠の内側」を塗る。Unicode 牌の枠線・
  // 中の絵柄はこの上に重ねる。visible なタイル境界は bbox (TILE.width × TILE.height)
  // と一致するので、spacing = TILE.width で完全密着する。
  // 選択中は白 → 黄色 (TILE.selectedFaceColor) に切り替え。
  const faceColor = options.selected === true ? TILE.selectedFaceColor : 0xffffff
  const face = new Graphics()
  face.roundRect(0, 0, TILE.width, TILE.height, TILE.cornerRadius).fill({ color: faceColor })
  container.addChild(face)

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
  // 自然 fontSize で 1 度作って width を測る → bbox にフィットさせる。
  // これでフォントの em metrics に依存せず常に TILE.width いっぱいで描画される。
  const style = new TextStyle({
    fontFamily: TILE_FONT_FAMILY,
    fontSize: 64,
    fill: suitColor,
  })
  const glyph = new Text({ text: tileToUnicodeChar(tile), style })
  glyph.anchor.set(0.5)
  container.addChild(glyph)
  glyph.scale.set(computeFitScale(glyph))
  glyph.x = TILE.width / 2
  glyph.y = TILE.height / 2

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

  // #99: 麻雀牌の裏面は実物では白ではなく象牙色 (クリーム色)。
  // 表向き牌の白と差別化することで「裏」の認識が一目で立つ。
  const face = new Graphics()
  face
    .roundRect(0, 0, TILE.width, TILE.height, TILE.cornerRadius)
    .fill({ color: TILE.backFaceColor })
  container.addChild(face)

  const style = new TextStyle({
    fontFamily: TILE_FONT_FAMILY,
    fontSize: 64,
    fill: TILE.backColor,
  })
  const glyph = new Text({ text: '\u{1F02B}', style })
  glyph.anchor.set(0.5)
  container.addChild(glyph)
  glyph.scale.set(computeFitScale(glyph))
  glyph.x = TILE.width / 2
  glyph.y = TILE.height / 2

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
