// Issue #2 スモークテスト: App が PixiJS Application を保持し、
// showTableBackground で stage に子が 1 つ追加されることを確認する。
//
// PixiJS の WebGL レンダラは jsdom 環境では init できないため、
// Application の init は呼ばずに stage だけモックする。

import { describe, it, expect, vi, afterEach } from 'vitest'
import { Container, Text } from 'pixi.js'
import { App } from './App'
import { initWithState } from './state'
import type { Tile } from './types'
import { EVENT_LOG_LIMIT } from './constants'

const sampleState = `Round: 1 | Wall: 69 tiles
Dora indicators: 5p
>親 あなた (25000点): 1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to
  河: 9m 1p
   CPU 南 (25000点): 1p 1p 2p 2p 3p 3p 4s 5s 6s na na ht cn
  河: 7s
   CPU 西 (25000点): 4m 5m 6m 7m 8m 9m 3p 4p 5p 6p 7p 8p pe
   CPU 北 (25000点): 1s 1s 2s 2s 3s 3s 4m 4m 5m 5m 6m 6m sa
`

const createBridgeMock = (overrides: Partial<import('./wasm').WasmGameBridge> = {}) =>
  ({
    getGameStateJson: () => sampleState,
    getPlayerScore: () => 25000,
    getPlayerName: (idx: number) => ['あなた', 'CPU 南', 'CPU 西', 'CPU 北'][idx],
    getPlayerDiscards: () => '',
    isPlayerRiichi: () => false,
    getCurrentHandString: () => '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to hk',
    getCurrentPlayerId: () => 0,
    getWallCount: () => 69,
    getDoraIndicators: () => '5p',
    isCurrentPlayerHuman: () => true,
    isCurrentPlayerCpu: () => false,
    isGameOver: () => false,
    drawTile: () => true,
    discardTile: (_tile: Tile) => true,
    executeCpuTurn: () => '5m',
    canRiichi: () => false,
    declareRiichi: () => false,
    destroy: () => undefined,
    ...overrides,
  }) as unknown as import('./wasm').WasmGameBridge

const getTable = (stage: Container): Container => stage.children[0] as Container

const getBottomArea = (stage: Container): Container =>
  getTable(stage).getChildByLabel('bottom-area') as Container

const getHandTile = (stage: Container, label: string): Container => {
  const hand = getBottomArea(stage).getChildByLabel('hand-0') as Container
  return hand.getChildByLabel(label) as Container
}

const getActionButton = (stage: Container, key: string): Container => {
  const actionArea = getBottomArea(stage).getChildByLabel('action-area') as Container
  return actionArea.getChildByLabel(`action-button-${key}`) as Container
}

const getSceneButton = (stage: Container, label: string): Container =>
  (stage.children[0] as Container).getChildByLabel(label) as Container

const getModeCard = (stage: Container, key: string): Container =>
  ((stage.children[0] as Container).getChildByLabel('mode-card-row') as Container).getChildByLabel(
    `mode-card-${key}`
  ) as Container

const getSceneTexts = (stage: Container): Text[] =>
  ((stage.children[0] as Container).children.filter(
    (child): child is Text => child instanceof Text
  ))

/** モード選択→場決め→対局開始までまとめて進める。 */
const walkToGame = (
  app: App,
  stage: Container,
  roll: { d1: number; d2: number }
): void => {
  getSceneButton(stage, 'title-start-button').emit('pointertap', {} as never)
  getSceneButton(stage, 'mode-select-confirm').emit('pointertap', {} as never)
  app.showDiceRollScene(roll)
  getSceneButton(stage, 'dice-roll-start-button').emit('pointertap', {} as never)
}

describe('App', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

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
    expect(grid.getChildByLabel('event-log')).toBeTruthy()
  })

  it('lastDiscard=null のとき中央情報盤に「なし」を表示する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const state = initWithState({ phase: 'game', lastDiscard: null })

    app.showInitialTable(state)

    const centerInfo = getTable(stage).getChildByLabel('center-info') as Container
    const texts = centerInfo.children.filter((child): child is Text => child instanceof Text)

    expect(texts.some(text => text.text === 'なし')).toBe(true)
  })

  it.each([
    { currentTurn: 0, areaLabel: 'bottom-area', markerText: 'あなたの手番' },
    { currentTurn: 1, areaLabel: 'right-area', markerText: 'CPU 南 の手番' },
    { currentTurn: 2, areaLabel: 'top-area', markerText: 'CPU 西 の手番' },
    { currentTurn: 3, areaLabel: 'left-area', markerText: 'CPU 北 の手番' },
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
        return area.getChildrenByLabel('turn-marker', true).length
      })

      expect(markerCounts.reduce((sum, count) => sum + count, 0)).toBe(1)

      const activeArea = table.getChildByLabel(areaLabel) as Container
      const marker = activeArea.getChildByLabel('turn-marker') as Container
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

  it('showInitialTable は保持中の bridge を destroy して切り離す', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let destroyCount = 0

    const bridge = createBridgeMock({
      destroy: () => {
        destroyCount += 1
      },
    })

    app.startGame(bridge, 0)
    app.showInitialTable(initWithState({ phase: 'game' }))

    expect(destroyCount).toBe(1)
    expect(app.bridge).toBe(null)
  })

  it('showTitleScene は title-scene を表示する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    app.showTitleScene()

    expect(stage.children).toHaveLength(1)
    expect((stage.children[0] as Container).label).toBe('title-scene')
    expect(getSceneButton(stage, 'title-start-button')).toBeTruthy()
  })

  it('createBridge が無い title-scene は開始ボタンを無効化し、案内文を表示する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    app.showTitleScene('Wasm 初期化に失敗したため、対局を開始できません。')

    const startButton = getSceneButton(stage, 'title-start-button')
    expect(startButton.eventMode).not.toBe('static')
    expect(
      getSceneTexts(stage).some(text =>
        text.text.includes('Wasm 初期化に失敗したため、対局を開始できません。')
      )
    ).toBe(true)
  })

  it('title-start-button はモード選択シーンへ遷移する (bridge は作らない)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const createBridge = vi.fn(() => createBridgeMock())
    const app = new App(fakeApp, { createBridge })

    app.showTitleScene()
    getSceneButton(stage, 'title-start-button').emit('pointertap', {} as never)

    expect(createBridge).not.toHaveBeenCalled()
    expect((stage.children[0] as Container).label).toBe('mode-select-scene')
    expect(app.bridge).toBe(null)
  })

  it('mode-select → dice-roll → game-table の完全フローを通る', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const createBridge = vi.fn(() =>
      createBridgeMock({
        getCurrentPlayerId: () => 1,
        isCurrentPlayerHuman: () => true,
        isCurrentPlayerCpu: () => false,
        getPlayerName: idx => ['CPU 東', 'あなた', 'CPU 西', 'CPU 北'][idx],
        getGameStateJson: () => `Round: 1 | Wall: 69 tiles
Dora indicators: 5p
 親 CPU 東 (25000点): 1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to
  河: 9m 1p
>あなた (25000点): 1p 1p 2p 2p 3p 3p 4s 5s 6s na na ht cn
  河: 7s
 CPU 西 (25000点): 4m 5m 6m 7m 8m 9m 3p 4p 5p 6p 7p 8p pe
 CPU 北 (25000点): 1s 1s 2s 2s 3s 3s 4m 4m 5m 5m 6m 6m sa`,
      })
    )
    // d1=2, d2=1 → 合計3 → ((3-2)%4)=1 → 南家(=PlayerIndex 1)
    const app = new App(fakeApp, { createBridge })

    app.showTitleScene()
    // タイトル → モード選択
    getSceneButton(stage, 'title-start-button').emit('pointertap', {} as never)
    expect((stage.children[0] as Container).label).toBe('mode-select-scene')

    // モード選択 → 場決め (デフォルト tonpuusen のまま「次へ」)
    getSceneButton(stage, 'mode-select-confirm').emit('pointertap', {} as never)
    // テスト用に明示的な dice を注入する
    app.showDiceRollScene({ d1: 2, d2: 1 })
    expect((stage.children[0] as Container).label).toBe('dice-roll-scene')
    expect(app.selectedHumanSeat).toBe(1)

    // 場決め → 対局開始
    getSceneButton(stage, 'dice-roll-start-button').emit('pointertap', {} as never)

    expect(createBridge).toHaveBeenCalledTimes(1)
    expect(createBridge).toHaveBeenCalledWith(1)
    expect(app.humanPlayerIndex).toBe(1)
    expect((stage.children[0] as Container).label).toBe('game-table')
    const bottomArea = getBottomArea(stage)
    expect(bottomArea.getChildByLabel('hand-1')).toBeTruthy()
    expect(bottomArea.getChildByLabel('hand-0')).toBeNull()
  })

  it('mode-select-back でタイトルに戻る', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp, { createBridge: () => createBridgeMock() })

    app.showTitleScene()
    getSceneButton(stage, 'title-start-button').emit('pointertap', {} as never)
    getSceneButton(stage, 'mode-select-back').emit('pointertap', {} as never)

    expect((stage.children[0] as Container).label).toBe('title-scene')
  })

  it('rollDice オプションで決定的なサイコロを注入できる', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const rollDice = vi.fn(() => ({ d1: 3, d2: 3 })) // 合計6 → (6-2)%4=0 → 東家
    const app = new App(fakeApp, {
      createBridge: () => createBridgeMock(),
      rollDice,
    })

    app.showTitleScene()
    getSceneButton(stage, 'title-start-button').emit('pointertap', {} as never)
    getSceneButton(stage, 'mode-select-confirm').emit('pointertap', {} as never)

    expect(rollDice).toHaveBeenCalledTimes(1)
    expect(app.selectedHumanSeat).toBe(0)
    expect((stage.children[0] as Container).label).toBe('dice-roll-scene')
  })

  it('場決め後の dice-roll-start で createBridge が例外を投げたら title-scene に戻り理由を表示する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp, {
      createBridge: () => {
        throw new Error('boom')
      },
    })

    app.showTitleScene()
    walkToGame(app, stage, { d1: 1, d2: 1 })

    expect((stage.children[0] as Container).label).toBe('title-scene')
    expect(app.bridge).toBe(null)
    expect(
      getSceneTexts(stage).some(text => text.text.includes('対局の初期化に失敗しました: boom'))
    ).toBe(true)
  })

  it('mode-card で半荘戦を選んでも enabled=false のため確定ボタンが反応しない', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp, { createBridge: () => createBridgeMock() })

    app.showTitleScene()
    getSceneButton(stage, 'title-start-button').emit('pointertap', {} as never)
    // 半荘戦は disabled なのでカード自体が pointertap を受け付けない
    const hanchanCard = getModeCard(stage, 'hanchan')
    expect(hanchanCard.eventMode).not.toBe('static')
    // 東風戦はちゃんと取れる
    expect(getModeCard(stage, 'tonpuusen')).toBeTruthy()
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

  it('startGame は人間手番かつ 13 枚なら自動で drawTile して 14 枚にする', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let drawCount = 0

    const bridge = createBridgeMock({
      getCurrentHandString: () =>
        drawCount === 0
          ? '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to'
          : '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to hk',
      drawTile: () => {
        drawCount += 1
        return true
      },
    })

    app.startGame(bridge, 0)

    expect(drawCount).toBe(1)
    expect(app.gameState?.players[0].hand).toHaveLength(14)
  })

  it('手牌タップで選択状態になり、同じ牌の再タップで discardTile が走る', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const discarded: Tile[] = []

    const bridge = createBridgeMock({
      drawTile: () => false,
      discardTile: (tile: Tile) => {
        discarded.push(tile)
        return true
      },
    })

    app.startGame(bridge, 0)

    const getTargetTile = (): Container => {
      const table = stage.children[0] as Container
      const bottomArea = table.getChildByLabel('bottom-area') as Container
      const hand = bottomArea.getChildByLabel('hand-0') as Container
      return hand.getChildByLabel('1m-0') as Container
    }

    getTargetTile().emit('pointertap', {} as never)
    expect(app.selectedHandIndex).toBe(0)

    getTargetTile().emit('pointertap', {} as never)
    expect(discarded).toEqual([{ suit: 'man', value: 1 }])
    expect(app.selectedHandIndex).toBe(null)
  })

  it('打牌後は CPU ターンを回し、人間に戻ったら自動ツモして再描画する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    let drawCount = 0
    let discardCount = 0
    let cpuCount = 0
    let currentPlayerId = 0

    const bridge = createBridgeMock({
      drawTile: () => {
        drawCount += 1
        return true
      },
      discardTile: () => {
        discardCount += 1
        currentPlayerId = 1
        return true
      },
      executeCpuTurn: () => {
        cpuCount += 1
        currentPlayerId = currentPlayerId === 3 ? 0 : (currentPlayerId + 1)
        return '5m'
      },
      getCurrentPlayerId: () => currentPlayerId,
      isCurrentPlayerHuman: () => currentPlayerId === 0,
      isCurrentPlayerCpu: () => currentPlayerId !== 0,
      getCurrentHandString: () =>
        drawCount >= 1
          ? '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to hk'
          : '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to',
    })

    app.startGame(bridge, 0)
    drawCount = 0

    app.selectedHandIndex = 0
    const result = (
      app as unknown as {
        confirmSelectedTile: () => boolean
      }
    ).confirmSelectedTile()

    expect(result).toBe(true)
    expect(discardCount).toBe(1)
    expect(cpuCount).toBe(3)
    expect(drawCount).toBe(1)
    expect(app.gameState?.currentTurn).toBe(0)
    expect(app.gameState?.players[0].hand).toHaveLength(14)
  })

  it('立直成功時は選択牌を打牌し、そのまま CPU ループに入る', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    let drawCount = 0
    let discardCount = 0
    let declareRiichiCount = 0
    let cpuCount = 0
    let currentPlayerId = 0

    const bridge = createBridgeMock({
      canRiichi: () => true,
      declareRiichi: () => {
        declareRiichiCount += 1
        return true
      },
      drawTile: () => {
        drawCount += 1
        return true
      },
      discardTile: () => {
        discardCount += 1
        currentPlayerId = 1
        return true
      },
      executeCpuTurn: () => {
        cpuCount += 1
        currentPlayerId = currentPlayerId === 3 ? 0 : (currentPlayerId + 1)
        return '5m'
      },
      getCurrentPlayerId: () => currentPlayerId,
      isCurrentPlayerHuman: () => currentPlayerId === 0,
      isCurrentPlayerCpu: () => currentPlayerId !== 0,
      getCurrentHandString: () =>
        drawCount >= 1
          ? '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to hk'
          : '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to',
    })

    app.startGame(bridge, 0)
    drawCount = 0

    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    getActionButton(stage, 'riichi-discard').emit('pointertap', {} as never)

    expect(declareRiichiCount).toBe(1)
    expect(discardCount).toBe(1)
    expect(cpuCount).toBe(3)
    expect(drawCount).toBe(1)
    expect(app.selectedHandIndex).toBe(null)
    expect(app.gameState?.currentTurn).toBe(0)
    expect(app.gameState?.players[0].hand).toHaveLength(14)
  })

  it('drawTile が false のとき startGame は 13 枚のまま維持する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let drawCount = 0

    const bridge = createBridgeMock({
      getCurrentHandString: () => '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to',
      drawTile: () => {
        drawCount += 1
        return false
      },
    })

    app.startGame(bridge, 0)

    expect(drawCount).toBe(1)
    expect(app.gameState?.players[0].hand).toHaveLength(13)
    expect(stage.children.length).toBe(1)
    expect(app.eventLog.some(entry => entry.includes('あなた がツモ'))).toBe(false)
  })

  it('bridge 差し替え時は旧 bridge を一度だけ destroy する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let oldDestroyCount = 0
    let newDestroyCount = 0

    const oldBridge = createBridgeMock({
      destroy: () => {
        oldDestroyCount += 1
      },
    })
    const newBridge = createBridgeMock({
      destroy: () => {
        newDestroyCount += 1
      },
    })

    app.startGame(oldBridge, 0)
    app.startGame(newBridge, 0)
    app.showInitialTable(initWithState({ phase: 'game' }))

    expect(oldDestroyCount).toBe(1)
    expect(newDestroyCount).toBe(1)
  })

  it('飛び終局時は result-scene に遷移して順位一覧を表示する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    let currentPlayerId = 0
    let destroyed = 0
    const bridge = createBridgeMock({
      drawTile: () => false,
      discardTile: () => {
        currentPlayerId = 1
        return true
      },
      executeCpuTurn: () => {
        currentPlayerId = 1
        return '5m'
      },
      getCurrentPlayerId: () => currentPlayerId,
      isCurrentPlayerHuman: () => currentPlayerId === 0,
      isCurrentPlayerCpu: () => currentPlayerId !== 0,
      isGameOver: () => currentPlayerId === 1,
      getPlayerScore: idx => [25000, -800, 24000, 18000][idx] ?? 0,
      getGameStateJson: () => `Round: 1 | Wall: 1 tiles
Dora indicators: 5p
>親 あなた (25000点): 1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to hk
  河: 9m 1p
   CPU 南 (-800点): 1p 1p 2p 2p 3p 3p 4s 5s 6s na na ht cn
  河: 7s
   CPU 西 (24000点): 4m 5m 6m 7m 8m 9m 3p 4p 5p 6p 7p 8p pe
   CPU 北 (18000点): 1s 1s 2s 2s 3s 3s 4m 4m 5m 5m 6m 6m sa
Last discard: 5m`,
      destroy: () => {
        destroyed += 1
      },
    })

    app.startGame(bridge, 0)
    app.selectedHandIndex = 0
    ;(app as unknown as { confirmSelectedTile: () => boolean }).confirmSelectedTile()

    const resultScene = stage.children[0] as Container
    const texts = resultScene.children.filter((child): child is Text => child instanceof Text)

    expect(resultScene.label).toBe('result-scene')
    expect(texts.some(text => text.text === 'CPU 南 が飛んで終局')).toBe(true)
    expect(texts.some(text => text.text === '1位')).toBe(true)
    expect(texts.some(text => text.text === '現 API では未取得')).toBe(true)
    expect(app.bridge).toBe(null)
    expect(destroyed).toBe(1)
  })

  it('result-scene の再戦ボタンでモード選択シーンに戻り、もう一度走らせると新しい bridge ができる', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application

    let createCount = 0
    const finishedBridge = createBridgeMock({
      isGameOver: () => true,
      getPlayerScore: idx => [25000, -1000, 24000, 20000][idx] ?? 0,
    })
    const freshBridge = createBridgeMock()
    const app = new App(fakeApp, {
      createBridge: () => {
        createCount += 1
        return freshBridge
      },
    })

    app.startGame(finishedBridge, 0)
    ;(app as unknown as { finalizeGameIfNeeded: () => void }).finalizeGameIfNeeded()
    getSceneButton(stage, 'result-rematch-button').emit('pointertap', {} as never)

    // 再戦は場決めをやり直すのでまずモード選択に戻る
    expect((stage.children[0] as Container).label).toBe('mode-select-scene')

    // モード選択 → 場決め (d1=2,d2=2 → 合計4 → (4-2)%4=2 → 西家) → 開始
    getSceneButton(stage, 'mode-select-confirm').emit('pointertap', {} as never)
    app.showDiceRollScene({ d1: 2, d2: 2 })
    getSceneButton(stage, 'dice-roll-start-button').emit('pointertap', {} as never)

    expect(createCount).toBe(1)
    expect((stage.children[0] as Container).label).toBe('game-table')
    expect(app.bridge).toBe(freshBridge)
  })

  it('result-scene のタイトルボタンで title-scene に戻る', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const finishedBridge = createBridgeMock({
      isGameOver: () => true,
      getPlayerScore: idx => [25000, -1000, 24000, 20000][idx] ?? 0,
    })

    app.startGame(finishedBridge, 0)
    ;(app as unknown as { finalizeGameIfNeeded: () => void }).finalizeGameIfNeeded()
    getSceneButton(stage, 'result-title-button').emit('pointertap', {} as never)

    expect((stage.children[0] as Container).label).toBe('title-scene')
    expect(app.bridge).toBe(null)
  })

  it('discard action button から打牌できる', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const discarded: Tile[] = []

    const bridge = createBridgeMock({
      drawTile: () => false,
      discardTile: (tile: Tile) => {
        discarded.push(tile)
        return true
      },
    })

    app.startGame(bridge, 0)

    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    getActionButton(stage, 'discard').emit('pointertap', {} as never)

    expect(discarded).toEqual([{ suit: 'man', value: 1 }])
    expect(app.selectedHandIndex).toBe(null)
  })

  it('discardTile が false のとき選択状態を維持して CPU ターンへ進まない', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let cpuCount = 0

    const bridge = createBridgeMock({
      drawTile: () => false,
      discardTile: () => false,
      executeCpuTurn: () => {
        cpuCount += 1
        return '5m'
      },
    })

    app.startGame(bridge, 0)

    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    getActionButton(stage, 'discard').emit('pointertap', {} as never)

    expect(app.selectedHandIndex).toBe(0)
    expect(cpuCount).toBe(0)
    expect(app.gameState?.players[0].discards).toHaveLength(0)
  })

  it('declareRiichi が true でも discardTile が false のとき手番を止めたまま選択状態を維持する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let riichiCount = 0
    let cpuCount = 0

    const bridge = createBridgeMock({
      drawTile: () => false,
      canRiichi: () => true,
      declareRiichi: () => {
        riichiCount += 1
        return true
      },
      discardTile: () => false,
      executeCpuTurn: () => {
        cpuCount += 1
        return '5m'
      },
    })

    app.startGame(bridge, 0)

    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    getActionButton(stage, 'riichi-discard').emit('pointertap', {} as never)

    expect(riichiCount).toBe(1)
    expect(cpuCount).toBe(0)
    expect(app.selectedHandIndex).toBe(0)
    expect(app.gameState?.currentTurn).toBe(0)
    expect(app.gameState?.players[0].hand).toHaveLength(14)
    expect(app.gameState?.players[0].discards).toHaveLength(0)
  })

  it('CPU 手番では手牌をタップしても選択できない', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    const bridge = createBridgeMock({
      drawTile: () => false,
      getCurrentPlayerId: () => 1,
      isCurrentPlayerHuman: () => false,
      isCurrentPlayerCpu: () => true,
    })

    app.startGame(bridge, 0)

    const hand = getBottomArea(stage).getChildByLabel('hand-0') as Container
    const tile = hand.children[0] as Container
    expect(hand.getChildByLabel('1m-0')).toBeNull()
    expect(tile.eventMode).not.toBe('static')
    tile.emit('pointertap', {} as never)

    expect(app.selectedHandIndex).toBe(null)
  })

  it('canRiichi=true かつ牌選択中のときだけ確定ボタンが立直表示になる', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    app.startGame(
      createBridgeMock({
        drawTile: () => false,
        canRiichi: () => false,
      }),
      0
    )
    expect(getActionButton(stage, 'discard')).toBeTruthy()
    expect(getActionButton(stage, 'riichi')).toBeNull()
    expect(getActionButton(stage, 'riichi-discard')).toBeNull()

    app.startGame(
      createBridgeMock({
        drawTile: () => false,
        canRiichi: () => true,
      }),
      0
    )
    expect(getActionButton(stage, 'discard')).toBeTruthy()
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    expect(getActionButton(stage, 'discard')).toBeNull()
    expect(getActionButton(stage, 'riichi-discard')).toBeTruthy()
  })

  it('declareRiichi が false のとき選択状態を維持する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let riichiCount = 0

    const bridge = createBridgeMock({
      drawTile: () => false,
      canRiichi: () => true,
      declareRiichi: () => {
        riichiCount += 1
        return false
      },
    })

    app.startGame(bridge, 0)

    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    getActionButton(stage, 'riichi-discard').emit('pointertap', {} as never)

    expect(riichiCount).toBe(1)
    expect(app.selectedHandIndex).toBe(0)
    expect(getActionButton(stage, 'riichi-discard')).toBeTruthy()
  })

  it('ゲームオーバー時は result-scene に切り替わる', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let gameOver = false

    const bridge = createBridgeMock({
      drawTile: () => false,
      discardTile: () => {
        gameOver = true
        return true
      },
      isGameOver: () => gameOver,
      getWallCount: () => 0,
    })

    app.startGame(bridge, 0)
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    getActionButton(stage, 'discard').emit('pointertap', {} as never)

    expect(app.resultMessage).toBe('山牌が尽きて終局')
    expect((stage.children[0] as Container).label).toBe('result-scene')
  })

  it('飛び終了時は終局理由に飛んだプレイヤー名を表示する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let gameOver = false

    const bridge = createBridgeMock({
      drawTile: () => false,
      discardTile: () => {
        gameOver = true
        return true
      },
      isGameOver: () => gameOver,
      getWallCount: () => 12,
      getPlayerScore: (idx: number) => (idx === 2 ? 0 : 25000),
      getPlayerName: (idx: number) => ['あなた', 'CPU 南', 'CPU 西', 'CPU 北'][idx],
    })

    app.startGame(bridge, 0)
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    getActionButton(stage, 'discard').emit('pointertap', {} as never)

    expect(app.resultMessage).toBe('CPU 西 が飛んで終局')
    expect(app.eventLog[app.eventLog.length - 1]).toBe('CPU 西 が飛んで終局')
    expect((stage.children[0] as Container).label).toBe('result-scene')
  })

  it('山牌切れでも飛びでもないゲームオーバー時は汎用の終局文言を表示する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let gameOver = false

    const bridge = createBridgeMock({
      drawTile: () => false,
      discardTile: () => {
        gameOver = true
        return true
      },
      isGameOver: () => gameOver,
      getWallCount: () => 12,
      getPlayerScore: () => 25000,
    })

    app.startGame(bridge, 0)
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    getActionButton(stage, 'discard').emit('pointertap', {} as never)

    expect(app.resultMessage).toBe('対局終了')
    expect(app.eventLog[app.eventLog.length - 1]).toBe('対局終了')
    expect((stage.children[0] as Container).label).toBe('result-scene')
  })

  it('非同期 CPU ターン中に advanceTurnLoop を再入しても CPU 実行は重複しない', async () => {
    vi.useFakeTimers()

    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp, { cpuTurnDelayMs: 10 })

    let currentPlayerId = 1
    let cpuCount = 0
    let drawCount = 0

    const bridge = createBridgeMock({
      drawTile: () => {
        drawCount += 1
        return true
      },
      getCurrentPlayerId: () => currentPlayerId,
      isCurrentPlayerHuman: () => currentPlayerId === 0,
      isCurrentPlayerCpu: () => currentPlayerId !== 0,
      executeCpuTurn: () => {
        cpuCount += 1
        currentPlayerId = currentPlayerId === 3 ? 0 : ((currentPlayerId + 1) as 0 | 1 | 2 | 3)
        return '5m'
      },
      getCurrentHandString: () =>
        drawCount >= 1
          ? '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to hk'
          : '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to',
    })

    app.startGame(bridge, 0)

    const internalApp = app as unknown as {
      advanceTurnLoop: () => void
      cpuTurnTask: Promise<void> | null
    }
    internalApp.advanceTurnLoop()
    internalApp.advanceTurnLoop()

    await vi.runAllTimersAsync()
    await internalApp.cpuTurnTask

    expect(cpuCount).toBe(3)
    expect(drawCount).toBe(1)
    expect(app.gameState?.currentTurn).toBe(0)
  })

  it('新しい startGame 後は古い非同期 cpuTurnTask が新しい bridge に混線しない', async () => {
    vi.useFakeTimers()

    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp, { cpuTurnDelayMs: 10 })

    let oldCurrentPlayerId = 1
    let oldCpuCount = 0
    const oldBridge = createBridgeMock({
      getCurrentPlayerId: () => oldCurrentPlayerId,
      isCurrentPlayerHuman: () => oldCurrentPlayerId === 0,
      isCurrentPlayerCpu: () => oldCurrentPlayerId !== 0,
      executeCpuTurn: () => {
        oldCpuCount += 1
        oldCurrentPlayerId = 0
        return '9m'
      },
    })

    app.startGame(oldBridge, 0)
    ;(app as unknown as { advanceTurnLoop: () => void }).advanceTurnLoop()

    const newBridge = createBridgeMock({
      drawTile: () => false,
      getCurrentPlayerId: () => 0,
      isCurrentPlayerHuman: () => true,
      isCurrentPlayerCpu: () => false,
      getPlayerName: (idx: number) => ['新しいあなた', 'CPU 南', 'CPU 西', 'CPU 北'][idx],
    })

    app.startGame(newBridge, 0)

    await vi.runAllTimersAsync()

    expect(oldCpuCount).toBe(0)
    expect(app.bridge).toBe(newBridge)
    expect(app.eventLog).toEqual(['対局開始'])
    expect(app.gameState?.currentTurn).toBe(0)
  })

  it('showInitialTable は進行中の非同期 cpuTurnTask を無効化する', async () => {
    vi.useFakeTimers()

    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp, { cpuTurnDelayMs: 10 })

    let oldCurrentPlayerId = 1
    let oldCpuCount = 0
    const oldBridge = createBridgeMock({
      getCurrentPlayerId: () => oldCurrentPlayerId,
      isCurrentPlayerHuman: () => oldCurrentPlayerId === 0,
      isCurrentPlayerCpu: () => oldCurrentPlayerId !== 0,
      executeCpuTurn: () => {
        oldCpuCount += 1
        oldCurrentPlayerId = 0
        return '9m'
      },
    })

    app.startGame(oldBridge, 0)
    ;(app as unknown as { advanceTurnLoop: () => void }).advanceTurnLoop()
    app.showInitialTable(initWithState({ phase: 'game', currentTurn: 2 }))

    await vi.runAllTimersAsync()

    expect(oldCpuCount).toBe(0)
    expect(app.bridge).toBe(null)
    expect(app.eventLog).toEqual([])
    expect(app.gameState?.currentTurn).toBe(2)
  })

  it('打牌ログに人間と CPU のイベントが積まれる', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    let currentPlayerId = 0
    let drawCount = 0

    const bridge = createBridgeMock({
      drawTile: () => {
        drawCount += 1
        return true
      },
      discardTile: () => {
        currentPlayerId = 1
        return true
      },
      executeCpuTurn: () => {
        currentPlayerId = currentPlayerId === 3 ? 0 : (currentPlayerId + 1)
        return '5m'
      },
      getCurrentPlayerId: () => currentPlayerId,
      isCurrentPlayerHuman: () => currentPlayerId === 0,
      isCurrentPlayerCpu: () => currentPlayerId !== 0,
      getCurrentHandString: () =>
        currentPlayerId === 0 && drawCount >= 2
          ? '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to hk'
          : '1m 2m 3m 4m 5mr 6m 7p 8p 9p 2s 3s 4s to',
    })

    app.startGame(bridge, 0)

    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    getActionButton(stage, 'discard').emit('pointertap', {} as never)

    expect(app.eventLog.some(entry => entry.includes('あなた が 1m を打牌'))).toBe(true)
    expect(app.eventLog.some(entry => entry.includes('CPU 南 がツモ'))).toBe(true)
    expect(app.eventLog.some(entry => entry.includes('CPU 南 が 5m を打牌'))).toBe(true)
    expect(app.eventLog.some(entry => entry.includes('あなた がツモ'))).toBe(true)
    expect(app.eventLog.some(entry => entry.includes('思考中'))).toBe(false)
  })

  it('eventLog は 12 件を上限に古い順から切り詰める', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const appendLog = (app as unknown as { appendLog: (message: string) => void }).appendLog

    for (let index = 1; index <= 14; index += 1) {
      appendLog.call(app, `log-${index}`)
    }

    expect(app.eventLog).toHaveLength(EVENT_LOG_LIMIT)
    expect(app.eventLog[0]).toBe('log-3')
    expect(app.eventLog[app.eventLog.length - 1]).toBe('log-14')
  })
})
