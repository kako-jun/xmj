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
}

export const createTitleScene = (options: TitleSceneOptions): Container => {
  const root = new Container()
  root.label = 'title-scene'

  const bg = new Graphics()
  bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: 0x040404 })
  bg.circle(STAGE_WIDTH / 2, 220, 360).fill({ color: TABLE_GLOW_COLOR, alpha: 0.12 })
  bg.circle(STAGE_WIDTH / 2, 300, 280).fill({ color: TABLE_BG_COLOR, alpha: 0.45 })
  root.addChild(bg)

  const frame = new Graphics()
  frame
    .roundRect(280, 110, 720, 470, 36)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.9 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 3 })
  root.addChild(frame)

  const logo = makeText('邪雀', 88, TEXT_PRIMARY_COLOR, 'center', 'bold')
  logo.anchor.set(0.5)
  logo.x = STAGE_WIDTH / 2
  logo.y = 210
  root.addChild(logo)

  const subtitle = makeText(
    'Xtreme Mahjong',
    28,
    PANEL_ACCENT_COLOR,
    'center',
    'bold'
  )
  subtitle.anchor.set(0.5)
  subtitle.x = STAGE_WIDTH / 2
  subtitle.y = 278
  root.addChild(subtitle)

  const description = makeText(
    'CPU 3 人との一局を、PixiJS の卓上で始める。',
    18,
    TEXT_MUTED_COLOR,
    'center'
  )
  description.anchor.set(0.5)
  description.x = STAGE_WIDTH / 2
  description.y = 342
  root.addChild(description)

  const startButton = new Container()
  startButton.label = 'title-start-button'
  startButton.x = STAGE_WIDTH / 2 - 144
  startButton.y = 410

  const buttonBg = new Graphics()
  buttonBg
    .roundRect(0, 0, 288, 58, 18)
    .fill({ color: 0x281608, alpha: 0.96 })
    .stroke({ color: PANEL_ACCENT_COLOR, width: 3 })
  startButton.addChild(buttonBg)

  const buttonLabel = makeText('CPU 対戦スタート', 24, TEXT_PRIMARY_COLOR, 'center', 'bold')
  buttonLabel.anchor.set(0.5)
  buttonLabel.x = 144
  buttonLabel.y = 29
  startButton.addChild(buttonLabel)

  startButton.eventMode = 'static'
  startButton.cursor = 'pointer'
  startButton.on('pointertap', options.onStart)
  root.addChild(startButton)

  const notice = makeText(
    options.notice ?? 'Rust/Wasm の現 API 範囲で最小対局フローを構成。',
    15,
    options.notice ? 0xd6b56f : TEXT_MUTED_COLOR,
    'center'
  )
  notice.anchor.set(0.5)
  notice.x = STAGE_WIDTH / 2
  notice.y = 506
  root.addChild(notice)

  const footer = new Graphics()
  footer.rect(0, STAGE_HEIGHT - 72, STAGE_WIDTH, 72).fill({ color: SHADOW_COLOR, alpha: 0.34 })
  root.addChild(footer)

  const footerText = makeText('Title → Table → Result を PixiJS Container で切替', 16, TEXT_MUTED_COLOR)
  footerText.anchor.set(0.5)
  footerText.x = STAGE_WIDTH / 2
  footerText.y = STAGE_HEIGHT - 42
  root.addChild(footerText)

  return root
}
