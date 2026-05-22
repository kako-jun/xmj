// xmj Web 表示まわりの定数。
//
// Pixi のステージは正方形に固定し、操作系・実況ログ・点数表はすべて HTML 側に
// 出すレイアウト方針。CSS Grid 側 (index.html) で「スマホ縦は卓→操作系の縦積み、
// PC 横は卓→操作系の横並び」を切り替える。
//
// 卓は square (STAGE_WIDTH = STAGE_HEIGHT) を前提に対称配置を組む。

export const STAGE_WIDTH = 720
export const STAGE_HEIGHT = 720

// 卓の中心。手牌・河はすべてここを基準に配置する。
export const TABLE_CENTER_X = STAGE_WIDTH / 2
export const TABLE_CENTER_Y = STAGE_HEIGHT / 2

// 卓背景: 彩度を抑えたフェルトグリーン。長時間プレイで目が疲れない明度に寄せている。
export const TABLE_BG_COLOR = 0x1f3a2a
export const TABLE_BORDER_COLOR = 0x161616
export const TABLE_FELT_INNER_COLOR = 0x1a3024
export const TABLE_GLOW_COLOR = 0x4a1f24
export const PANEL_BG_COLOR = 0x14140f
export const PANEL_BORDER_COLOR = 0x7a6038
export const PANEL_ACCENT_COLOR = 0xb39a6e
export const DISCARD_SLOT_COLOR = 0x223a2d
export const TEXT_PRIMARY_COLOR = 0xece2c4
export const TEXT_MUTED_COLOR = 0xa89c80
export const TEXT_DANGER_COLOR = 0xb84a4a
export const TURN_GLOW_COLOR = 0xe8c47a
export const SHADOW_COLOR = 0x000000
export const EVENT_LOG_LIMIT = 24
export const EVENT_LOG_VISIBLE_COUNT = 14

// 牌の基本サイズ。
// 720×720 の盤面で 13 牌 + ツモ牌 (合計 14) を中央寄せに収めるため、handSpacing は
// 牌 width 以下にせず、合計幅 ≦ 卓内幅 (約 640px) になるよう調整する。
//
// ピッチは「牌が隙間なく密着する」値 (= width * scale) に揃える。
// 手牌・CPU 手牌・河でばらつきがあると見た目が散らかるので意図的に統一する。
export const TILE = {
  width: 40,
  height: 56,
  // 自家手牌: 40 (= width)
  handSpacing: 40,
  // CPU 裏向き手牌: 28 (= width * cpuHandScale 0.7)
  cpuHandScale: 0.7,
  cpuHandSpacing: 28,
  // 河: 25 / 35 (= width * 0.625 / height * 0.625)
  discardColPitch: 25,
  discardRowPitch: 35,
  discardScale: 0.625,
  cornerRadius: 5,
  textColor: 0x1a1a1a,
  souColor: 0x2f6b3a,
  pinColor: 0x365a85,
  // 萬子は黒に寄せたダークレッド (字牌は textColor のまま黒)
  manColor: 0x4a2020,
  redTextColor: 0xa83a3a,
  // 裏面 (Unicode 🀫) の glyph 色。肌色寄りの竹色で「竹の裏面」風に
  backColor: 0xa07550,
  // 裏面の地色 (#99)。実物の麻雀牌の裏は白でなく象牙色なので、表向き牌の白と差別化する。
  backFaceColor: 0xf0e2c0,
  // 選択中の牌の面色 (#98)。白 → 黄系で「光ってる」感を出す。
  // 周囲に glow を描かず、面色だけで状態を表現する方針。
  selectedFaceColor: 0xf5d96a,
} as const

// 河 6×3 グリッドの幅・高さ (タイル本体だけ、間隔ピッチで算出)。
export const DISCARD_COLS = 6
export const DISCARD_ROWS = 3
export const DISCARD_BLOCK_WIDTH = DISCARD_COLS * TILE.discardColPitch
export const DISCARD_BLOCK_HEIGHT = DISCARD_ROWS * TILE.discardRowPitch

// 河ブロックの内縁から卓中心までの距離。4 方位で同じ値を使うことで対称配置を保証する。
// 中央に「直前打牌・流れ表示」などのちょっとした余地を残しつつ、河同士の被りを防ぐ。
export const DISCARD_INNER_MARGIN = 96
