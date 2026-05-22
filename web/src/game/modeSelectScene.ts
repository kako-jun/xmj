import { Container, Graphics, Text, TextStyle } from 'pixi.js'
import {
  PANEL_ACCENT_COLOR,
  PANEL_BG_COLOR,
  PANEL_BORDER_COLOR,
  STAGE_HEIGHT,
  STAGE_WIDTH,
  TABLE_BG_COLOR,
  TABLE_GLOW_COLOR,
  TEXT_MUTED_COLOR,
  TEXT_PRIMARY_COLOR,
} from './constants'
import type { GameMode, GameModeOption } from './types'

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

interface ModeSelectSceneOptions {
  selectedMode: GameMode
  modes: GameModeOption[]
  /** カードタップで「選択 + 確定」を一括で行うコールバック。 */
  onSelectMode: (mode: GameMode) => void
  onBack: () => void
}

const CARD_WIDTH = 260
const CARD_HEIGHT = 200
const CARD_GAP = 24

const createModeCard = (
  mode: GameModeOption,
  selected: boolean,
  onSelectMode: (mode: GameMode) => void
): Container => {
  const card = new Container()
  card.label = `mode-card-${mode.key}`

  const bg = new Graphics()
  bg
    .roundRect(0, 0, CARD_WIDTH, CARD_HEIGHT, 18)
    .fill({
      color: selected ? 0x24160c : 0x141414,
      alpha: mode.enabled ? 0.96 : 0.82,
    })
    .stroke({
      color: selected ? PANEL_ACCENT_COLOR : mode.enabled ? PANEL_BORDER_COLOR : TEXT_MUTED_COLOR,
      width: selected ? 4 : 2,
      alpha: mode.enabled ? 1 : 0.5,
    })
  card.addChild(bg)

  const title = makeText(
    mode.title,
    32,
    mode.enabled ? TEXT_PRIMARY_COLOR : TEXT_MUTED_COLOR,
    'center',
    'bold'
  )
  title.anchor.set(0.5, 0)
  title.x = CARD_WIDTH / 2
  title.y = 36
  card.addChild(title)

  const description = makeText(
    mode.description,
    14,
    mode.enabled ? TEXT_MUTED_COLOR : 0x6f6f6f,
    'center'
  )
  description.anchor.set(0.5, 0)
  description.x = CARD_WIDTH / 2
  description.y = 96
  card.addChild(description)

  const status = makeText(
    selected ? '選択中' : mode.enabled ? '選択可能' : '準備中',
    13,
    selected ? PANEL_ACCENT_COLOR : mode.enabled ? TEXT_MUTED_COLOR : 0x6f6f6f,
    'center',
    'bold'
  )
  status.anchor.set(0.5, 0)
  status.x = CARD_WIDTH / 2
  status.y = CARD_HEIGHT - 32
  card.addChild(status)

  if (mode.enabled) {
    card.eventMode = 'static'
    card.cursor = 'pointer'
    card.on('pointertap', () => {
      onSelectMode(mode.key)
    })
  }

  return card
}

export const createModeSelectScene = (options: ModeSelectSceneOptions): Container => {
  const root = new Container()
  root.label = 'mode-select-scene'

  const cx = STAGE_WIDTH / 2

  const bg = new Graphics()
  bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: 0x040404 })
  bg.circle(cx, STAGE_HEIGHT / 2 - 60, STAGE_WIDTH * 0.5).fill({ color: TABLE_GLOW_COLOR, alpha: 0.12 })
  bg.circle(cx, STAGE_HEIGHT / 2, STAGE_WIDTH * 0.4).fill({ color: TABLE_BG_COLOR, alpha: 0.45 })
  root.addChild(bg)

  const frameMargin = 28
  const frame = new Graphics()
  frame
    .roundRect(frameMargin, frameMargin, STAGE_WIDTH - frameMargin * 2, STAGE_HEIGHT - frameMargin * 2, 24)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.9 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 3 })
  root.addChild(frame)

  const heading = makeText('対局モードを選ぶ', 30, TEXT_PRIMARY_COLOR, 'center', 'bold')
  heading.anchor.set(0.5)
  heading.x = cx
  heading.y = 90
  root.addChild(heading)

  const subheading = makeText(
    '東風戦は東場のみ、半荘戦は東南両場。',
    14,
    TEXT_MUTED_COLOR,
    'center'
  )
  subheading.anchor.set(0.5)
  subheading.x = cx
  subheading.y = 128
  root.addChild(subheading)

  // 縦並び (722 wide では 2 枚横並びだと窮屈なので、縦に 1 列で並べる)
  const cardsRow = new Container()
  cardsRow.label = 'mode-card-row'
  const totalHeight = options.modes.length * CARD_HEIGHT + (options.modes.length - 1) * CARD_GAP
  cardsRow.x = cx - CARD_WIDTH / 2
  cardsRow.y = 160
  options.modes.forEach((mode, index) => {
    const card = createModeCard(mode, options.selectedMode === mode.key, options.onSelectMode)
    card.x = 0
    card.y = index * (CARD_HEIGHT + CARD_GAP)
    cardsRow.addChild(card)
  })
  root.addChild(cardsRow)

  // 「次へ」ボタンは廃止。カードタップで即確定するため不要。
  const backButton = new Container()
  backButton.label = 'mode-select-back'
  backButton.x = cx - 60
  backButton.y = 160 + totalHeight + 30

  const backLabel = makeText('< 戻る', 16, TEXT_MUTED_COLOR, 'center')
  backLabel.anchor.set(0.5)
  backLabel.x = 60
  backLabel.y = 10
  backButton.addChild(backLabel)

  backButton.eventMode = 'static'
  backButton.cursor = 'pointer'
  backButton.on('pointertap', options.onBack)
  root.addChild(backButton)

  return root
}
