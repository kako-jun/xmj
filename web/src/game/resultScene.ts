import { Container, Graphics, Text, TextStyle } from 'pixi.js'
import {
  PANEL_ACCENT_COLOR,
  PANEL_BG_COLOR,
  PANEL_BORDER_COLOR,
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

  const W = 200
  const H = 50
  const bg = new Graphics()
  bg
    .roundRect(0, 0, W, H, 16)
    .fill({ color: 0x281608, alpha: 0.96 })
    .stroke({ color: PANEL_ACCENT_COLOR, width: 3 })
  button.addChild(bg)

  const label = makeText(labelText, 20, TEXT_PRIMARY_COLOR, 'center', 'bold')
  label.anchor.set(0.5)
  label.x = W / 2
  label.y = H / 2
  button.addChild(label)

  button.eventMode = 'static'
  button.cursor = 'pointer'
  button.on('pointertap', onTap)
  return button
}

export const createResultScene = (options: ResultSceneOptions): Container => {
  const root = new Container()
  root.label = 'result-scene'

  const cx = STAGE_WIDTH / 2

  const bg = new Graphics()
  bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: 0x050505 })
  bg.circle(cx, STAGE_HEIGHT / 2 - 80, STAGE_WIDTH * 0.45).fill({ color: TABLE_GLOW_COLOR, alpha: 0.12 })
  bg.circle(cx, STAGE_HEIGHT / 2, STAGE_WIDTH * 0.38).fill({ color: TABLE_BG_COLOR, alpha: 0.38 })
  root.addChild(bg)

  const frameMargin = 32
  const frame = new Graphics()
  frame
    .roundRect(frameMargin, frameMargin, STAGE_WIDTH - frameMargin * 2, STAGE_HEIGHT - frameMargin * 2, 24)
    .fill({ color: PANEL_BG_COLOR, alpha: 0.92 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 3 })
  root.addChild(frame)

  const title = makeText('終局結果', 38, TEXT_PRIMARY_COLOR, 'center', 'bold')
  title.anchor.set(0.5)
  title.x = cx
  title.y = 90
  root.addChild(title)

  const reason = makeText(options.reason, 18, TEXT_DANGER_COLOR, 'center', 'bold')
  reason.anchor.set(0.5)
  reason.x = cx
  reason.y = 130
  root.addChild(reason)

  // 順位パネル: 正方形ステージなので detail は省き、順位だけ大きく出す
  const panelW = STAGE_WIDTH - frameMargin * 2 - 40
  const panelX = frameMargin + 20
  const panelY = 170
  const panelH = 280
  const rankingPanel = new Graphics()
  rankingPanel
    .roundRect(panelX, panelY, panelW, panelH, 20)
    .fill({ color: 0x171717, alpha: 0.96 })
    .stroke({ color: PANEL_BORDER_COLOR, width: 2 })
  root.addChild(rankingPanel)

  const rankingTitle = makeText('順位 / 点数', 18, PANEL_ACCENT_COLOR, 'left', 'bold')
  rankingTitle.x = panelX + 22
  rankingTitle.y = panelY + 18
  root.addChild(rankingTitle)

  options.entries.forEach((entry, index) => {
    const y = panelY + 64 + index * 50
    const rank = makeText(`${entry.rank}位`, 18, TEXT_PRIMARY_COLOR, 'left', 'bold')
    rank.x = panelX + 26
    rank.y = y
    root.addChild(rank)

    const name = makeText(entry.name, 18, TEXT_PRIMARY_COLOR)
    name.x = panelX + 96
    name.y = y
    root.addChild(name)

    const score = makeText(`${entry.score.toLocaleString()} 点`, 18, TEXT_MUTED_COLOR, 'right', 'bold')
    score.anchor.set(1, 0)
    score.x = panelX + panelW - 24
    score.y = y
    root.addChild(score)
  })

  const buttonY = panelY + panelH + 30
  root.addChild(createButton('再戦', 'result-rematch-button', cx - 210, buttonY, options.onRematch))
  root.addChild(
    createButton('タイトルへ', 'result-title-button', cx + 10, buttonY, options.onBackToTitle)
  )

  return root
}
