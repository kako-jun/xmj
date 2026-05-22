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
import type { GameState, MeldGroup, PlayerIndex, PlayerState, Tile } from './types'
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
 * 副露 1 組分の Container を作る (#83 副露表示)。
 *
 * ローカル座標は (0,0) = 左端の縦中心 (= 横並びの牌の中央 y) として組む。
 * 牌は左から右へ並び、`claimedIndex` の牌だけ 90° 回転して横向き (= 取った牌)。
 * - chi / pon / minkan: claimed 牌の表示位置は `fromOffset` で決まる
 *     1 (下家)  → meld の **右端**
 *     2 (対面)  → meld の **中央**
 *     3 (上家)  → meld の **左端**
 * - ankan: 4 枚すべて表示、両端 2 枚が表向き、中 2 枚が裏向き、回転なし
 * - kakan: minkan と同じ並びに加えて、claimed 牌の **上に** 4 枚目を横向きで重ねる
 *
 * @param meld 副露データ
 * @param scale 牌の表示倍率 (自家は 1.0、CPU は TILE.cpuHandScale)
 */
export const createMeldGroup = (meld: MeldGroup, scale: number): Container => {
  const root = new Container()
  root.label = `meld-${meld.kind}`

  const tileW = TILE.width * scale
  const tileH = TILE.height * scale

  /** 表向き牌スプライト (Container)。tile はそのまま createTileGraphics に渡す。 */
  const makeFace = (tile: Tile): Container => {
    const sprite = createTileGraphics(tile)
    sprite.scale.set(scale)
    return sprite
  }
  /** 裏向き牌スプライト (Container)。 */
  const makeBack = (): Container => {
    const sprite = createTileBackGraphics()
    sprite.scale.set(scale)
    return sprite
  }

  // 暗槓は特別扱い (claimed なし、中 2 枚を裏向き)
  if (meld.kind === 'ankan') {
    const tiles = meld.tiles
    if (tiles.length === 0) return root
    let cursorX = 0
    for (let i = 0; i < tiles.length; i++) {
      const isBack = i === 1 || i === 2
      const sprite = isBack ? makeBack() : makeFace(tiles[i])
      sprite.x = cursorX
      sprite.y = -tileH / 2
      root.addChild(sprite)
      cursorX += tileW
    }
    return root
  }

  // chi / pon / minkan / kakan: claimed 牌だけ横向き (90° 回転)
  // 横向き牌のローカル bbox は (tileH 横, tileW 縦) — 元の縦長を寝かせる。
  //
  // 並べる順序:
  //   - 非 claimed タイルを「tiles 配列順序」で残し、`sidewaysPos` の位置に claimed を置く。
  //   - fromOffset で「sideways position」を決める:
  //       3 (上家から) → 左端 / 2 (対面から) → 中央 / 1 (下家から) → 右端
  //     fromOffset === 0 は自家からの鳴き = 通常ありえない (加槓の鳴き元は元 Pon の向きを使う)
  //     ので、安全側で fromOffset === 1 と同じ「右端 sideways」に倒す。
  //
  // tiles の長さ (chi/pon=3、minkan=4、kakan=4) を考慮する:
  //   - chi/pon: 3 枚並び。claimed 1 枚 sideways + 残り 2 枚 face up。
  //   - minkan : 4 枚並び。claimed 1 枚 sideways + 残り 3 枚 face up (stacked なし)。
  //   - kakan  : 3 枚並び + stacked。minkan と同じ並び (claimed sideways) に
  //              加えて、claimed の上に 4 枚目を横向きで重ねる。
  //              (= 元 Pon の上に「加えた 1 枚」を乗せる慣習表現)

  const claimedIdx = meld.claimedIndex ?? 0
  const fromOffset = meld.fromOffset ?? 1

  const claimedTile = meld.tiles[claimedIdx] ?? meld.tiles[0]
  const nonClaimedTiles: Tile[] = []
  meld.tiles.forEach((t, i) => {
    if (i === claimedIdx) return
    nonClaimedTiles.push(t)
  })
  // minkan は 4 枚並び (face/face/face/sideways 等)、kakan は 3 枚並び + stacked。
  // useStack は「claimed の上にもう 1 枚重ねるかどうか」(= kakan のみ)。
  const useStack = meld.kind === 'kakan'
  const slotCount = meld.kind === 'minkan' ? 4 : 3
  // kakan の場合: 3 スロット並びに使う non-claimed は 2 枚 (3 枚目は stack 用)
  // minkan の場合: 4 スロット並びに使う non-claimed は 3 枚すべて
  // chi/pon の場合: 3 スロット並びに使う non-claimed は 2 枚すべて
  const baseTiles: Tile[] = useStack ? nonClaimedTiles.slice(0, 2) : nonClaimedTiles

  // sideways position index (0 = 左端 / 1 = 中央 / 2 = 右端 / 3 = 4 スロット最右端)
  // 3 スロット (chi/pon/kakan) は 0/1/2 のいずれか。
  // 4 スロット (minkan) も同じく上家=0/対面=1/下家=2 を使い、それ以外 (fromOffset===0等) は最右の 3。
  // fromOffset === 0 は通常ありえないが、安全側で「下家から鳴いた相当」(= 右端) に倒す。
  const sidewaysPos: 0 | 1 | 2 | 3 =
    fromOffset === 3
      ? 0
      : fromOffset === 2
        ? 1
        : slotCount === 4
          ? // minkan で fromOffset=1 (下家) は最右端 (= スロット 3)
            3
          : 2

  // 並び順を決める。slotCount スロットに base (slotCount-1) 枚 + claimed 1 枚 (sideways) を配置。
  // baseTiles の順序はそのまま (左→右で詰める)。
  const slots: Array<{ kind: 'face' | 'sideways'; tile: Tile }> = []
  let baseCursor = 0
  for (let pos = 0; pos < slotCount; pos++) {
    if (pos === sidewaysPos) {
      slots.push({ kind: 'sideways', tile: claimedTile })
    } else {
      const t = baseTiles[baseCursor] ?? claimedTile
      baseCursor += 1
      slots.push({ kind: 'face', tile: t })
    }
  }

  // 横向き牌の幅 = 元の高さ * scale (= tileH)。縦向き牌の幅 = tileW。
  // 横向き牌の縦寸 = tileW。縦向き牌の縦寸 = tileH。
  // base line は「縦向き牌の底辺」に合わせる (= y = 0 が底、上が -tileH)。
  // 横向き牌は底辺合わせで y = -tileW (横向きの高さ)。
  let cursorX = 0
  const stackTargets: Array<{ x: number; sidewaysWidth: number }> = []
  for (const slot of slots) {
    if (slot.kind === 'face') {
      const sprite = makeFace(slot.tile)
      sprite.x = cursorX
      sprite.y = -tileH
      root.addChild(sprite)
      cursorX += tileW
    } else {
      // sideways: 90° 回転 (pivot を tile 中心に合わせる)
      const sprite = makeFace(slot.tile)
      // 元の bbox は (TILE.width, TILE.height) — pivot を中心に。
      // pivot を中心にすると scale 後の中心が (0,0) になり、回転後の中心も維持される。
      sprite.pivot.set(TILE.width / 2, TILE.height / 2)
      sprite.rotation = Math.PI / 2
      // 回転後の bbox 中心が (cursorX + tileH/2, -tileW/2) になるよう配置 (底辺合わせ)。
      sprite.x = cursorX + tileH / 2
      sprite.y = -tileW / 2
      root.addChild(sprite)
      const sidewaysX = cursorX
      cursorX += tileH
      if (useStack) {
        stackTargets.push({ x: sidewaysX, sidewaysWidth: tileH })
      }
    }
  }

  // 加槓 / 大明槓の stacked タイル (claimed の上に 4 枚目を横向きで重ねる)
  if (useStack && stackTargets.length > 0) {
    const target = stackTargets[0]
    // 4 枚目の tile = meld.tiles のうち、まだ使われてない 1 枚 (= nonClaimedTiles[2])
    const stackTile: Tile = nonClaimedTiles[2] ?? claimedTile
    const sprite = makeFace(stackTile)
    sprite.pivot.set(TILE.width / 2, TILE.height / 2)
    sprite.rotation = Math.PI / 2
    sprite.x = target.x + target.sidewaysWidth / 2
    // claimed の上にもう 1 段 (-tileW ぶん上にずらす)
    sprite.y = -tileW / 2 - tileW
    root.addChild(sprite)
  }

  return root
}

const MELD_GAP = 8

/**
 * 副露 1 グループの占有横幅 (scale 適用済み)。
 *   - chi / pon / kakan: 横向き 1 枚 (= tileH) + 縦向き 2 枚 (= tileW * 2)、3 スロット並び。
 *     kakan は claimed の上に stack するので横幅は増えない (depth が増えるだけ)。
 *   - minkan: 横向き 1 枚 (= tileH) + 縦向き 3 枚 (= tileW * 3)、4 スロット並び。
 *   - ankan: 縦向き 4 枚 (= tileW * 4)。
 */
const meldGroupWidth = (meld: MeldGroup, scale: number): number => {
  const tileW = TILE.width * scale
  const tileH = TILE.height * scale
  if (meld.kind === 'ankan') return tileW * 4
  if (meld.kind === 'minkan') return tileW * 3 + tileH
  return tileW * 2 + tileH
}

/** 副露 row の合計横幅 (gap 込み)。 */
const meldRowWidth = (melds: MeldGroup[], scale: number): number => {
  if (melds.length === 0) return 0
  return melds.reduce((sum, m) => sum + meldGroupWidth(m, scale), 0) + (melds.length - 1) * MELD_GAP
}

/**
 * 副露 row が `maxWidth` に収まる最大 scale を求める。
 * 上限 `preferredScale` を超えない範囲で 0.4 まで段階的に下げる。
 * 下限 0.4 は「これ以上小さくすると牌の絵柄が判別できない」実用最小値。
 */
const fitMeldScale = (
  melds: MeldGroup[],
  preferredScale: number,
  maxWidth: number
): number => {
  if (melds.length === 0) return preferredScale
  let scale = preferredScale
  while (scale > 0.4 && meldRowWidth(melds, scale) > maxWidth) {
    scale -= 0.05
  }
  return Math.max(scale, 0.4)
}

/**
 * 副露 row の **depth (横向き牌 1 枚ぶん高さ + stack)** が `maxDepth` に収まる
 * 最大 scale を返す。CPU 副露が stage 外側余白に収まるかを担保するために使う。
 *
 * depth 必要量は基本 `tileH = TILE.height * scale`、kakan を含むなら `tileW * 2`。
 * すべて scale に比例するので、最大の depth-per-scale から線形に逆算できる。
 *
 * @param melds 対象 meld 群
 * @param preferredScale 上限 scale
 * @param maxDepth 許容される depth (px)
 */
const fitMeldDepthScale = (
  melds: MeldGroup[],
  preferredScale: number,
  maxDepth: number
): number => {
  if (melds.length === 0) return preferredScale
  // depth = scale * depthPerScale。最大の depthPerScale を取る meld に合わせる。
  // - 通常 meld: depthPerScale = TILE.height
  // - kakan を含む: depthPerScale = max(TILE.height, TILE.width * 2)
  const hasKakan = melds.some(m => m.kind === 'kakan')
  const depthPerScale = hasKakan ? Math.max(TILE.height, TILE.width * 2) : TILE.height
  const maxAllowedScale = maxDepth / depthPerScale
  return Math.max(0.4, Math.min(preferredScale, maxAllowedScale))
}

/**
 * 1 プレイヤー分の副露ブロック (複数 meld を横並びにしたもの) を作る (#83 副露表示)。
 *
 * ローカル座標は (0,0) = 左端・縦中心。meld 間に `MELD_GAP` の空きを入れる。
 * 結果 Container は handRow と同じローカル系に乗せられる前提。
 */
const createMeldRow = (melds: MeldGroup[], scale: number): Container => {
  const row = new Container()
  row.label = 'meld-row'
  if (melds.length === 0) return row
  let cursorX = 0
  for (const meld of melds) {
    const group = createMeldGroup(meld, scale)
    group.x = cursorX
    row.addChild(group)
    cursorX += meldGroupWidth(meld, scale) + MELD_GAP
  }
  return row
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
  // CPU 手牌の卓中心からの距離 (px)。session505 で 304 → 280 に下げ、CPU 手牌の
  // 外側に副露 row (sideways 牌の縦寸 = TILE.height * cpuHandScale ≈ 39.2px) が
  // 安全に収まる余白を作っている (旧 304 では stage 端から ~36px しか取れず、
  // sideways 牌が約 11px はみ出ていた)。
  const cpuHandBaseline = STAGE_HEIGHT / 2 + 280

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

  // #83 副露 (鳴き) ブロック。
  // 配置方針: 「どの牌とも重ならない」ことを最優先。
  //   - 自家 (offset 0): 手牌の下、stage 下辺と手牌底辺の間の空き帯に、右端揃え。
  //   - CPU (offset 1/2/3): 手牌の外側 (stage 外周方向) の余白に、手牌方向と平行で。
  // 余白幅に応じて scale を自動縮小する。
  const STAGE_EDGE_MARGIN = 12
  const preferredMeldScale = player.isCPU ? TILE.cpuHandScale : 0.8
  const meldList = player.melds ?? []
  const meldRow = new Container()
  meldRow.label = 'meld-row'

  // 各 offset の配置・回転をまとめて定義。座標は卓中心 (TABLE_CENTER_X, TABLE_CENTER_Y) 基準。
  switch (offset) {
    case 0: {
      // 下 (自家): 回転なし
      discardBlock.x = TABLE_CENTER_X - DISCARD_BLOCK_WIDTH / 2
      discardBlock.y = TABLE_CENTER_Y + DISCARD_INNER_MARGIN
      handRow.x = TABLE_CENTER_X
      handRow.y = handBaseline
      // 自家副露: 手牌の下に並べ、stage 右端寄せ。手牌底辺 (handBaseline + TILE.height/2)
      // から下に 8px、stage 下端より上に STAGE_EDGE_MARGIN 余白。
      const handBottom = handBaseline + TILE.height / 2
      const availableW = STAGE_WIDTH - 2 * STAGE_EDGE_MARGIN
      const meldScale = fitMeldScale(meldList, preferredMeldScale, availableW)
      const builtRow = createMeldRow(meldList, meldScale)
      const usedW = meldRowWidth(meldList, meldScale)
      builtRow.x = STAGE_WIDTH - STAGE_EDGE_MARGIN - usedW
      builtRow.y = handBottom + 8 + TILE.height * meldScale
      meldRow.addChild(builtRow)
      break
    }
    case 1: {
      // 右 (下家): -90° 回転
      discardBlock.rotation = -Math.PI / 2
      discardBlock.x = TABLE_CENTER_X + DISCARD_INNER_MARGIN
      discardBlock.y = TABLE_CENTER_Y + DISCARD_BLOCK_WIDTH / 2
      handRow.rotation = -Math.PI / 2
      handRow.x = cpuHandBaseline
      handRow.y = TABLE_CENTER_Y
      // CPU 1 副露: 手牌の外側 (stage 右端側、x が大きい方) に並べる。
      // 手牌の右端 (world x): handRow.x + TILE.height * cpuHandScale / 2
      // 並びの方向: 手牌と平行 (= world y 方向)、scale はその回廊幅 (stage 高さ) に収まる範囲。
      const handOuter = handRow.x + (TILE.height * TILE.cpuHandScale) / 2
      const availableW = STAGE_HEIGHT - 2 * STAGE_EDGE_MARGIN
      // depth = sideways 牌の縦寸 + 8px gap が、handOuter から stage 右端の余白に収まる。
      const outerSpace = STAGE_WIDTH - handOuter - STAGE_EDGE_MARGIN - 8
      const widthFitted = fitMeldScale(meldList, preferredMeldScale, availableW)
      const meldScale = fitMeldDepthScale(meldList, widthFitted, outerSpace)
      const builtRow = createMeldRow(meldList, meldScale)
      builtRow.rotation = -Math.PI / 2
      // builtRow の local +x は world -y。右端 (= world y 最大) から並べたいので、
      // builtRow の local 原点を「world (handOuter + tileH*scale + 8, STAGE_HEIGHT - margin)」
      // 相当に置く。回転後、local x 進行は world -y。
      builtRow.x = handOuter + 8 + TILE.height * meldScale
      builtRow.y = STAGE_HEIGHT - STAGE_EDGE_MARGIN
      meldRow.addChild(builtRow)
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
      // CPU 2 副露: 手牌の上 (stage 上端側) に並べる。
      // 手牌の上端 (world y): handRow.y - TILE.height * cpuHandScale / 2
      const handOuter = handRow.y - (TILE.height * TILE.cpuHandScale) / 2
      const availableW = STAGE_WIDTH - 2 * STAGE_EDGE_MARGIN
      const outerSpace = handOuter - STAGE_EDGE_MARGIN - 8
      const widthFitted = fitMeldScale(meldList, preferredMeldScale, availableW)
      const meldScale = fitMeldDepthScale(meldList, widthFitted, outerSpace)
      const builtRow = createMeldRow(meldList, meldScale)
      const usedW = meldRowWidth(meldList, meldScale)
      builtRow.rotation = Math.PI
      // 180° 回転後、local +x → world -x。左端 (stage 左端 margin) から並べたいので
      // builtRow の local 原点を「world 右上方向」に置く。
      builtRow.x = STAGE_EDGE_MARGIN + usedW
      builtRow.y = handOuter - 8 - TILE.height * meldScale
      meldRow.addChild(builtRow)
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
      // CPU 3 副露: 手牌の左 (stage 左端側) に並べる。
      const handOuter = handRow.x - (TILE.height * TILE.cpuHandScale) / 2
      const availableW = STAGE_HEIGHT - 2 * STAGE_EDGE_MARGIN
      const outerSpace = handOuter - STAGE_EDGE_MARGIN - 8
      const widthFitted = fitMeldScale(meldList, preferredMeldScale, availableW)
      const meldScale = fitMeldDepthScale(meldList, widthFitted, outerSpace)
      const builtRow = createMeldRow(meldList, meldScale)
      builtRow.rotation = Math.PI / 2
      // 90° 回転後、local +x → world +y。stage 上端 margin から並べたいので
      // builtRow の local 原点を「world (handOuter - tileH*scale - 8, STAGE_EDGE_MARGIN)」相当に。
      builtRow.x = handOuter - 8 - TILE.height * meldScale
      builtRow.y = STAGE_EDGE_MARGIN
      meldRow.addChild(builtRow)
      break
    }
  }

  root.addChild(discardBlock)
  root.addChild(handRow)
  root.addChild(meldRow)
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
