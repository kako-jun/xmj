// xmj Web 表示まわりの定数。
//
// 麻雀卓の基本ステージサイズは 16:9 寄りで FF14 UI 的に下部にログを置く想定。
// 牌のサイズは Issue #4 で確定するが、初期値だけ仮置きしておく。

export const STAGE_WIDTH = 1280
export const STAGE_HEIGHT = 720

// 卓背景: 深緑 (麻雀卓の典型色)。フェルトの色は #006400 を一段落として #003300 寄り。
export const TABLE_BG_COLOR = 0x003300
export const TABLE_BORDER_COLOR = 0x121212
export const TABLE_FELT_INNER_COLOR = 0x0d2f1d
export const TABLE_GLOW_COLOR = 0x7a0f16
export const PANEL_BG_COLOR = 0x111111
export const PANEL_BORDER_COLOR = 0x8f6a2f
export const PANEL_ACCENT_COLOR = 0xd4b06a
export const DISCARD_SLOT_COLOR = 0x1f3f2a
export const TEXT_PRIMARY_COLOR = 0xf3e7c7
export const TEXT_MUTED_COLOR = 0xb8aa8c
export const TEXT_DANGER_COLOR = 0xc93a3a
export const TURN_GLOW_COLOR = 0xffd166
export const SHADOW_COLOR = 0x000000
export const EVENT_LOG_LIMIT = 12
export const EVENT_LOG_VISIBLE_COUNT = 4

// 牌の基本サイズ (Issue #4 で createTileGraphics が参照)
export const TILE = {
  width: 50,
  height: 70,
  // 牌面のベース色 (アイボリー寄り)
  faceColor: 0xfaf3e0,
  // 牌面の縁取り
  edgeColor: 0x4a4a4a,
  // 角丸の半径
  cornerRadius: 6,
  // 通常文字色 (萬子・字牌)
  textColor: 0x1a1a1a,
  // 索子 (緑系)
  souColor: 0x117733,
  // 筒子 (青系)
  pinColor: 0x1e4e8c,
  // 赤ドラ
  redTextColor: 0xc1121f,
  // 裏向き (背中) 色
  backColor: 0x1e4e8c,
} as const
