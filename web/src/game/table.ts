// 卓 (Pixi) シーン。
//
// 方針:
//   - 正方形ステージ (STAGE_WIDTH = STAGE_HEIGHT) の中央に卓を描く
//   - 上下左右 4 プレイヤーの手牌と河を、卓中心を軸に厳密対称配置する
//   - 牌そのものと卓の felt 以外、文字情報は一切描画しない (htmlUi 側に出す)
//
// 河 (捨て牌) は 6 列 × 3 行 = 18 マス。各プレイヤーのブロックは、卓中心から
// `DISCARD_INNER_MARGIN` の距離に内縁を持つ。これにより 4 プレイヤーの河が必ず
// 等距離・対称になり、中央の空き地に被らない。

import { Container, Graphics, Text, TextStyle } from 'pixi.js'
import {
  DISCARD_BLOCK_HEIGHT,
  DISCARD_BLOCK_WIDTH,
  DISCARD_INNER_MARGIN,
  DISCARD_ROWS,
  DISCARD_COLS,
  DISCARD_SLOT_COLOR,
  PANEL_ACCENT_COLOR,
  STAGE_HEIGHT,
  STAGE_WIDTH,
  TABLE_BG_COLOR,
  TABLE_CENTER_X,
  TABLE_CENTER_Y,
  TABLE_FELT_INNER_COLOR,
  TABLE_GLOW_COLOR,
  TILE,
  TURN_GLOW_COLOR,
} from './constants'
import { createTileBackGraphics, createTileGraphics } from './tile'
import type { GameState, PlayerIndex, PlayerState, Tile } from './types'
import { tileToCuiCode } from './types'

export interface TableSceneOptions {
  humanPlayerIndex?: PlayerIndex
  selectedHandIndex?: number | null
  interactiveHandPlayerId?: PlayerIndex | null
  /**
   * 自家が直近で引いた牌。指定があると、自家の手牌のうちこの牌1枚を右端に
   * 小さく分離して描画する (ツモ牌の視認性を上げる)。
   */
  justDrawnTile?: Tile | null
  /**
   * 鳴き判定モーダル中に「対象になっている直前打牌」を川の中で強調表示するかどうか。
   * 中央表示は廃止、河の該当牌位置を黄色いハローで光らせる。
   */
  highlightLastDiscard?: boolean
  onHandTileTap?: (index: number) => void
}

const addStageBackdrop = (root: Container): void => {
  const backdrop = new Graphics()
  backdrop.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: 0x050505 })
  backdrop
    .circle(TABLE_CENTER_X, TABLE_CENTER_Y, STAGE_WIDTH * 0.42)
    .fill({ color: TABLE_GLOW_COLOR, alpha: 0.06 })
  root.addChild(backdrop)
}

/**
 * 正方形の felt を描く。外枠 → felt → 中央の薄い影、と 3 段重ねる。
 */
const createTableSurface = (): Container => {
  const table = new Container()
  table.label = 'table-surface'

  // 金色フレーム廃止 — 緑のフェルトをステージ端近くまで広げる
  const innerMargin = 16
  const innerSize = STAGE_WIDTH - innerMargin * 2
  const inner = new Graphics()
  inner
    .roundRect(innerMargin, innerMargin, innerSize, innerSize, 24)
    .fill({ color: TABLE_BG_COLOR })
  table.addChild(inner)

  // 中央の薄い felt。河に囲まれた領域を視覚的に示す。
  const centerSize = (DISCARD_INNER_MARGIN - 4) * 2
  const center = new Graphics()
  center
    .roundRect(
      TABLE_CENTER_X - centerSize / 2,
      TABLE_CENTER_Y - centerSize / 2,
      centerSize,
      centerSize,
      18
    )
    .fill({ color: TABLE_FELT_INNER_COLOR })
  table.addChild(center)

  return table
}

const getRelativePlayer = (
  state: GameState,
  humanPlayerIndex: PlayerIndex,
  offset: number
): PlayerState => state.players[((humanPlayerIndex + offset) % 4) as PlayerIndex]

const createDiscardBlock = (
  player: PlayerState,
  highlightLast: boolean = false
): Container => {
  // ローカル座標は「(0,0) が河ブロックの左上、6 列×3 行 (col×row → x=col*pitch, y=row*pitch)」
  // で、上端の row=0 が中央に最も近い。回転 & 配置は呼び出し側で行う。
  const discards = new Container()
  discards.label = `discards-${player.id}`

  const tileScale = TILE.discardScale
  const colPitch = TILE.discardColPitch
  const rowPitch = TILE.discardRowPitch
  const slotW = TILE.width * tileScale
  const slotH = TILE.height * tileScale
  const TOTAL_SLOTS = DISCARD_COLS * DISCARD_ROWS

  for (let index = 0; index < TOTAL_SLOTS; index++) {
    const col = index % DISCARD_COLS
    const row = Math.floor(index / DISCARD_COLS)
    const filled = index < player.discards.length
    const slot = new Graphics()
    slot
      .roundRect(col * colPitch, row * rowPitch, slotW, slotH, 5)
      .fill({ color: DISCARD_SLOT_COLOR, alpha: filled ? 0.18 : 0.32 })
      .stroke({ color: PANEL_ACCENT_COLOR, width: 1, alpha: 0.12 })
    discards.addChild(slot)
  }

  const lastIndex = player.discards.length - 1
  player.discards.forEach((tile, index) => {
    const col = index % DISCARD_COLS
    const row = Math.floor(index / DISCARD_COLS)
    const sprite = createTileGraphics(tile)
    sprite.scale.set(tileScale)
    sprite.x = col * colPitch
    sprite.y = row * rowPitch
    // 鳴き判定中: 直前打牌 (= 河の末尾) を黄色いハローで光らせて「これに対して
    // 鳴くか」のアンカーにする。中央に複製を出すよりこちらの方が誤認しにくい。
    if (highlightLast && index === lastIndex) {
      const halo = new Graphics()
      halo
        .roundRect(-4, -4, TILE.width + 8, TILE.height + 8, TILE.cornerRadius + 2)
        .fill({ color: TURN_GLOW_COLOR, alpha: 0.22 })
        .stroke({ color: TURN_GLOW_COLOR, width: 2, alpha: 0.95 })
      sprite.addChildAt(halo, 0)
    }
    discards.addChild(sprite)
  })

  return discards
}

/**
 * 自家の手牌から「直近ツモ牌」のインデックスを 1 つ特定する。
 * Rust 側 sort 後にどの位置に来たかを toCuiCode の一致で探す。
 * 一致候補が複数 (=同じ牌が手中に複数) ある場合は右端寄りを採用する。
 * justDrawnTile が null なら -1。
 */
const findJustDrawnIndex = (hand: Tile[], justDrawnTile: Tile | null | undefined): number => {
  if (!justDrawnTile) return -1
  const key = tileToCuiCode(justDrawnTile)
  for (let i = hand.length - 1; i >= 0; i--) {
    if (tileToCuiCode(hand[i]) === key) return i
  }
  return -1
}

const createHandRow = (player: PlayerState, options: TableSceneOptions = {}): Container => {
  // ローカル座標: (0,0) を手牌の中心とし、左右に等間隔で並べる。
  // 自家は表向き、CPU は裏向き。
  // 自家かつ justDrawnTile 指定があれば、その 1 枚を抜いて末尾に再配置し、
  // 本体と少し離して描く (麻雀 UI 慣習: ツモ牌は右端に分離)。
  const hand = new Container()
  hand.label = `hand-${player.id}`

  const isCpu = player.isCPU
  const tileScale = isCpu ? TILE.cpuHandScale : 1
  const spacing = isCpu ? TILE.cpuHandSpacing : TILE.handSpacing

  // ツモ牌分離の対象は「自家・表向き・justDrawnTile 指定あり」のときだけ。
  const drawnIdx = isCpu ? -1 : findJustDrawnIndex(player.hand, options.justDrawnTile)
  const reordered: Tile[] = player.hand.slice()
  let tsumoTile: Tile | null = null
  if (drawnIdx >= 0) {
    tsumoTile = reordered.splice(drawnIdx, 1)[0]
  }

  const total = reordered.length + (tsumoTile ? 1 : 0)
  const tsumoGap = tsumoTile ? spacing * 0.45 : 0 // 本体と分離する間隔
  // 仮想 total スロット幅で中心揃え (tsumoTile はオフセット tsumoGap だけ右にずらす)
  const totalWidth = (total - 1) * spacing + tsumoGap
  const leftEdgeX = -totalWidth / 2

  const placeTile = (tile: Tile, slotIndex: number, isTsumoTile: boolean): void => {
    const sprite = isCpu ? createTileBackGraphics() : createTileGraphics(tile)
    sprite.scale.set(tileScale)
    const extraGap = isTsumoTile ? tsumoGap : 0
    sprite.x = leftEdgeX + slotIndex * spacing + extraGap - (TILE.width * tileScale) / 2
    sprite.y = -(TILE.height * tileScale) / 2

    // 元 hand 配列での index (タップハンドラに渡す)。Rust 側 hand と一致させる。
    const originalIndex = isTsumoTile
      ? drawnIdx
      : (() => {
          // reordered[slotIndex] が player.hand のどの位置か。drawnIdx を抜いた配列なので、
          // slotIndex < drawnIdx ならそのまま、>= drawnIdx なら +1。
          if (drawnIdx < 0) return slotIndex
          return slotIndex < drawnIdx ? slotIndex : slotIndex + 1
        })()

    const isInteractive =
      options.interactiveHandPlayerId === player.id && typeof options.onHandTileTap === 'function'
    const isSelected = options.selectedHandIndex === originalIndex && isInteractive
    if (isSelected) {
      const glow = new Graphics()
      glow
        .roundRect(-4, -6, TILE.width + 8, TILE.height + 8, TILE.cornerRadius + 2)
        .fill({ color: TURN_GLOW_COLOR, alpha: 0.16 })
        .stroke({ color: TURN_GLOW_COLOR, width: 2, alpha: 0.9 })
      sprite.addChildAt(glow, 0)
      sprite.y -= 14
      sprite.scale.set(tileScale * 1.04)
    }

    if (isInteractive) {
      sprite.label = `${sprite.label ?? 'tile'}-${originalIndex}`
      sprite.eventMode = 'static'
      sprite.cursor = 'pointer'
      sprite.on('pointertap', () => {
        options.onHandTileTap?.(originalIndex)
      })
    }

    hand.addChild(sprite)
  }

  reordered.forEach((tile, slotIndex) => placeTile(tile, slotIndex, false))
  if (tsumoTile) {
    placeTile(tsumoTile, reordered.length, true)
  }

  return hand
}

/**
 * プレイヤー i の手牌 + 河を配置する。
 *   offset 0 = 自家 (下)、1 = 下家 (右)、2 = 対面 (上)、3 = 上家 (左)。
 * 配置は卓中心軸対称: 下と上は y 反転、左右は x 反転 + 90° 回転。
 */
const addSeatLayout = (
  root: Container,
  state: GameState,
  humanPlayerIndex: PlayerIndex,
  offset: 0 | 1 | 2 | 3,
  options: TableSceneOptions
): void => {
  const player = getRelativePlayer(state, humanPlayerIndex, offset)

  // 河ブロックの内縁から卓中心までの距離 = DISCARD_INNER_MARGIN
  // 自家河は中央から下に DISCARD_INNER_MARGIN ぶん離した位置に上端を置く。
  // 自家手牌は河の下端 + tile 半身 + 余白を確保して川との重なりを防ぐ。
  // CPU の手牌は卓の対称ラインから少し外側 (overlap 回避のため自家より外) に置く。
  const handBaseline =
    STAGE_HEIGHT / 2 + DISCARD_INNER_MARGIN + DISCARD_BLOCK_HEIGHT + TILE.height / 2 + 12
  const cpuHandBaseline = STAGE_HEIGHT / 2 + 304

  // 鳴き対象 (直前打牌) の強調は、最後に lastDiscard を捨てたプレイヤーの河だけで行う。
  // wasm bridge は lastDiscarder を直接渡してこないので、河の末尾と lastDiscard を
  // 突き合わせて推定する (普通は唯一の一致点になる)。
  const highlightLast =
    options.highlightLastDiscard === true &&
    state.lastDiscard !== null &&
    player.discards.length > 0 &&
    tileToCuiCode(player.discards[player.discards.length - 1]) ===
      tileToCuiCode(state.lastDiscard)
  const discardBlock = createDiscardBlock(player, highlightLast)
  const handRow = createHandRow(player, offset === 0 ? options : {})

  // 各 offset の配置・回転をまとめて定義。座標は卓中心 (TABLE_CENTER_X, TABLE_CENTER_Y) 基準。
  switch (offset) {
    case 0: {
      // 下 (自家): 回転なし
      discardBlock.x = TABLE_CENTER_X - DISCARD_BLOCK_WIDTH / 2
      discardBlock.y = TABLE_CENTER_Y + DISCARD_INNER_MARGIN
      handRow.x = TABLE_CENTER_X
      handRow.y = handBaseline
      break
    }
    case 1: {
      // 右 (下家): -90° 回転。CPU は別 baseline で外側へ
      discardBlock.rotation = -Math.PI / 2
      discardBlock.x = TABLE_CENTER_X + DISCARD_INNER_MARGIN
      discardBlock.y = TABLE_CENTER_Y + DISCARD_BLOCK_WIDTH / 2
      handRow.rotation = -Math.PI / 2
      handRow.x = cpuHandBaseline
      handRow.y = TABLE_CENTER_Y
      break
    }
    case 2: {
      // 上 (対面): 180° 回転
      discardBlock.rotation = Math.PI
      discardBlock.x = TABLE_CENTER_X + DISCARD_BLOCK_WIDTH / 2
      discardBlock.y = TABLE_CENTER_Y - DISCARD_INNER_MARGIN
      handRow.rotation = Math.PI
      handRow.x = TABLE_CENTER_X
      handRow.y = STAGE_HEIGHT - cpuHandBaseline
      break
    }
    case 3: {
      // 左 (上家): 90° 回転
      discardBlock.rotation = Math.PI / 2
      discardBlock.x = TABLE_CENTER_X - DISCARD_INNER_MARGIN
      discardBlock.y = TABLE_CENTER_Y - DISCARD_BLOCK_WIDTH / 2
      handRow.rotation = Math.PI / 2
      handRow.x = STAGE_HEIGHT - cpuHandBaseline
      handRow.y = TABLE_CENTER_Y
      break
    }
  }

  root.addChild(discardBlock)
  root.addChild(handRow)
}

/**
 * 卓シーンを生成する。
 * 文字描画 (点数・局・本場・ドラ表示・実況ログ・操作ボタン) は htmlUi.ts 側で行う。
 */
export const createTableScene = (
  state: GameState,
  options: TableSceneOptions = {}
): Container => {
  const humanPlayerIndex = options.humanPlayerIndex ?? 0
  const root = new Container()
  root.label = 'game-table'

  addStageBackdrop(root)
  root.addChild(createTableSurface())

  // 4 プレイヤーぶんの手牌 + 河を対称配置で並べる。
  for (const offset of [0, 1, 2, 3] as const) {
    addSeatLayout(root, state, humanPlayerIndex, offset, options)
  }

  // 親 (dealer) の前に「東」マーカーを置いて自風が一目で分かるようにする。
  // 親が東家・他家が南/西/北家の関係は dealer index + 自家からの相対位置で決まる。
  addDealerWindMarker(root, state, humanPlayerIndex)

  return root
}

/**
 * 親 (= 東家) の手牌上端付近に小さな「東」マーカーを置く。
 * 自家からの相対位置で 4 方位どこに出すかを決める。
 */
const addDealerWindMarker = (
  root: Container,
  state: GameState,
  humanPlayerIndex: PlayerIndex
): void => {
  const dealerOffset = ((state.dealer - humanPlayerIndex + 4) % 4) as 0 | 1 | 2 | 3
  const marker = new Container()
  marker.label = 'dealer-marker'

  const W = 30
  const H = 30
  const bg = new Graphics()
  bg.roundRect(0, 0, W, H, 4)
    .fill({ color: 0xffffff })
    .stroke({ color: 0x4a4a4a, width: 1 })
  marker.addChild(bg)

  const style = new TextStyle({
    fontFamily: '"Hiragino Mincho ProN", "Yu Mincho", "Noto Serif CJK JP", serif',
    fontSize: 22,
    fontWeight: '700',
    fill: 0x1a1a1a,
  })
  const text = new Text({ text: '東', style })
  text.anchor.set(0.5)
  text.x = W / 2
  text.y = H / 2 + 1
  marker.addChild(text)

  // 自家手牌の左外側、上家・下家・対面はそれぞれ手番表示と被らない卓内寄り。
  // 自家を 0 として時計回りに、(0=下) (1=右) (2=上) (3=左)
  const inset = 18
  switch (dealerOffset) {
    case 0:
      marker.x = TABLE_CENTER_X - DISCARD_BLOCK_WIDTH / 2 - W - inset
      marker.y = TABLE_CENTER_Y + DISCARD_INNER_MARGIN + DISCARD_BLOCK_HEIGHT - H
      break
    case 1:
      marker.x = TABLE_CENTER_X + DISCARD_INNER_MARGIN + DISCARD_BLOCK_HEIGHT - W
      marker.y = TABLE_CENTER_Y - DISCARD_BLOCK_WIDTH / 2 - H - inset
      break
    case 2:
      marker.x = TABLE_CENTER_X + DISCARD_BLOCK_WIDTH / 2 + inset
      marker.y = TABLE_CENTER_Y - DISCARD_INNER_MARGIN - DISCARD_BLOCK_HEIGHT
      break
    case 3:
      marker.x = TABLE_CENTER_X - DISCARD_INNER_MARGIN - DISCARD_BLOCK_HEIGHT
      marker.y = TABLE_CENTER_Y + DISCARD_BLOCK_WIDTH / 2 + inset
      break
  }
  root.addChild(marker)
}
