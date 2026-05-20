import { Container, Graphics, Text, TextStyle } from 'pixi.js'
import {
  PANEL_ACCENT_COLOR,
  PANEL_BG_COLOR,
  PANEL_BORDER_COLOR,
  SHADOW_COLOR,
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
  onSelectMode: (mode: GameMode) => void
  onConfirm: () => void
  onBack: () => void
}

const CARD_WIDTH = 280
const CARD_HEIGHT = 220
const CARD_GAP = 48

const createModeCard = (
  mode: GameModeOption,
  selected: boolean,
  onSelectMode: (mode: GameMode) => void
): Container => {
  const card = new Container()
  card.label = `mode-card-${mode.key}`

  const bg = new Graphics()
  bg
    .roundRect(0, 0, CARD_WIDTH, CARD_HEIGHT, 22)
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
    36,
    mode.enabled ? TEXT_PRIMARY_COLOR : TEXT_MUTED_COLOR,
    'center',
    'bold'
  )
  title.anchor.set(0.5, 0)
  title.x = CARD_WIDTH / 2
  title.y = 40
  card.addChild(title)

  const description = makeText(
    mode.description,
    16,
    mode.enabled ? TEXT_MUTED_COLOR : 0x6f6f6f,
    'center'
  )
  description.anchor.set(0.5, 0)
  description.x = CARD_WIDTH / 2
  description.y = 110
  card.addChild(description)

  const status = makeText(
    selected ? '選択中' : mode.enabled ? '選択可能' : '準備中',
    15,
    selected ? PANEL_ACCENT_COLOR : mode.enabled ? TEXT_MUTED_COLOR : 0x6f6f6f,
    'center',
    'bold'
  )
  status.anchor.set(0.5, 0)
  status.x = CARD_WIDTH / 2
  status.y = CARD_HEIGHT - 38
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

  const bg = new Graphics()
  bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: 0x040404 })
  bg.circle(STAGE_WIDTH / 2, 240, 380).fill({ color: TABLE_GLOW_COLOR, alpha: 0.12 })
  bg.circle(STAGE_WIDTH / 2, 320, 300).fill({ color: TABLE_BG_COLOR, alpha: 0.45 })
  root.addChild(bg)

  const frame = new Graphics()
  frame
    .roundRect(180, 90, 920, 510, 36)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.9 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 3 })
  root.addChild(frame)

  const heading = makeText('対局モードを選ぶ', 36, TEXT_PRIMARY_COLOR, 'center', 'bold')
  heading.anchor.set(0.5)
  heading.x = STAGE_WIDTH / 2
  heading.y = 150
  root.addChild(heading)

  const subheading = makeText(
    '東風戦は東場のみ、半荘戦は東南両場を打つ。',
    16,
    TEXT_MUTED_COLOR,
    'center'
  )
  subheading.anchor.set(0.5)
  subheading.x = STAGE_WIDTH / 2
  subheading.y = 196
  root.addChild(subheading)

  const cardsRow = new Container()
  cardsRow.label = 'mode-card-row'
  const totalWidth = options.modes.length * CARD_WIDTH + (options.modes.length - 1) * CARD_GAP
  cardsRow.x = (STAGE_WIDTH - totalWidth) / 2
  cardsRow.y = 240
  options.modes.forEach((mode, index) => {
    const card = createModeCard(mode, options.selectedMode === mode.key, options.onSelectMode)
    card.x = index * (CARD_WIDTH + CARD_GAP)
    cardsRow.addChild(card)
  })
  root.addChild(cardsRow)

  // 確定ボタン
  const selectedMode = options.modes.find(mode => mode.key === options.selectedMode)
  const confirmEnabled = selectedMode?.enabled ?? false

  const confirmButton = new Container()
  confirmButton.label = 'mode-select-confirm'
  confirmButton.x = STAGE_WIDTH / 2 - 140
  confirmButton.y = 504

  const confirmBg = new Graphics()
  confirmBg
    .roundRect(0, 0, 280, 58, 18)
    .fill({ color: confirmEnabled ? 0x281608 : 0x2a2a2a, alpha: confirmEnabled ? 0.96 : 0.8 })
    .stroke({
      color: confirmEnabled ? PANEL_ACCENT_COLOR : TEXT_MUTED_COLOR,
      width: 3,
      alpha: confirmEnabled ? 1 : 0.5,
    })
  confirmButton.addChild(confirmBg)

  const confirmLabel = makeText(
    '次へ',
    24,
    confirmEnabled ? TEXT_PRIMARY_COLOR : TEXT_MUTED_COLOR,
    'center',
    'bold'
  )
  confirmLabel.anchor.set(0.5)
  confirmLabel.x = 140
  confirmLabel.y = 29
  confirmButton.addChild(confirmLabel)

  if (confirmEnabled) {
    confirmButton.eventMode = 'static'
    confirmButton.cursor = 'pointer'
    confirmButton.on('pointertap', options.onConfirm)
  }
  root.addChild(confirmButton)

  // 戻るリンク
  const backButton = new Container()
  backButton.label = 'mode-select-back'
  backButton.x = STAGE_WIDTH / 2 - 60
  backButton.y = 588

  const backLabel = makeText('< 戻る', 17, TEXT_MUTED_COLOR, 'center')
  backLabel.anchor.set(0.5)
  backLabel.x = 60
  backLabel.y = 12
  backButton.addChild(backLabel)

  backButton.eventMode = 'static'
  backButton.cursor = 'pointer'
  backButton.on('pointertap', options.onBack)
  root.addChild(backButton)

  const footer = new Graphics()
  footer.rect(0, STAGE_HEIGHT - 72, STAGE_WIDTH, 72).fill({ color: SHADOW_COLOR, alpha: 0.34 })
  root.addChild(footer)

  return root
}
