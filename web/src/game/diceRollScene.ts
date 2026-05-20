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

const createDie = (value: number | null): Container => {
  const die = new Container()
  const size = 120
  const bg = new Graphics()
  bg
    .roundRect(0, 0, size, size, 18)
    .fill({ color: 0xece2c4 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 3 })
  die.addChild(bg)

  if (value !== null && PIP_POSITIONS[value]) {
    const pipRadius = 9
    for (const [rx, ry] of PIP_POSITIONS[value]) {
      const pip = new Graphics()
      pip.circle(rx * size, ry * size, pipRadius).fill({ color: 0x1a1a1a })
      die.addChild(pip)
    }
  } else {
    const question = makeText('?', 64, 0x6f6f6f, 'center', 'bold')
    question.anchor.set(0.5)
    question.x = size / 2
    question.y = size / 2
    die.addChild(question)
  }

  return die
}

export const createDiceRollScene = (options: DiceRollSceneOptions): Container => {
  const root = new Container()
  root.label = 'dice-roll-scene'

  const bg = new Graphics()
  bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: 0x040404 })
  bg.circle(STAGE_WIDTH / 2, 240, 380).fill({ color: TABLE_GLOW_COLOR, alpha: 0.12 })
  bg.circle(STAGE_WIDTH / 2, 320, 300).fill({ color: TABLE_BG_COLOR, alpha: 0.45 })
  root.addChild(bg)

  const frame = new Graphics()
  frame
    .roundRect(280, 110, 720, 510, 36)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.9 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 3 })
  root.addChild(frame)

  const heading = makeText('場決め', 38, TEXT_PRIMARY_COLOR, 'center', 'bold')
  heading.anchor.set(0.5)
  heading.x = STAGE_WIDTH / 2
  heading.y = 170
  root.addChild(heading)

  const subheading = makeText(
    'サイコロ 2 個の合計で起家を決める。',
    16,
    TEXT_MUTED_COLOR,
    'center'
  )
  subheading.anchor.set(0.5)
  subheading.x = STAGE_WIDTH / 2
  subheading.y = 214
  root.addChild(subheading)

  // ダイス 2 個を中央に並べる
  const diceRow = new Container()
  diceRow.label = 'dice-row'
  const dieSize = 120
  const dieGap = 36
  diceRow.x = (STAGE_WIDTH - (dieSize * 2 + dieGap)) / 2
  diceRow.y = 260

  const die1 = createDie(options.roll?.d1 ?? null)
  die1.label = 'die-1'
  diceRow.addChild(die1)

  const die2 = createDie(options.roll?.d2 ?? null)
  die2.label = 'die-2'
  die2.x = dieSize + dieGap
  diceRow.addChild(die2)
  root.addChild(diceRow)

  // 結果テキスト
  if (options.roll && options.humanSeat !== null) {
    const sumText = makeText(
      `合計 ${options.roll.d1 + options.roll.d2}`,
      18,
      TEXT_MUTED_COLOR,
      'center'
    )
    sumText.anchor.set(0.5)
    sumText.x = STAGE_WIDTH / 2
    sumText.y = 410
    sumText.label = 'dice-sum'
    root.addChild(sumText)

    const seatText = makeText(
      `あなたは${SEAT_NAMES[options.humanSeat]}からスタート`,
      24,
      PANEL_ACCENT_COLOR,
      'center',
      'bold'
    )
    seatText.anchor.set(0.5)
    seatText.x = STAGE_WIDTH / 2
    seatText.y = 446
    seatText.label = 'dice-seat-result'
    root.addChild(seatText)

    const startButton = new Container()
    startButton.label = 'dice-roll-start-button'
    startButton.x = STAGE_WIDTH / 2 - 140
    startButton.y = 506

    const buttonBg = new Graphics()
    buttonBg
      .roundRect(0, 0, 280, 58, 18)
      .fill({ color: 0x281608, alpha: 0.96 })
      .stroke({ color: PANEL_ACCENT_COLOR, width: 3 })
    startButton.addChild(buttonBg)

    const buttonLabel = makeText('対局を始める', 24, TEXT_PRIMARY_COLOR, 'center', 'bold')
    buttonLabel.anchor.set(0.5)
    buttonLabel.x = 140
    buttonLabel.y = 29
    startButton.addChild(buttonLabel)

    startButton.eventMode = 'static'
    startButton.cursor = 'pointer'
    startButton.on('pointertap', options.onComplete)
    root.addChild(startButton)
  } else {
    // 現状は App.showDiceRollScene が同期的に roll を確定させて渡すため、この分岐は
    // テスト経路でしか踏まれない。将来サイコロのアニメーションを入れる時に
    // 「振っている最中」のフレームを描画する経路として残す。
    const rolling = makeText(
      'サイコロを振っています…',
      22,
      TEXT_MUTED_COLOR,
      'center',
      'bold'
    )
    rolling.anchor.set(0.5)
    rolling.x = STAGE_WIDTH / 2
    rolling.y = 446
    rolling.label = 'dice-rolling-text'
    root.addChild(rolling)
  }

  const footer = new Graphics()
  footer.rect(0, STAGE_HEIGHT - 72, STAGE_WIDTH, 72).fill({ color: SHADOW_COLOR, alpha: 0.34 })
  root.addChild(footer)

  return root
}
