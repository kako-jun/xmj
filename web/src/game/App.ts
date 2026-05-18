// xmj Web 版エントリ。シーン基盤は Issue #5 以降で本実装。
// Issue #2 段階では PixiJS Application を保持して空ステージを描く。
// Issue #4 で全 34 種類の牌を並べる確認用ステージを追加した。

import { Application, Container, Graphics } from 'pixi.js'
import { STAGE_WIDTH, STAGE_HEIGHT, TABLE_BG_COLOR, TILE } from './constants'
import { createTileGraphics, enumerateAllTiles } from './tile'

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

  /**
   * Issue #4 の動作確認用: 全 34 種類の牌を 9 列 x 4 行で並べる。
   * 5m / 5p / 5s は赤ドラとしても 1 枚ずつ追加。
   * 後の Issue で GameScene に置き換える。
   */
  showAllTilesDemo(): void {
    const grid = new Container()
    grid.label = 'tile-demo'

    const cols = 9
    const padX = 8
    const padY = 12
    const tiles = [
      ...enumerateAllTiles(),
      // 赤ドラ
      { suit: 'man', value: 5, isRed: true } as const,
      { suit: 'pin', value: 5, isRed: true } as const,
      { suit: 'sou', value: 5, isRed: true } as const,
    ]

    tiles.forEach((tile, i) => {
      const col = i % cols
      const row = Math.floor(i / cols)
      const g = createTileGraphics(tile)
      g.x = col * (TILE.width + padX)
      g.y = row * (TILE.height + padY)
      grid.addChild(g)
    })

    // 中央に寄せる
    const gridWidth = cols * (TILE.width + padX) - padX
    const gridHeight = Math.ceil(tiles.length / cols) * (TILE.height + padY) - padY
    grid.x = (STAGE_WIDTH - gridWidth) / 2
    grid.y = (STAGE_HEIGHT - gridHeight) / 2

    this.app.stage.addChild(grid)
  }
}
