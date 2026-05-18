// xmj Web 版エントリ。シーン基盤は Issue #5 以降で本実装。
// Issue #2 段階では PixiJS Application を保持して空ステージを描くだけ。

import { Application, Graphics } from 'pixi.js'
import { STAGE_WIDTH, STAGE_HEIGHT, TABLE_BG_COLOR } from './constants'

export class App {
  app: Application

  constructor(app: Application) {
    this.app = app
  }

  /**
   * 麻雀卓の背景を描画する。Issue #2 では「緑画面が出る」ことが完了条件。
   * Issue #5 で実 GameScene に置き換える。
   */
  showTableBackground(): void {
    const bg = new Graphics()
    bg.rect(0, 0, STAGE_WIDTH, STAGE_HEIGHT).fill({ color: TABLE_BG_COLOR })
    this.app.stage.addChild(bg)
  }
}
