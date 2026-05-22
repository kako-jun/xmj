import { Container, Graphics, Text, TextStyle } from 'pixi.js'
import {
  PANEL_ACCENT_COLOR,
  PANEL_BG_COLOR,
  PANEL_BORDER_COLOR,
  STAGE_HEIGHT,
  STAGE_WIDTH,
  TEXT_MUTED_COLOR,
  TEXT_PRIMARY_COLOR,
} from './constants'
import type { DiceRoll, PlayerIndex } from './types'

const SEAT_NAMES = ['東家', '南家', '西家', '北家'] as const

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

interface DiceRollSceneOptions {
  roll: DiceRoll | null
  humanSeat: PlayerIndex | null
  onComplete: () => void
}

const PIP_POSITIONS: Record<number, Array<[number, number]>> = {
  1: [[0.5, 0.5]],
  2: [
    [0.25, 0.25],
    [0.75, 0.75],
  ],
  3: [
    [0.25, 0.25],
    [0.5, 0.5],
    [0.75, 0.75],
  ],
  4: [
    [0.25, 0.25],
    [0.75, 0.25],
    [0.25, 0.75],
    [0.75, 0.75],
  ],
  5: [
    [0.25, 0.25],
    [0.75, 0.25],
    [0.5, 0.5],
    [0.25, 0.75],
    [0.75, 0.75],
  ],
  6: [
    [0.25, 0.25],
    [0.75, 0.25],
    [0.25, 0.5],
    [0.75, 0.5],
    [0.25, 0.75],
    [0.75, 0.75],
  ],
}

const DIE_SIZE = 110

const createDie = (value: number | null): Container => {
  const die = new Container()
  const bg = new Graphics()
  bg
    .roundRect(0, 0, DIE_SIZE, DIE_SIZE, 16)
    .fill({ color: 0xece2c4 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 3 })
  die.addChild(bg)

  if (value !== null && PIP_POSITIONS[value]) {
    const pipRadius = 8
    for (const [rx, ry] of PIP_POSITIONS[value]) {
      const pip = new Graphics()
      pip.circle(rx * DIE_SIZE, ry * DIE_SIZE, pipRadius).fill({ color: 0x1a1a1a })
      die.addChild(pip)
    }
  } else {
    const question = makeText('?', 58, 0x6f6f6f, 'center', 'bold')
    question.anchor.set(0.5)
    question.x = DIE_SIZE / 2
    question.y = DIE_SIZE / 2
    die.addChild(question)
  }

  return die
}

export const createDiceRollScene = (options: DiceRollSceneOptions): Container => {
  const root = new Container()
  root.label = 'dice-roll-scene'

  const cx = STAGE_WIDTH / 2

  // 対局画面の上に薄くかぶせる overlay。背面の卓が透けて見える設計。
  // overlay 中は背面の卓の牌・ボタンへのクリックを意図せずブロックしたいので
  // (場決め中に勝手に打牌できない方が安全)、root を interactive にして bg が
  // pointer を吸収する形にする。
  const bg = new Graphics()
  bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: 0x000000, alpha: 0.55 })
  bg.eventMode = 'static'
  root.addChild(bg)
  root.eventMode = 'static'

  // 中央のパネルだけ濃く
  const panelW = 420
  const panelH = 440
  const panel = new Graphics()
  panel
    .roundRect(cx - panelW / 2, STAGE_HEIGHT / 2 - panelH / 2 - 30, panelW, panelH, 24)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.94 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 3 })
  root.addChild(panel)

  const heading = makeText('場決め', 34, TEXT_PRIMARY_COLOR, 'center', 'bold')
  heading.anchor.set(0.5)
  heading.x = cx
  heading.y = 100
  root.addChild(heading)

  const subheading = makeText(
    'サイコロ 2 個の合計で起家を決める。',
    14,
    TEXT_MUTED_COLOR,
    'center'
  )
  subheading.anchor.set(0.5)
  subheading.x = cx
  subheading.y = 138
  root.addChild(subheading)

  const diceRow = new Container()
  diceRow.label = 'dice-row'
  const dieGap = 28
  diceRow.x = (STAGE_WIDTH - (DIE_SIZE * 2 + dieGap)) / 2
  diceRow.y = 200

  const die1 = createDie(options.roll?.d1 ?? null)
  die1.label = 'die-1'
  diceRow.addChild(die1)

  const die2 = createDie(options.roll?.d2 ?? null)
  die2.label = 'die-2'
  die2.x = DIE_SIZE + dieGap
  diceRow.addChild(die2)
  root.addChild(diceRow)

  if (options.roll && options.humanSeat !== null) {
    const sumText = makeText(
      `合計 ${options.roll.d1 + options.roll.d2}`,
      18,
      TEXT_MUTED_COLOR,
      'center'
    )
    sumText.anchor.set(0.5)
    sumText.x = cx
    sumText.y = 350
    sumText.label = 'dice-sum'
    root.addChild(sumText)

    const seatText = makeText(
      `あなたは${SEAT_NAMES[options.humanSeat]}からスタート`,
      22,
      PANEL_ACCENT_COLOR,
      'center',
      'bold'
    )
    seatText.anchor.set(0.5)
    seatText.x = cx
    seatText.y = 386
    seatText.label = 'dice-seat-result'
    root.addChild(seatText)

    const buttonW = 260
    const buttonH = 56
    const startButton = new Container()
    startButton.label = 'dice-roll-start-button'
    startButton.x = cx - buttonW / 2
    startButton.y = 450

    const buttonBg = new Graphics()
    buttonBg
      .roundRect(0, 0, buttonW, buttonH, 16)
      .fill({ color: 0x281608, alpha: 0.96 })
      .stroke({ color: PANEL_ACCENT_COLOR, width: 3 })
    startButton.addChild(buttonBg)

    const buttonLabel = makeText('対局を始める', 22, TEXT_PRIMARY_COLOR, 'center', 'bold')
    buttonLabel.anchor.set(0.5)
    buttonLabel.x = buttonW / 2
    buttonLabel.y = buttonH / 2
    startButton.addChild(buttonLabel)

    startButton.eventMode = 'static'
    startButton.cursor = 'pointer'
    startButton.on('pointertap', options.onComplete)
    root.addChild(startButton)
  } else {
    const rolling = makeText('サイコロを振っています…', 20, TEXT_MUTED_COLOR, 'center', 'bold')
    rolling.anchor.set(0.5)
    rolling.x = cx
    rolling.y = 386
    rolling.label = 'dice-rolling-text'
    root.addChild(rolling)
  }

  return root
}
