// 中間結果シーンの描画・ボタンテスト (Issue #27)

import { describe, expect, it, vi } from 'vitest'
import { Container, Text } from 'pixi.js'
import { createRoundResultScene } from './roundResultScene'
import type { PlayerIndex, RoundOutcome } from './types'

const namer = (idx: PlayerIndex): string =>
  ['東家', '南家', '西家', '北家'][idx] ?? `P${idx}`

describe('createRoundResultScene', () => {
  it('和了結果のラベル・スコア・役を描画する', () => {
    const outcome: RoundOutcome = {
      kind: 'win',
      data: {
        winner: 1,
        winType: 'ron',
        from: 2,
        han: 3,
        fu: 40,
        totalPoints: 5200,
        yaku: ['Riichi', 'Tanyao'],
      },
    }
    const scene = createRoundResultScene({
      outcome,
      getPlayerName: namer,
      onNext: () => undefined,

    })
    expect(scene.label).toBe('round-result-scene')

    const texts = collectTexts(scene).map(t => t.text)
    expect(texts).toContain('和了')
    expect(texts.some(t => t.includes('南家 が 西家 からロン和了'))).toBe(true)
    expect(texts.some(t => t.includes('3飜') && t.includes('5,200 点'))).toBe(true)
    // yakuLabels.ts により Rust の Debug 表記が日本語に変換される
    expect(texts.some(t => t.includes('立直') && t.includes('断幺九'))).toBe(true)
  })

  it('流局結果でテンパイ者名を描画する', () => {
    const outcome: RoundOutcome = {
      kind: 'draw',
      data: { tenpaiPlayers: [0, 3] },
    }
    const scene = createRoundResultScene({
      outcome,
      getPlayerName: namer,
      onNext: () => undefined,

    })
    const texts = collectTexts(scene).map(t => t.text)
    expect(texts).toContain('流局')
    expect(texts.some(t => t.includes('テンパイ: 東家 / 北家'))).toBe(true)
  })

  it('全員ノーテン時は専用ラベルを出す', () => {
    const outcome: RoundOutcome = {
      kind: 'draw',
      data: { tenpaiPlayers: [] },
    }
    const scene = createRoundResultScene({
      outcome,
      getPlayerName: namer,
      onNext: () => undefined,

    })
    const texts = collectTexts(scene).map(t => t.text)
    expect(texts.some(t => t.includes('全員ノーテン'))).toBe(true)
  })

  it('流局 1 テンパイで罰符 +3000 / -1000 を表示する', () => {
    const outcome: RoundOutcome = {
      kind: 'draw',
      data: { tenpaiPlayers: [2] },
    }
    const scene = createRoundResultScene({
      outcome,
      getPlayerName: namer,
      onNext: () => undefined,

    })
    const texts = collectTexts(scene).map(t => t.text)
    expect(
      texts.some(t => t.includes('聴牌者 +3,000') && t.includes('ノーテン者 -1,000'))
    ).toBe(true)
  })

  it('流局 2 テンパイで罰符 ±1500 を表示する', () => {
    const outcome: RoundOutcome = {
      kind: 'draw',
      data: { tenpaiPlayers: [0, 1] },
    }
    const scene = createRoundResultScene({
      outcome,
      getPlayerName: namer,
      onNext: () => undefined,

    })
    const texts = collectTexts(scene).map(t => t.text)
    expect(
      texts.some(t => t.includes('聴牌者 +1,500') && t.includes('ノーテン者 -1,500'))
    ).toBe(true)
  })

  it('流局 3 テンパイで罰符 +1000 / -3000 を表示する', () => {
    const outcome: RoundOutcome = {
      kind: 'draw',
      data: { tenpaiPlayers: [0, 1, 2] },
    }
    const scene = createRoundResultScene({
      outcome,
      getPlayerName: namer,
      onNext: () => undefined,

    })
    const texts = collectTexts(scene).map(t => t.text)
    expect(
      texts.some(t => t.includes('聴牌者 +1,000') && t.includes('ノーテン者 -3,000'))
    ).toBe(true)
  })

  it('流局 4 テンパイ・0 テンパイは罰符無しと表示する', () => {
    for (const tenpai of [[], [0, 1, 2, 3]]) {
      const outcome: RoundOutcome = {
        kind: 'draw',
        data: { tenpaiPlayers: tenpai as PlayerIndex[] },
      }
      const scene = createRoundResultScene({
        outcome,
        getPlayerName: namer,
        onNext: () => undefined,
  
      })
      const texts = collectTexts(scene).map(t => t.text)
      expect(texts.some(t => t.includes('ノーテン罰符') && t.includes('なし'))).toBe(true)
    }
  })

  it('「次局へ」ボタンで onNext が発火し、タイトルへ戻るボタンは存在しない (#90)', () => {
    const onNext = vi.fn()
    const outcome: RoundOutcome = {
      kind: 'draw',
      data: { tenpaiPlayers: [] },
    }
    const scene = createRoundResultScene({
      outcome,
      getPlayerName: namer,
      onNext,
    })
    const next = scene.getChildByLabel('round-result-next-button') as Container
    const back = scene.getChildByLabel('round-result-title-button')
    expect(next).toBeTruthy()
    // #90: 局終了画面にタイトル戻り導線は出さない
    expect(back).toBeNull()
    next.emit('pointertap', {} as never)
    expect(onNext).toHaveBeenCalledTimes(1)
  })
})

const collectTexts = (root: Container): Text[] => {
  const out: Text[] = []
  const walk = (c: Container): void => {
    for (const child of c.children) {
      if (child instanceof Text) out.push(child)
      if (child instanceof Container) walk(child)
    }
  }
  walk(root)
  return out
}
