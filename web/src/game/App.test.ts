// Issue #2 スモークテスト: App が PixiJS Application を保持し、
// showTableBackground で stage に子が 1 つ追加されることを確認する。
//
// PixiJS の WebGL レンダラは jsdom 環境では init できないため、
// Application の init は呼ばずに stage だけモックする。

import { describe, it, expect } from 'vitest'
import { Container } from 'pixi.js'
import { App } from './App'

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

  it('showAllTilesDemo は label="tile-demo" の Container を stage に 1 つ追加する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    app.showAllTilesDemo()
    expect(stage.children.length).toBe(1)
    const grid = stage.children[0] as Container
    expect(grid.label).toBe('tile-demo')
    // 34 種 + 赤ドラ 3 枚 = 37 子
    expect(grid.children.length).toBe(37)
  })
})
