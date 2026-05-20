// 1 局終了 (中間結果) シーン (Issue #27)
//
// 役割:
//   - 和了 / 流局の結果を表示する
//   - 「次局へ」「タイトルへ」のボタンを持つ
//   - 終局 (game over) は別の resultScene が担当する。本シーンは
//     「対局はまだ続く」前提のみ扱う。

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
import type { PlayerIndex, RoundOutcome } from './types'
import { yakuLabel } from './yakuLabels'

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

interface RoundResultSceneOptions {
  outcome: RoundOutcome
  /** プレイヤー名解決。Rust 側 `bridge.getPlayerName(idx)` を渡す想定。 */
  getPlayerName: (idx: PlayerIndex) => string
  /** 次局へ進む。Apps 側で `bridge.nextRound()` → 再描画する。 */
  onNext: () => void
  /** タイトルに戻る。途中棄権相当。 */
  onBackToTitle: () => void
}

const summarizeWin = (
  outcome: Extract<RoundOutcome, { kind: 'win' }>,
  getPlayerName: (idx: PlayerIndex) => string
): string[] => {
  const lines: string[] = []
  const w = outcome.data
  const winnerName = getPlayerName(w.winner)
  const head =
    w.winType === 'tsumo'
      ? `${winnerName} のツモ和了`
      : `${winnerName} が ${getPlayerName(w.from ?? 0)} からロン和了`
  lines.push(head)
  lines.push(`${w.han}飜 ${w.fu}符 / ${w.totalPoints.toLocaleString()} 点`)
  if (w.yaku.length > 0) {
    lines.push(`役: ${w.yaku.map(yakuLabel).join(' / ')}`)
  } else {
    lines.push('役: なし')
  }
  return lines
}

const summarizeDraw = (
  outcome: Extract<RoundOutcome, { kind: 'draw' }>,
  getPlayerName: (idx: PlayerIndex) => string
): string[] => {
  const lines: string[] = ['流局']
  const tenpaiCount = outcome.data.tenpaiPlayers.length
  if (tenpaiCount === 0) {
    lines.push('テンパイ者: なし (全員ノーテン)')
    lines.push('ノーテン罰符: なし')
  } else if (tenpaiCount === 4) {
    const names = outcome.data.tenpaiPlayers.map(i => getPlayerName(i)).join(' / ')
    lines.push(`テンパイ: ${names}`)
    lines.push('ノーテン罰符: なし (全員聴牌)')
  } else {
    const names = outcome.data.tenpaiPlayers.map(i => getPlayerName(i)).join(' / ')
    lines.push(`テンパイ: ${names}`)
    // 1人 → 聴牌 +3000 / ノーテン -1000
    // 2人 → 聴牌 +1500 / ノーテン -1500
    // 3人 → 聴牌 +1000 / ノーテン -3000
    // 罰符配分定数は src/game.rs::resolve_draw の真値。
    // Rust 側を変更したら追従が必要 (本来は resolve_draw 結果から差分を逆算すべき)。
    const perTenpai = tenpaiCount === 1 ? 3000 : tenpaiCount === 2 ? 1500 : 1000
    const perNoten = tenpaiCount === 1 ? 1000 : tenpaiCount === 2 ? 1500 : 3000
    lines.push(
      `ノーテン罰符: 聴牌者 +${perTenpai.toLocaleString()} / ノーテン者 -${perNoten.toLocaleString()}`
    )
  }
  return lines
}

export const createRoundResultScene = (
  options: RoundResultSceneOptions
): Container => {
  const root = new Container()
  root.label = 'round-result-scene'

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

  const titleText = options.outcome.kind === 'win' ? '和了' : '流局'
  const title = makeText(titleText, 36, TEXT_PRIMARY_COLOR, 'center', 'bold')
  title.anchor.set(0.5)
  title.x = cx
  title.y = 100
  root.addChild(title)

  const lines =
    options.outcome.kind === 'win'
      ? summarizeWin(options.outcome, options.getPlayerName)
      : summarizeDraw(options.outcome, options.getPlayerName)

  lines.forEach((line, i) => {
    const color = i === 0 ? TEXT_DANGER_COLOR : TEXT_PRIMARY_COLOR
    const fontSize = i === 0 ? 22 : 18
    const text = makeText(line, fontSize, color, 'center', i === 0 ? 'bold' : 'normal')
    text.anchor.set(0.5)
    text.x = cx
    text.y = 180 + i * 44
    root.addChild(text)
  })

  const buttonY = STAGE_HEIGHT - 130
  root.addChild(createButton('次局へ', 'round-result-next-button', cx - 210, buttonY, options.onNext))
  root.addChild(
    createButton('タイトルへ', 'round-result-title-button', cx + 10, buttonY, options.onBackToTitle)
  )

  const footerText = makeText(
    '次局ボタンで継続。終局時は別画面に切り替わります。',
    13,
    TEXT_MUTED_COLOR,
    'center'
  )
  footerText.anchor.set(0.5)
  footerText.x = cx
  footerText.y = STAGE_HEIGHT - 56
  root.addChild(footerText)

  return root
}
