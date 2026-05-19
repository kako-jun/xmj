// Issue #2 スモークテスト: App が PixiJS Application を保持し、
// showTableBackground で stage に子が 1 つ追加されることを確認する。
//
// PixiJS の WebGL レンダラは jsdom 環境では init できないため、
// Application の init は呼ばずに stage だけモックする。

import { describe, it, expect } from 'vitest'
import { Container, Text } from 'pixi.js'
import { App } from './App'
import { initWithState } from './state'

describe('App', () => {
  it('showTableBackground は stage に背景を 1 つ追加する', () => {
    const stage = new Container()
    // 最低限 stage プロパティを持つ Application モックで足りる。
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    expect(stage.children.length).toBe(0)
    app.showTableBackground()
    expect(stage.children.length).toBe(1)
  })

  it('showInitialTable は label="game-table" の Container を stage に 1 つ追加する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const state = initWithState({ phase: 'game' })
    app.showInitialTable(state)
    expect(stage.children.length).toBe(1)
    const grid = stage.children[0] as Container
    expect(grid.label).toBe('game-table')
    expect(grid.getChildByLabel('center-info')).toBeTruthy()
    expect(grid.getChildByLabel('score-badges')).toBeTruthy()
    expect(grid.getChildByLabel('bottom-area')).toBeTruthy()
  })

  it.each([
    { currentTurn: 0, areaLabel: 'bottom-area', markerText: 'あなたの手番' },
    { currentTurn: 1, areaLabel: 'right-area', markerText: '南家の手番' },
    { currentTurn: 2, areaLabel: 'top-area', markerText: '対面の手番' },
    { currentTurn: 3, areaLabel: 'left-area', markerText: '北家の手番' },
  ] as const)(
    'currentTurn=%s のとき手番マーカーが対応する方角に 1 つだけ出る',
    ({ currentTurn, areaLabel, markerText }) => {
      const stage = new Container()
      const fakeApp = { stage } as unknown as import('pixi.js').Application
      const app = new App(fakeApp)
      const state = initWithState({ phase: 'game', currentTurn })

      app.showInitialTable(state)

      const table = stage.children[0] as Container
      const areas = ['bottom-area', 'right-area', 'top-area', 'left-area'] as const
      const markerCounts = areas.map(label => {
        const area = table.getChildByLabel(label) as Container
        return area.children.length - 2
      })

      expect(markerCounts.reduce((sum, count) => sum + count, 0)).toBe(1)

      const activeArea = table.getChildByLabel(areaLabel) as Container
      const marker = activeArea.children[2] as Container
      const markerLabel = marker.children[1] as Text
      expect(markerLabel.text).toBe(markerText)
    }
  )

  it('showInitialTable を連続実行しても stage 配下は 1 卓のまま', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    app.showInitialTable(initWithState({ phase: 'game', currentTurn: 0 }))
    const firstTable = stage.children[0] as Container
    app.showInitialTable(initWithState({ phase: 'game', currentTurn: 2 }))

    expect(stage.children.length).toBe(1)
    expect(firstTable.destroyed).toBe(true)
    expect((stage.children[0] as Container).label).toBe('game-table')
  })

  it('立直中のプレイヤーだけ score badge に立直表示が出る', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const state = initWithState({
      phase: 'game',
      players: [
        { ...initWithState().players[0], isRiichi: false },
        { ...initWithState().players[1], isRiichi: true },
        { ...initWithState().players[2], isRiichi: false },
        { ...initWithState().players[3], isRiichi: false },
      ],
    })

    app.showInitialTable(state)

    const table = stage.children[0] as Container
    const badges = table.getChildByLabel('score-badges') as Container
    const riichiTexts = badges.children.flatMap(badge =>
      (badge as Container).children.filter(
        child => child instanceof Text && child.text === '立直'
      )
    )

    expect(riichiTexts).toHaveLength(1)
  })
})
