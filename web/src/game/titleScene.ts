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

interface TitleSceneOptions {
  onStart: () => void
  notice?: string | null
  startEnabled?: boolean
}

export const createTitleScene = (options: TitleSceneOptions): Container => {
  const startEnabled = options.startEnabled ?? true
  const root = new Container()
  root.label = 'title-scene'

  const cx = STAGE_WIDTH / 2
  const cy = STAGE_HEIGHT / 2

  const bg = new Graphics()
  bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: 0x040404 })
  bg.circle(cx, cy - 80, STAGE_WIDTH * 0.5).fill({ color: TABLE_GLOW_COLOR, alpha: 0.12 })
  bg.circle(cx, cy - 20, STAGE_WIDTH * 0.4).fill({ color: TABLE_BG_COLOR, alpha: 0.45 })
  root.addChild(bg)

  const frameMargin = 40
  const frameSize = STAGE_WIDTH - frameMargin * 2
  const frame = new Graphics()
  frame
    .roundRect(frameMargin, frameMargin + 30, frameSize, frameSize - 60, 28)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.9 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 3 })
  root.addChild(frame)

  const logo = makeText('邪雀', 80, TEXT_PRIMARY_COLOR, 'center', 'bold')
  logo.anchor.set(0.5)
  logo.x = cx
  logo.y = cy - 130
  root.addChild(logo)

  const subtitle = makeText('Xtreme Mahjong', 24, PANEL_ACCENT_COLOR, 'center', 'bold')
  subtitle.anchor.set(0.5)
  subtitle.x = cx
  subtitle.y = cy - 60
  root.addChild(subtitle)

  const description = makeText('CPU 3 人との一局を、PixiJS の卓上で。', 15, TEXT_MUTED_COLOR, 'center')
  description.anchor.set(0.5)
  description.x = cx
  description.y = cy - 8
  root.addChild(description)

  const buttonW = 264
  const buttonH = 60
  const startButton = new Container()
  startButton.label = 'title-start-button'
  startButton.x = cx - buttonW / 2
  startButton.y = cy + 40

  const buttonBg = new Graphics()
  buttonBg
    .roundRect(0, 0, buttonW, buttonH, 16)
    .fill({ color: startEnabled ? 0x281608 : 0x2a2a2a, alpha: startEnabled ? 0.96 : 0.8 })
    .stroke({
      color: startEnabled ? PANEL_ACCENT_COLOR : TEXT_MUTED_COLOR,
      width: 3,
      alpha: startEnabled ? 1 : 0.5,
    })
  startButton.addChild(buttonBg)

  const buttonLabel = makeText(
    '対局開始',
    24,
    startEnabled ? TEXT_PRIMARY_COLOR : TEXT_MUTED_COLOR,
    'center',
    'bold'
  )
  buttonLabel.anchor.set(0.5)
  buttonLabel.x = buttonW / 2
  buttonLabel.y = buttonH / 2
  startButton.addChild(buttonLabel)

  if (startEnabled) {
    startButton.eventMode = 'static'
    startButton.cursor = 'pointer'
    startButton.on('pointertap', options.onStart)
  }
  root.addChild(startButton)

  const notice = makeText(
    options.notice ?? '東風戦 / 半荘戦 → 場決め → 対局',
    14,
    options.notice ? 0xd6b56f : TEXT_MUTED_COLOR,
    'center'
  )
  notice.anchor.set(0.5)
  notice.x = cx
  notice.y = cy + 130
  root.addChild(notice)

  return root
}
