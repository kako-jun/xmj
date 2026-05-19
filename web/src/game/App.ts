import { Application, Graphics } from 'pixi.js'
import { STAGE_WIDTH, STAGE_HEIGHT, TABLE_BG_COLOR } from './constants'
import { createTableScene } from './table'
import type { GameState } from './types'

export class App {
  app: Application

  constructor(app: Application) {
    this.app = app
  }

  /**
   * Wasm 卓の生成に失敗したときのフォールバックとして、単色の卓背景だけを描画する。
   */
  showTableBackground(): void {
    const bg = new Graphics()
    bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: TABLE_BG_COLOR })
    this.app.stage.addChild(bg)
  }

  showInitialTable(gameState: GameState): void {
    const previousChildren = this.app.stage.removeChildren()
    previousChildren.forEach(child => {
      child.destroy({ children: true })
    })
    const table = createTableScene(gameState)
    this.app.stage.addChild(table)
  }
}
