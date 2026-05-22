// 席色 (風固定) の単一ソース。
//
// 方針: 東=赤 / 南=黄 / 西=青 / 北=緑。風で固定し、起家移動 (= 風が変わる
// プレイヤー名ではなく、卓上の方位) では「同じ風 = 同じ色」を保つ。
//
// この色は 3 か所で共有する:
//   1. HTML score panel (htmlUi.ts::renderScores)
//   2. HTML 実況ログ chat (htmlUi.ts::SPEAKER_PATTERNS → spk-east 等の CSS)
//   3. Pixi 卓上の席色ドット (table.ts::addSeatColorDots)
//
// 「南 (CPU) と人間が両方黄色」のような衝突は、人間 = 南風のときは人間も黄色を
// 当てて (席風で決まる) むしろ統一する方が分かりやすい。旧 spk-self の黄色は廃止。
//
// fg (文字色) は背景色の輝度に応じて白/黒を自動選択 (`pickReadableFgColor`)
// する。水色 (西=青) の白文字問題は、ここで一律に判定して片付ける。

import type { PlayerIndex } from './types'

export type SeatWind = 'east' | 'south' | 'west' | 'north'

export interface SeatColor {
  /** 16進文字列。CSS 用 (#rrggbb) */
  bg: string
  /** 16進文字列。CSS 用 (#rrggbb)。bg の可読性から自動選択 */
  fg: string
  /** Pixi 用の数値色 (0xrrggbb) */
  bgNumber: number
}

/**
 * 背景色 (#rrggbb) に対して読みやすい文字色 (黒 or 白) を返す。
 * sRGB 輝度の WCAG 推奨ライン (~0.55) で切る — 中明度の青や緑は黒文字優先。
 */
export const pickReadableFgColor = (bgHex: string): string => {
  const hex = bgHex.replace(/^#/, '')
  if (hex.length !== 6) return '#1a1a1a'
  const r = parseInt(hex.slice(0, 2), 16) / 255
  const g = parseInt(hex.slice(2, 4), 16) / 255
  const b = parseInt(hex.slice(4, 6), 16) / 255
  // 簡易輝度 (WCAG 相当の係数)
  const lum = 0.2126 * r + 0.7152 * g + 0.0722 * b
  return lum > 0.55 ? '#1a1a1a' : '#ffffff'
}

const buildSeatColor = (bg: string): SeatColor => ({
  bg,
  fg: pickReadableFgColor(bg),
  bgNumber: parseInt(bg.replace(/^#/, ''), 16),
})

/**
 * 風 → 席色の対応。
 * 彩度を抑え、4 色がはっきり区別できる中明度系。
 * - 東 (#d65a4a): 落ち着いた赤橙。黒文字が乗る明度。
 * - 南 (#e8c94a): 黄。黒文字。
 * - 西 (#3a7ab8): 青。白文字 (pickReadableFgColor で自動)。
 * - 北 (#3f9a5b): 緑。白文字 or 黒文字いずれも可読、自動選択。
 */
export const SEAT_COLORS: Record<SeatWind, SeatColor> = {
  east: buildSeatColor('#d65a4a'),
  south: buildSeatColor('#e8c94a'),
  west: buildSeatColor('#3a7ab8'),
  north: buildSeatColor('#3f9a5b'),
}

/**
 * player.id (= PlayerIndex 0..3) はゲーム開始時の風に対応する。
 * id 0 = 東、id 1 = 南、id 2 = 西、id 3 = 北。
 * 起家移動 (dealer rotation) があっても player.id は不変、つまり「player.id で
 * 引いた SEAT_COLORS」は風固定の色を返す。
 */
export const PLAYER_WIND_BY_ID: readonly SeatWind[] = ['east', 'south', 'west', 'north']

export const seatColorForPlayerId = (id: PlayerIndex): SeatColor =>
  SEAT_COLORS[PLAYER_WIND_BY_ID[id]]

export const seatWindForPlayerId = (id: PlayerIndex): SeatWind => PLAYER_WIND_BY_ID[id]
