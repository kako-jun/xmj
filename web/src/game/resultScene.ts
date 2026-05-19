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
  TEXT_DANGER_COLOR,
  TEXT_MUTED_COLOR,
  TEXT_PRIMARY_COLOR,
} from './constants'
import type { PlayerIndex } from './types'

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

export interface ResultEntry {
  rank: number
  playerId: PlayerIndex
  name: string
  score: number
}

interface ResultSceneOptions {
  reason: string
  entries: ResultEntry[]
  detailPlaceholder: string
  onRematch: () => void
  onBackToTitle: () => void
}

const createButton = (
  labelText: string,
  sceneLabel: string,
  x: number,
  y: number,
  onTap: () => void
): Container => {
  const button = new Container()
  button.label = sceneLabel
  button.x = x
  button.y = y

  const bg = new Graphics()
  bg
    .roundRect(0, 0, 220, 52, 18)
    .fill({ color: 0x281608, alpha: 0.96 })
    .stroke({ color: PANEL_ACCENT_COLOR, width: 3 })
  button.addChild(bg)

  const label = makeText(labelText, 22, TEXT_PRIMARY_COLOR, 'center', 'bold')
  label.anchor.set(0.5)
  label.x = 110
  label.y = 26
  button.addChild(label)

  button.eventMode = 'static'
  button.cursor = 'pointer'
  button.on('pointertap', onTap)
  return button
}

export const createResultScene = (options: ResultSceneOptions): Container => {
  const root = new Container()
  root.label = 'result-scene'

  const bg = new Graphics()
  bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: 0x050505 })
  bg.circle(STAGE_WIDTH / 2, 160, 320).fill({ color: TABLE_GLOW_COLOR, alpha: 0.12 })
  bg.circle(STAGE_WIDTH / 2, 220, 280).fill({ color: TABLE_BG_COLOR, alpha: 0.38 })
  root.addChild(bg)

  const frame = new Graphics()
  frame
    .roundRect(186, 56, 908, 588, 34)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.92 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 3 })
  root.addChild(frame)

  const title = makeText('終局結果', 42, TEXT_PRIMARY_COLOR, 'center', 'bold')
  title.anchor.set(0.5)
  title.x = STAGE_WIDTH / 2
  title.y = 108
  root.addChild(title)

  const reason = makeText(options.reason, 22, TEXT_DANGER_COLOR, 'center', 'bold')
  reason.anchor.set(0.5)
  reason.x = STAGE_WIDTH / 2
  reason.y = 156
  root.addChild(reason)

  const rankingPanel = new Graphics()
  rankingPanel
    .roundRect(244, 204, 418, 248, 24)
    .fill({ color: 0x171717, alpha: 0.96 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 2 })
  root.addChild(rankingPanel)

  const rankingTitle = makeText('順位 / 点数', 20, PANEL_ACCENT_COLOR, 'left', 'bold')
  rankingTitle.x = 272
  rankingTitle.y = 228
  root.addChild(rankingTitle)

  options.entries.forEach((entry, index) => {
    const y = 274 + index * 42
    const rank = makeText(`${entry.rank}位`, 18, TEXT_PRIMARY_COLOR, 'left', 'bold')
    rank.x = 276
    rank.y = y
    root.addChild(rank)

    const name = makeText(entry.name, 18, TEXT_PRIMARY_COLOR)
    name.x = 352
    name.y = y
    root.addChild(name)

    const score = makeText(`${entry.score.toLocaleString()} 点`, 18, TEXT_MUTED_COLOR, 'right', 'bold')
    score.anchor.set(1, 0)
    score.x = 626
    score.y = y
    root.addChild(score)
  })

  const detailPanel = new Graphics()
  detailPanel
    .roundRect(700, 204, 336, 248, 24)
    .fill({ color: 0x171717, alpha: 0.96 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 2 })
  root.addChild(detailPanel)

  const detailTitle = makeText('役 / 点数 / 収支', 20, PANEL_ACCENT_COLOR, 'left', 'bold')
  detailTitle.x = 728
  detailTitle.y = 228
  root.addChild(detailTitle)

  const detailText = makeText(options.detailPlaceholder, 18, TEXT_MUTED_COLOR)
  detailText.x = 728
  detailText.y = 284
  root.addChild(detailText)

  root.addChild(createButton('再戦', 'result-rematch-button', 368, 508, options.onRematch))
  root.addChild(
    createButton('タイトルへ', 'result-title-button', 692, 508, options.onBackToTitle)
  )

  const footer = new Graphics()
  footer.rect(0, STAGE_HEIGHT - 72, STAGE_WIDTH, 72).fill({ color: SHADOW_COLOR, alpha: 0.34 })
  root.addChild(footer)

  const footerText = makeText(
    '詳細 API は未実装のため、最小結果画面で終局を成立させる。',
    16,
    TEXT_MUTED_COLOR,
    'center'
  )
  footerText.anchor.set(0.5)
  footerText.x = STAGE_WIDTH / 2
  footerText.y = STAGE_HEIGHT - 42
  root.addChild(footerText)

  return root
}
