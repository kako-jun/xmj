import { Container, Graphics, Text, TextStyle } from 'pixi.js'
import {
  DISCARD_SLOT_COLOR,
  PANEL_ACCENT_COLOR,
  PANEL_BORDER_COLOR,
  SHADOW_COLOR,
  STAGE_HEIGHT,
  STAGE_WIDTH,
  TABLE_BG_COLOR,
  TABLE_BORDER_COLOR,
  TABLE_FELT_INNER_COLOR,
  TABLE_GLOW_COLOR,
  TEXT_DANGER_COLOR,
  EVENT_LOG_VISIBLE_COUNT,
  TEXT_MUTED_COLOR,
  TEXT_PRIMARY_COLOR,
  TILE,
  TURN_GLOW_COLOR,
} from './constants'
import { createTileBackGraphics, createTileGraphics } from './tile'
import type { GameState, PlayerIndex, PlayerState } from './types'

const TABLE_CENTER_X = STAGE_WIDTH / 2
const TABLE_CENTER_Y = STAGE_HEIGHT / 2 - 12

const PLAYER_WIND = ['東', '南', '西', '北'] as const

export interface TableActionButton {
  key: string
  label: string
  enabled: boolean
  onTap: () => void
}

export interface TableSceneOptions {
  humanPlayerIndex?: PlayerIndex
  selectedHandIndex?: number | null
  interactiveHandPlayerId?: PlayerIndex | null
  onHandTileTap?: (index: number) => void
  actionButtons?: TableActionButton[]
  eventLog?: string[]
}

const makeText = (
  text: string,
  fontSize: number,
  fill: number,
  align: 'left' | 'center' | 'right' = 'left',
  fontWeight: 'normal' | 'bold' = 'normal'
): Text =>
  new Text({
    text,
    style: new TextStyle({
      fontFamily: '"Segoe UI", "Hiragino Sans", sans-serif',
      fontSize,
      fontWeight,
      fill,
      align,
    }),
  })

const addStageBackdrop = (root: Container): void => {
  const backdrop = new Graphics()
  backdrop.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: 0x050505 })
  backdrop
    .circle(TABLE_CENTER_X, TABLE_CENTER_Y, 420)
    .fill({ color: TABLE_GLOW_COLOR, alpha: 0.07 })
  root.addChild(backdrop)
}

const createTableSurface = (): Container => {
  const table = new Container()

  const outer = new Graphics()
  outer
    .roundRect(180, 70, 920, 580, 34)
    .fill({ color: TABLE_BORDER_COLOR })
    .stroke({ color: PANEL_BORDER_COLOR, width: 4 })
  table.addChild(outer)

  const inner = new Graphics()
  inner
    .roundRect(220, 110, 840, 500, 28)
    .fill({ color: TABLE_BG_COLOR })
    .stroke({ color: PANEL_ACCENT_COLOR, width: 2, alpha: 0.45 })
  table.addChild(inner)

  const center = new Graphics()
  center
    .roundRect(470, 235, 340, 250, 24)
    .fill({ color: TABLE_FELT_INNER_COLOR })
    .stroke({ color: PANEL_ACCENT_COLOR, width: 2, alpha: 0.4 })
  table.addChild(center)

  return table
}

const createInfoPanel = (state: GameState): Container => {
  const panel = new Container()
  panel.label = 'center-info'

  // 中央卓上の情報帯は枠を引かず、フェルトに直書きしたような見せ方にする。
  const bg = new Graphics()
  bg
    .roundRect(0, 0, 290, 190, 14)
    .fill({ color: SHADOW_COLOR, alpha: 0.22 })
  panel.addChild(bg)

  const title = makeText(`東${state.round}局`, 26, TEXT_PRIMARY_COLOR, 'center', 'bold')
  title.anchor.set(0.5, 0)
  title.x = 145
  title.y = 16
  panel.addChild(title)

  const subtitle = makeText('極限の配牌', 14, TEXT_MUTED_COLOR, 'center')
  subtitle.anchor.set(0.5, 0)
  subtitle.x = 145
  subtitle.y = 48
  panel.addChild(subtitle)

  const wall = makeText(`山牌 ${state.wall.length}枚`, 22, TEXT_PRIMARY_COLOR, 'center', 'bold')
  wall.anchor.set(0.5, 0)
  wall.x = 145
  wall.y = 78
  panel.addChild(wall)

  const doraLabel = makeText('ドラ表示', 14, TEXT_MUTED_COLOR)
  doraLabel.x = 22
  doraLabel.y = 122
  panel.addChild(doraLabel)

  state.doraIndicators.forEach((tile, idx) => {
    const sprite = createTileGraphics(tile)
    sprite.scale.set(0.72)
    sprite.x = 112 + idx * (TILE.width * 0.72 + 6)
    sprite.y = 114
    panel.addChild(sprite)
  })

  const tension = makeText('流れ: 張り詰め', 14, TEXT_DANGER_COLOR, 'right', 'bold')
  tension.anchor.set(1, 0)
  tension.x = 266
  tension.y = 156
  panel.addChild(tension)

  const lastDiscardLabel = makeText('直前打牌', 14, TEXT_MUTED_COLOR)
  lastDiscardLabel.x = 188
  lastDiscardLabel.y = 122
  panel.addChild(lastDiscardLabel)

  if (state.lastDiscard) {
    const sprite = createTileGraphics(state.lastDiscard)
    sprite.scale.set(0.72)
    sprite.x = 236
    sprite.y = 114
    panel.addChild(sprite)
  } else {
    const empty = makeText('なし', 14, TEXT_MUTED_COLOR)
    empty.x = 236
    empty.y = 138
    panel.addChild(empty)
  }

  panel.x = TABLE_CENTER_X - 145
  panel.y = TABLE_CENTER_Y - 95
  return panel
}

const getRelativePlayer = (
  state: GameState,
  humanPlayerIndex: PlayerIndex,
  offset: number
): PlayerState => state.players[((humanPlayerIndex + offset) % 4) as PlayerIndex]

// スコアバッジは「卓に貼り付いた札」ではなく、ステージ上に置いた小さな名札の
// イメージで枠を最小限にする。風牌の漢字を大きく出し、名前と点数はその右に流す。
const addSeatBadge = (player: PlayerState, x: number, y: number): Container => {
  const seatBadge = new Container()

  const wind = makeText(PLAYER_WIND[player.id], 26, PANEL_ACCENT_COLOR, 'center', 'bold')
  wind.anchor.set(0.5)
  wind.x = 18
  wind.y = 30
  seatBadge.addChild(wind)

  const name = makeText(player.name, 16, TEXT_PRIMARY_COLOR, 'left', 'bold')
  name.x = 40
  name.y = 10
  seatBadge.addChild(name)

  const score = makeText(`${player.score.toLocaleString()}`, 18, TEXT_MUTED_COLOR, 'left', 'bold')
  score.x = 40
  score.y = 32
  seatBadge.addChild(score)

  if (player.isRiichi) {
    const riichi = makeText('立直', 14, TEXT_DANGER_COLOR, 'left', 'bold')
    riichi.x = 130
    riichi.y = 12
    seatBadge.addChild(riichi)
  }

  seatBadge.x = x
  seatBadge.y = y
  return seatBadge
}

const createScoreBadges = (state: GameState, humanPlayerIndex: PlayerIndex): Container => {
  // バッジは「自家=右下の操作 UI に統合」「他家=ステージ四隅」に配置し、
  // 卓内に置かないことで手牌・河と重ならないようにする。
  const badges = new Container()
  badges.label = 'score-badges'
  // 上 (offset 2): ステージ上端中央
  badges.addChild(addSeatBadge(getRelativePlayer(state, humanPlayerIndex, 2), 546, 4))
  // 右 (offset 1): ステージ右上角、右手牌より外側
  badges.addChild(addSeatBadge(getRelativePlayer(state, humanPlayerIndex, 1), STAGE_WIDTH - 188 - 8, 4))
  // 左 (offset 3): ステージ左上角、左手牌より外側
  badges.addChild(addSeatBadge(getRelativePlayer(state, humanPlayerIndex, 3), 8, 4))
  // 自家 (offset 0): 操作 UI の上に小さく出す。createBottomArea 側で配置
  return badges
}

const createSelfScoreBadge = (player: PlayerState, x: number, y: number): Container =>
  addSeatBadge(player, x, y)

const createPlayerHand = (player: PlayerState, options: TableSceneOptions = {}): Container => {
  const hand = new Container()
  hand.label = `hand-${player.id}`

  // CPU 手牌は裏向きの装飾なので、小さめのスケール + 詰めた間隔にして
  // 卓の外にはみ出さないようにする。自家は読みやすさ優先で原寸 + 余裕を持たせる。
  const isCpu = player.isCPU
  const tileScale = isCpu ? 0.72 : 1
  const spacing = isCpu ? 38 : TILE.handSpacing

  const total = player.hand.length
  player.hand.forEach((tile, index) => {
    const sprite = player.isCPU ? createTileBackGraphics() : createTileGraphics(tile)
    sprite.scale.set(tileScale)
    sprite.x = (index - (total - 1) / 2) * spacing

    const isInteractive =
      options.interactiveHandPlayerId === player.id && typeof options.onHandTileTap === 'function'
    const isSelected = options.selectedHandIndex === index && isInteractive
    if (isSelected) {
      const glow = new Graphics()
      glow
        .roundRect(-4, -6, TILE.width + 8, TILE.height + 8, TILE.cornerRadius + 2)
        .fill({ color: TURN_GLOW_COLOR, alpha: 0.16 })
        .stroke({ color: TURN_GLOW_COLOR, width: 2, alpha: 0.9 })
      sprite.addChildAt(glow, 0)
      sprite.y = -18
      sprite.scale.set(1.04)
    }

    if (isInteractive) {
      sprite.label = `${sprite.label ?? 'tile'}-${index}`
      sprite.eventMode = 'static'
      sprite.cursor = 'pointer'
      sprite.on('pointertap', () => {
        options.onHandTileTap?.(index)
      })
    }

    hand.addChild(sprite)
  })

  return hand
}

const createPlayerDiscards = (player: PlayerState): Container => {
  const discards = new Container()
  discards.label = `discards-${player.id}`

  const maxCols = 6
  const tileScale = TILE.discardScale
  const colPitch = TILE.discardColPitch
  const rowPitch = TILE.discardRowPitch
  const slotW = TILE.width * tileScale
  const slotH = TILE.height * tileScale
  // 河の全 18 マスを必ず描画して「場」のサイズ感を出す。createTableScene は
  // 状態更新ごとに走るため、空マスを per-frame で 18 枚生成する分のコストは
  // 単純な Graphics なのでベンチ上問題なし (タイル本体の方が重い)。
  const TOTAL_SLOTS = 18
  for (let index = 0; index < TOTAL_SLOTS; index++) {
    const col = index % maxCols
    const row = Math.floor(index / maxCols)
    const filled = index < player.discards.length
    const slot = new Graphics()
    slot
      .roundRect(col * colPitch, row * rowPitch, slotW, slotH, 6)
      .fill({ color: DISCARD_SLOT_COLOR, alpha: filled ? 0.18 : 0.34 })
      .stroke({ color: PANEL_ACCENT_COLOR, width: 1, alpha: 0.14 })
    discards.addChild(slot)
  }

  player.discards.forEach((tile, index) => {
    const col = index % maxCols
    const row = Math.floor(index / maxCols)
    const sprite = createTileGraphics(tile)
    sprite.scale.set(tileScale)
    sprite.x = col * colPitch
    sprite.y = row * rowPitch
    discards.addChild(sprite)
  })

  return discards
}

// 手番マーカーは枠を持たず、小さな丸印 + 文字だけで存在感を主張しない。
const createTurnMarker = (label: string): Container => {
  const marker = new Container()
  marker.label = 'turn-marker'

  const dot = new Graphics()
  dot.circle(8, 17, 5).fill({ color: TURN_GLOW_COLOR })
  marker.addChild(dot)

  const text = makeText(label, 14, TURN_GLOW_COLOR, 'left', 'bold')
  text.x = 22
  text.y = 8
  marker.addChild(text)
  return marker
}

// 操作 UI はスマホの親指圏 (画面右下) に集約する。
// ボタンは縦積み、最小高さ 48px (タッチターゲット推奨値) を確保する。
const ACTION_AREA_WIDTH = 220
const ACTION_BUTTON_WIDTH = 188
const ACTION_BUTTON_HEIGHT = 52
const ACTION_BUTTON_GAP = 10
const ACTION_AREA_PADDING_X = 16
const ACTION_AREA_HEADER_HEIGHT = 30

const createActionArea = (buttons: TableActionButton[]): Container => {
  const area = new Container()
  area.label = 'action-area'

  const rows = Math.max(buttons.length, 1)
  const height =
    ACTION_AREA_HEADER_HEIGHT +
    rows * ACTION_BUTTON_HEIGHT +
    (rows - 1) * ACTION_BUTTON_GAP +
    16

  // 操作 UI はボタン自体が枠を持つので、外周パネルは薄い影だけにする。
  const bg = new Graphics()
  bg
    .roundRect(0, 0, ACTION_AREA_WIDTH, height, 14)
    .fill({ color: SHADOW_COLOR, alpha: 0.34 })
  area.addChild(bg)

  const title = makeText('行動', 14, TEXT_MUTED_COLOR, 'left', 'bold')
  title.x = ACTION_AREA_PADDING_X
  title.y = 10
  area.addChild(title)

  if (buttons.length === 0) {
    const empty = makeText('選択肢なし', 16, TEXT_MUTED_COLOR, 'left')
    empty.x = ACTION_AREA_PADDING_X
    empty.y = ACTION_AREA_HEADER_HEIGHT + 12
    area.addChild(empty)
    return area
  }

  buttons.forEach((button, index) => {
    const action = new Container()
    action.label = `action-button-${button.key}`
    action.x = ACTION_AREA_PADDING_X
    action.y = ACTION_AREA_HEADER_HEIGHT + index * (ACTION_BUTTON_HEIGHT + ACTION_BUTTON_GAP)

    const plate = new Graphics()
    plate
      .roundRect(0, 0, ACTION_BUTTON_WIDTH, ACTION_BUTTON_HEIGHT, 14)
      .fill({
        color: button.enabled ? 0x2a1d0d : 0x242422,
        alpha: button.enabled ? 0.96 : 0.78,
      })
      .stroke({
        color: button.enabled ? PANEL_ACCENT_COLOR : TEXT_MUTED_COLOR,
        width: 2,
        alpha: button.enabled ? 1 : 0.45,
      })
    action.addChild(plate)

    const label = makeText(
      button.label,
      18,
      button.enabled ? TEXT_PRIMARY_COLOR : TEXT_MUTED_COLOR,
      'center',
      'bold'
    )
    label.anchor.set(0.5)
    label.x = ACTION_BUTTON_WIDTH / 2
    label.y = ACTION_BUTTON_HEIGHT / 2
    action.addChild(label)

    if (button.enabled) {
      action.eventMode = 'static'
      action.cursor = 'pointer'
      action.on('pointertap', button.onTap)
    }

    area.addChild(action)
  })

  return area
}

const createBottomArea = (state: GameState, options: TableSceneOptions = {}): Container => {
  const humanPlayerIndex = options.humanPlayerIndex ?? 0
  const player = getRelativePlayer(state, humanPlayerIndex, 0)
  const area = new Container()
  area.label = 'bottom-area'

  // 縦の積み順 (上→下): 中央パネル → 自家河 → 自家手牌 → ステージ底
  // 牌の重なりを避けるため河と手牌の y 範囲を明示的にずらす。
  // 6 列 × discardColPitch(36) = 216px、3 行 × discardRowPitch(50) = 150px。
  const discards = createPlayerDiscards(player)
  discards.x = TABLE_CENTER_X - (6 * TILE.discardColPitch) / 2
  discards.y = 450
  area.addChild(discards)

  // 13 牌 × handSpacing(54) = 約 702px。中央寄せで右端は約 x=990、操作 UI と被らない。
  const hand = createPlayerHand(player, options)
  hand.x = TABLE_CENTER_X
  hand.y = 612
  area.addChild(hand)

  // 操作 UI は親指圏内 (右下) に集約。
  const actionArea = createActionArea(options.actionButtons ?? [])
  const actionX = STAGE_WIDTH - ACTION_AREA_WIDTH - 24
  const actionY = STAGE_HEIGHT - 24 - getActionAreaHeight(options.actionButtons ?? [])
  actionArea.x = actionX
  actionArea.y = actionY
  area.addChild(actionArea)

  // 自家の点数バッジは操作 UI の真上に配置 (4 隅ではなく親指圏内)。
  const selfBadge = createSelfScoreBadge(player, actionX + (ACTION_AREA_WIDTH - 188) / 2, actionY - 84)
  area.addChild(selfBadge)

  if (state.currentTurn === player.id) {
    const marker = createTurnMarker('あなたの手番')
    marker.x = actionX + (ACTION_AREA_WIDTH - 124) / 2
    marker.y = actionY - 42
    area.addChild(marker)
  }

  return area
}

const getActionAreaHeight = (buttons: TableActionButton[]): number => {
  const rows = Math.max(buttons.length, 1)
  return (
    ACTION_AREA_HEADER_HEIGHT +
    rows * ACTION_BUTTON_HEIGHT +
    (rows - 1) * ACTION_BUTTON_GAP +
    16
  )
}

const createTopArea = (state: GameState, humanPlayerIndex: PlayerIndex): Container => {
  const player = getRelativePlayer(state, humanPlayerIndex, 2)
  const area = new Container()
  area.label = 'top-area'

  const hand = createPlayerHand(player)
  hand.rotation = Math.PI
  hand.x = TABLE_CENTER_X
  hand.y = 160
  area.addChild(hand)

  const discards = createPlayerDiscards(player)
  discards.rotation = Math.PI
  discards.x = 742
  discards.y = 408
  area.addChild(discards)

  // 手番マーカーはスコアバッジ直下に水平で出す (回転で縦書きにならないように)。
  if (state.currentTurn === player.id) {
    const marker = createTurnMarker(`${player.name} の手番`)
    marker.x = 546 + (188 - 124) / 2
    marker.y = 82
    area.addChild(marker)
  }

  return area
}

const createLeftArea = (state: GameState, humanPlayerIndex: PlayerIndex): Container => {
  const player = getRelativePlayer(state, humanPlayerIndex, 3)
  const area = new Container()
  area.label = 'left-area'

  const hand = createPlayerHand(player)
  hand.rotation = Math.PI / 2
  hand.x = 292
  hand.y = TABLE_CENTER_Y
  area.addChild(hand)

  const discards = createPlayerDiscards(player)
  discards.rotation = Math.PI / 2
  discards.x = 428
  discards.y = 267
  area.addChild(discards)

  if (state.currentTurn === player.id) {
    const marker = createTurnMarker(`${player.name} の手番`)
    marker.x = 8 + (188 - 124) / 2
    marker.y = 82
    area.addChild(marker)
  }

  return area
}

const createRightArea = (state: GameState, humanPlayerIndex: PlayerIndex): Container => {
  const player = getRelativePlayer(state, humanPlayerIndex, 1)
  const area = new Container()
  area.label = 'right-area'

  const hand = createPlayerHand(player)
  hand.rotation = -Math.PI / 2
  hand.x = 988
  hand.y = TABLE_CENTER_Y
  area.addChild(hand)

  const discards = createPlayerDiscards(player)
  discards.rotation = -Math.PI / 2
  discards.x = 852
  discards.y = 453
  area.addChild(discards)

  if (state.currentTurn === player.id) {
    const marker = createTurnMarker(`${player.name} の手番`)
    marker.x = STAGE_WIDTH - 188 - 8 + (188 - 124) / 2
    marker.y = 82
    area.addChild(marker)
  }

  return area
}

const createFooter = (): Container => {
  const footer = new Container()
  footer.label = 'footer'

  // 下辺は操作 UI とログが占有するので、底にうっすら陰だけ敷く。
  const line = new Graphics()
  line.rect(0, STAGE_HEIGHT - 18, STAGE_WIDTH, 18).fill({ color: SHADOW_COLOR, alpha: 0.32 })
  footer.addChild(line)

  // 卓画面はゲームに集中する場面なのでブランドロゴは出さない。タイトルで露出済み。

  return footer
}

const EVENT_LOG_WIDTH = 360
const EVENT_LOG_HEIGHT = 104

// 対局ログは枠なしの薄い影だけで、読みやすさを保ちつつ前景に出ないようにする。
// 操作 UI は右下に集約しているのでパネル自身が左下に座る (左 24, 下端 24 マージン)。
const createEventLogPanel = (eventLog: string[]): Container => {
  const panel = new Container()
  panel.label = 'event-log'
  panel.x = 24
  panel.y = STAGE_HEIGHT - EVENT_LOG_HEIGHT - 24

  const bg = new Graphics()
  bg
    .roundRect(0, 0, EVENT_LOG_WIDTH, EVENT_LOG_HEIGHT, 12)
    .fill({ color: SHADOW_COLOR, alpha: 0.32 })
  panel.addChild(bg)

  const title = makeText('対局ログ', 13, TEXT_MUTED_COLOR, 'left', 'bold')
  title.x = 14
  title.y = 10
  panel.addChild(title)

  const visibleEntries = eventLog.slice(-EVENT_LOG_VISIBLE_COUNT)
  visibleEntries.forEach((entry, index) => {
    const row = makeText(entry, 13, TEXT_PRIMARY_COLOR)
    row.x = 14
    row.y = 32 + index * 17
    panel.addChild(row)
  })

  return panel
}

export const createTableScene = (state: GameState, options: TableSceneOptions = {}): Container => {
  const humanPlayerIndex = options.humanPlayerIndex ?? 0
  const root = new Container()
  root.label = 'game-table'

  addStageBackdrop(root)
  root.addChild(createTableSurface())
  root.addChild(createScoreBadges(state, humanPlayerIndex))
  root.addChild(createInfoPanel(state))
  root.addChild(createTopArea(state, humanPlayerIndex))
  root.addChild(createLeftArea(state, humanPlayerIndex))
  root.addChild(createRightArea(state, humanPlayerIndex))
  root.addChild(createBottomArea(state, options))
  root.addChild(createEventLogPanel(options.eventLog ?? []))
  root.addChild(createFooter())

  return root
}
