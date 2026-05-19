import { Container, Graphics, Text, TextStyle } from 'pixi.js'
import {
  DISCARD_SLOT_COLOR,
  PANEL_ACCENT_COLOR,
  PANEL_BG_COLOR,
  PANEL_BORDER_COLOR,
  SHADOW_COLOR,
  STAGE_HEIGHT,
  STAGE_WIDTH,
  TABLE_BG_COLOR,
  TABLE_BORDER_COLOR,
  TABLE_FELT_INNER_COLOR,
  TABLE_GLOW_COLOR,
  TEXT_DANGER_COLOR,
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
  selectedHandIndex?: number | null
  interactiveHandPlayerId?: PlayerIndex | null
  onHandTileTap?: (index: number) => void
  actionButtons?: TableActionButton[]
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
    .fill({ color: TABLE_GLOW_COLOR, alpha: 0.14 })
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

  const bg = new Graphics()
  bg
    .roundRect(0, 0, 290, 190, 18)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.92 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 2 })
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

  panel.x = TABLE_CENTER_X - 145
  panel.y = TABLE_CENTER_Y - 95
  return panel
}

const addSeatBadge = (player: PlayerState, seat: number, x: number, y: number): Container => {
  const seatBadge = new Container()

  const frame = new Graphics()
  frame
    .roundRect(0, 0, 188, 72, 18)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.9 })
    .stroke({ color: player.id === seat ? PANEL_BORDER_COLOR : PANEL_ACCENT_COLOR, width: 2 })
  seatBadge.addChild(frame)

  const wind = makeText(PLAYER_WIND[seat], 24, PANEL_ACCENT_COLOR, 'center', 'bold')
  wind.anchor.set(0.5)
  wind.x = 26
  wind.y = 36
  seatBadge.addChild(wind)

  const name = makeText(player.name, 18, TEXT_PRIMARY_COLOR, 'left', 'bold')
  name.x = 48
  name.y = 12
  seatBadge.addChild(name)

  const score = makeText(`${player.score.toLocaleString()} 点`, 16, TEXT_MUTED_COLOR)
  score.x = 48
  score.y = 40
  seatBadge.addChild(score)

  if (player.isRiichi) {
    const riichi = makeText('立直', 15, TEXT_DANGER_COLOR, 'right', 'bold')
    riichi.anchor.set(1, 0)
    riichi.x = 170
    riichi.y = 14
    seatBadge.addChild(riichi)
  }

  seatBadge.x = x
  seatBadge.y = y
  return seatBadge
}

const createScoreBadges = (state: GameState): Container => {
  const badges = new Container()
  badges.label = 'score-badges'
  badges.addChild(addSeatBadge(state.players[2], 2, 546, 34))
  badges.addChild(addSeatBadge(state.players[1], 1, 888, 242))
  badges.addChild(addSeatBadge(state.players[0], 0, 546, 610))
  badges.addChild(addSeatBadge(state.players[3], 3, 204, 242))
  return badges
}

const createPlayerHand = (player: PlayerState, options: TableSceneOptions = {}): Container => {
  const hand = new Container()
  hand.label = `hand-${player.id}`

  const total = player.hand.length
  player.hand.forEach((tile, index) => {
    const sprite = player.isCPU ? createTileBackGraphics() : createTileGraphics(tile)
    sprite.x = (index - (total - 1) / 2) * 42

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
  const tileScale = 0.62
  for (let index = 0; index < 18; index++) {
    const col = index % maxCols
    const row = Math.floor(index / maxCols)
    const slot = new Graphics()
    slot
      .roundRect(col * 34, row * 48, 30, 42, 8)
      .fill({ color: DISCARD_SLOT_COLOR, alpha: index < player.discards.length ? 0.2 : 0.42 })
      .stroke({ color: PANEL_ACCENT_COLOR, width: 1, alpha: 0.18 })
    discards.addChild(slot)
  }

  player.discards.forEach((tile, index) => {
    const col = index % maxCols
    const row = Math.floor(index / maxCols)
    const sprite = createTileGraphics(tile)
    sprite.scale.set(tileScale)
    sprite.x = col * 34
    sprite.y = row * 48
    discards.addChild(sprite)
  })

  return discards
}

const createTurnMarker = (label: string): Container => {
  const marker = new Container()
  marker.label = 'turn-marker'
  const chip = new Graphics()
  chip
    .roundRect(0, 0, 124, 34, 12)
    .fill({ color: 0x23180b, alpha: 0.94 })
    .stroke({ color: TURN_GLOW_COLOR, width: 2 })
  marker.addChild(chip)

  const text = makeText(label, 14, TURN_GLOW_COLOR, 'center', 'bold')
  text.anchor.set(0.5)
  text.x = 62
  text.y = 17
  marker.addChild(text)
  return marker
}

const createActionArea = (buttons: TableActionButton[]): Container => {
  const area = new Container()
  area.label = 'action-area'

  const bg = new Graphics()
  bg
    .roundRect(0, 0, 248, 92, 18)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.94 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 2 })
  area.addChild(bg)

  const title = makeText('行動', 15, TEXT_MUTED_COLOR, 'left', 'bold')
  title.x = 16
  title.y = 12
  area.addChild(title)

  if (buttons.length === 0) {
    const empty = makeText('選択肢なし', 17, TEXT_MUTED_COLOR, 'left')
    empty.x = 16
    empty.y = 48
    area.addChild(empty)
    return area
  }

  buttons.forEach((button, index) => {
    const action = new Container()
    action.label = `action-button-${button.key}`
    action.x = 16 + index * 110
    action.y = 40

    const plate = new Graphics()
    plate
      .roundRect(0, 0, 98, 36, 12)
      .fill({
        color: button.enabled ? 0x2a1d0d : 0x2a2a2a,
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
      16,
      button.enabled ? TEXT_PRIMARY_COLOR : TEXT_MUTED_COLOR,
      'center',
      'bold'
    )
    label.anchor.set(0.5)
    label.x = 49
    label.y = 18
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
  const area = new Container()
  area.label = 'bottom-area'

  const hand = createPlayerHand(state.players[0], options)
  hand.x = TABLE_CENTER_X
  hand.y = 560
  area.addChild(hand)

  const discards = createPlayerDiscards(state.players[0])
  discards.x = 538
  discards.y = 486
  area.addChild(discards)

  if (state.currentTurn === 0) {
    const marker = createTurnMarker('あなたの手番')
    marker.x = 980
    marker.y = 620
    area.addChild(marker)
  }

  const actionArea = createActionArea(options.actionButtons ?? [])
  actionArea.x = 38
  actionArea.y = 606
  area.addChild(actionArea)

  return area
}

const createTopArea = (state: GameState): Container => {
  const area = new Container()
  area.label = 'top-area'

  const hand = createPlayerHand(state.players[2])
  hand.rotation = Math.PI
  hand.x = TABLE_CENTER_X
  hand.y = 160
  area.addChild(hand)

  const discards = createPlayerDiscards(state.players[2])
  discards.rotation = Math.PI
  discards.x = 742
  discards.y = 408
  area.addChild(discards)

  if (state.currentTurn === 2) {
    const marker = createTurnMarker('対面の手番')
    marker.rotation = Math.PI
    marker.x = 746
    marker.y = 106
    area.addChild(marker)
  }

  return area
}

const createLeftArea = (state: GameState): Container => {
  const area = new Container()
  area.label = 'left-area'

  const hand = createPlayerHand(state.players[3])
  hand.rotation = Math.PI / 2
  hand.x = 292
  hand.y = TABLE_CENTER_Y
  area.addChild(hand)

  const discards = createPlayerDiscards(state.players[3])
  discards.rotation = Math.PI / 2
  discards.x = 428
  discards.y = 267
  area.addChild(discards)

  if (state.currentTurn === 3) {
    const marker = createTurnMarker('北家の手番')
    marker.rotation = Math.PI / 2
    marker.x = 258
    marker.y = 432
    area.addChild(marker)
  }

  return area
}

const createRightArea = (state: GameState): Container => {
  const area = new Container()
  area.label = 'right-area'

  const hand = createPlayerHand(state.players[1])
  hand.rotation = -Math.PI / 2
  hand.x = 988
  hand.y = TABLE_CENTER_Y
  area.addChild(hand)

  const discards = createPlayerDiscards(state.players[1])
  discards.rotation = -Math.PI / 2
  discards.x = 852
  discards.y = 453
  area.addChild(discards)

  if (state.currentTurn === 1) {
    const marker = createTurnMarker('南家の手番')
    marker.rotation = -Math.PI / 2
    marker.x = 1022
    marker.y = 288
    area.addChild(marker)
  }

  return area
}

const createFooter = (): Container => {
  const footer = new Container()
  footer.label = 'footer'

  const line = new Graphics()
  line.rect(0, STAGE_HEIGHT - 86, STAGE_WIDTH, 86).fill({ color: SHADOW_COLOR, alpha: 0.34 })
  footer.addChild(line)

  const caption = makeText('手牌 / 河 / 山を PixiJS で描画', 14, TEXT_MUTED_COLOR)
  caption.x = 46
  caption.y = STAGE_HEIGHT - 52
  footer.addChild(caption)

  const brand = makeText('邪雀 Xtreme Mahjong', 18, TEXT_PRIMARY_COLOR, 'right', 'bold')
  brand.anchor.set(1, 0)
  brand.x = STAGE_WIDTH - 40
  brand.y = STAGE_HEIGHT - 58
  footer.addChild(brand)

  return footer
}

export const createTableScene = (state: GameState, options: TableSceneOptions = {}): Container => {
  const root = new Container()
  root.label = 'game-table'

  addStageBackdrop(root)
  root.addChild(createTableSurface())
  root.addChild(createScoreBadges(state))
  root.addChild(createInfoPanel(state))
  root.addChild(createTopArea(state))
  root.addChild(createLeftArea(state))
  root.addChild(createRightArea(state))
  root.addChild(createBottomArea(state, options))
  root.addChild(createFooter())

  return root
}
