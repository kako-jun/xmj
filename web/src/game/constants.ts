// xmj Web 表示まわりの定数。
//
// 麻雀卓の基本ステージサイズは 16:9 寄りで FF14 UI 的に下部にログを置く想定。
// 牌のサイズは Issue #4 で確定するが、初期値だけ仮置きしておく。

export const STAGE_WIDTH = 1280
export const STAGE_HEIGHT = 720

// 卓背景: 深緑 (麻雀卓の典型色)。フェルトの色は #006400 を一段落として #003300 寄り。
export const TABLE_BG_COLOR = 0x003300

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
