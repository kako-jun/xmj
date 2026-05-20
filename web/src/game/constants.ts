// xmj Web 表示まわりの定数。
//
// 麻雀卓の基本ステージサイズは 16:9。下辺右側に操作 UI をまとめ、
// スマホでも親指圏内で操作できる配置を狙う。

export const STAGE_WIDTH = 1280
export const STAGE_HEIGHT = 720

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
export const EVENT_LOG_LIMIT = 12
export const EVENT_LOG_VISIBLE_COUNT = 4

// 牌の基本サイズ。隣接牌との中心間距離は width 以上を確保し、重なりを避ける。
export const TILE = {
  width: 50,
  height: 70,
  // 手牌の隣接牌中心間ピッチ (>= width)
  handSpacing: 54,
  // 河 (捨牌) のグリッドピッチ
  discardColPitch: 36,
  discardRowPitch: 50,
  discardScale: 0.62,
  // 牌面のベース色 (アイボリー寄り)
  faceColor: 0xf3ead2,
  // 牌面の縁取り
  edgeColor: 0x4a4a4a,
  // 角丸の半径
  cornerRadius: 6,
  // 通常文字色 (萬子・字牌)
  textColor: 0x1a1a1a,
  // 索子 (緑系、彩度抑制)
  souColor: 0x2f6b3a,
  // 筒子 (青系、彩度抑制)
  pinColor: 0x365a85,
  // 赤ドラ
  redTextColor: 0xa83a3a,
  // 裏向き (背中) 色
  backColor: 0x33445e,
} as const
