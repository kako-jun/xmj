// Issue #2 スモークテスト: App が PixiJS Application を保持し、
// showTableBackground で stage に子が 1 つ追加されることを確認する。
//
// PixiJS の WebGL レンダラは jsdom 環境では init できないため、
// Application の init は呼ばずに stage だけモックする。
//
// 対局情報・行動ボタン・実況ログは Pixi ではなく HTML オーバーレイ (#ui-side) に
// 描画されるので、beforeEach で index.html 相当の DOM を組み立てる。

import { describe, it, expect, vi, afterEach, beforeEach } from 'vitest'
import { Container, Text } from 'pixi.js'
import { App } from './App'
import { initWithState } from './state'
import type { Tile } from './types'
import { EVENT_LOG_LIMIT } from './constants'

const UI_SIDE_HTML = `
  <aside id="ui-side">
    <span data-ui="round"></span>
    <span data-ui="honba"></span>
    <span data-ui="wall"></span>
    <span data-ui="dora"></span>
    <div data-ui="scores"></div>
    <div data-ui="actions"></div>
    <div data-ui="hint"></div>
    <div data-ui="log"></div>
  </aside>
`

const setupDom = (): HTMLElement => {
  document.body.innerHTML = UI_SIDE_HTML
  return document.getElementById('ui-side') as HTMLElement
}

const findActionButton = (key: string): HTMLButtonElement | null =>
  document.querySelector<HTMLButtonElement>(`button[data-action-key="${key}"]`)

const clickActionButton = (key: string): void => {
  const btn = findActionButton(key)
  if (!btn) throw new Error(`HTML action button "${key}" not found`)
  if (btn.disabled) throw new Error(`HTML action button "${key}" is disabled`)
  btn.click()
}

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
    canTsumo: () => false,
    canRon: () => false,
    getLastDiscarder: () => undefined,
    resolveWinTsumo: () => null,
    resolveWinRon: () => null,
    computeTenpaiPlayers: () => [],
    destroy: () => undefined,
    ...overrides,
  }) as unknown as import('./wasm').WasmGameBridge

const getTable = (stage: Container): Container => stage.children[0] as Container

const getHand = (stage: Container, playerId: 0 | 1 | 2 | 3): Container =>
  getTable(stage).getChildByLabel(`hand-${playerId}`) as Container

const getHandTile = (stage: Container, label: string): Container => {
  const hand = getHand(stage, 0)
  return hand.getChildByLabel(label) as Container
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
  beforeEach(() => {
    setupDom()
  })

  afterEach(() => {
    vi.useRealTimers()
    document.body.innerHTML = ''
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

  it('showInitialTable は label="game-table" の Container と 4 プレイヤーぶんの hand/discards を含む', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const state = initWithState({ phase: 'game' })
    app.showInitialTable(state)
    expect(stage.children.length).toBe(1)
    const grid = stage.children[0] as Container
    expect(grid.label).toBe('game-table')
    expect(grid.getChildByLabel('table-surface')).toBeTruthy()
    for (const id of [0, 1, 2, 3] as const) {
      expect(grid.getChildByLabel(`hand-${id}`)).toBeTruthy()
      expect(grid.getChildByLabel(`discards-${id}`)).toBeTruthy()
    }
  })

  it('lastDiscard=null のとき卓上に last-discard 牌を描画しない', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const state = initWithState({ phase: 'game', lastDiscard: null })

    app.showInitialTable(state)

    const table = getTable(stage)
    expect(table.getChildByLabel('last-discard')).toBeNull()
  })

  it('lastDiscard が指定されているとき卓中央に last-discard を描画する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const state = initWithState({
      phase: 'game',
      lastDiscard: { suit: 'man', value: 7 },
    })

    app.showInitialTable(state)

    const table = getTable(stage)
    expect(table.getChildByLabel('last-discard')).toBeTruthy()
  })

  it.each([
    { currentTurn: 0, wind: '東', markerText: 'あなた (あなた)' },
    { currentTurn: 1, wind: '南', markerText: 'CPU 南' },
    { currentTurn: 2, wind: '西', markerText: 'CPU 西' },
    { currentTurn: 3, wind: '北', markerText: 'CPU 北' },
  ] as const)(
    'currentTurn=%s のときスコア行に .is-turn が 1 つだけ付く',
    ({ currentTurn, markerText }) => {
      const stage = new Container()
      const fakeApp = { stage } as unknown as import('pixi.js').Application
      const app = new App(fakeApp)
      const state = initWithState({ phase: 'game', currentTurn })

      app.showInitialTable(state)

      const turnRows = document.querySelectorAll<HTMLElement>('.score-row.is-turn')
      expect(turnRows.length).toBe(1)
      expect((turnRows[0].querySelector('.name') as HTMLElement).textContent).toContain(
        markerText.replace(/ \(あなた\)$/, '')
      )
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
    // 人間座席が南家 (PlayerIndex 1) なので、卓の自家として hand-1 が描かれる。
    // 旧実装の「bottom-area の中の hand-0」とは違い、新実装は hand-N が
    // table 直下にあり、N は absolute な PlayerIndex を持つ。
    expect(getHand(stage, 1)).toBeTruthy()
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

    // HTML overlay の .score-row に .riichi が 1 つだけ出る
    const riichiBadges = document.querySelectorAll('.score-row .riichi')
    expect(riichiBadges.length).toBe(1)
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

    const getTargetTile = (): Container => getHandTile(stage, '1m-0')

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
    clickActionButton('riichi-discard')

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
    clickActionButton('discard')

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
    clickActionButton('discard')

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
    clickActionButton('riichi-discard')

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

    const hand = getHand(stage, 0)
    const tile = hand.children[0] as Container
    expect(hand.getChildByLabel('1m-0')).toBeNull()
    expect(tile.eventMode).not.toBe('static')
    tile.emit('pointertap', {} as never)

    expect(app.selectedHandIndex).toBe(null)
  })

  it('canTsumo=true のとき「ツモ」ボタンが表示され押下で resolveWinTsumo が呼ばれる (Issue #35)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    // N5: テストでは CPU ターン遅延を明示的に 0 にして flake を避ける
    const app = new App(fakeApp, { cpuTurnDelayMs: 0 })
    let resolveTsumoCalls: number[] = []

    const bridge = createBridgeMock({
      drawTile: () => false,
      canTsumo: () => true,
      resolveWinTsumo: idx => {
        resolveTsumoCalls.push(idx)
        return {
          winner: idx,
          winType: 'tsumo',
          han: 3,
          fu: 30,
          totalPoints: 3900,
          yaku: ['Riichi'],
        }
      },
      // 中間結果シーン表示のため getLastOutcomeJson を埋める
      getLastOutcomeJson: () =>
        JSON.stringify({
          kind: 'win',
          winner: 0,
          winType: 'tsumo',
          han: 3,
          fu: 30,
          totalPoints: 3900,
          yaku: ['Riichi'],
        }),
    })

    app.startGame(bridge, 0)

    const tsumoBtn = findActionButton('tsumo')
    expect(tsumoBtn).toBeTruthy()
    clickActionButton('tsumo')

    expect(resolveTsumoCalls).toEqual([0])
    // 中間結果シーンに遷移している
    expect(app.pendingRoundOutcome).not.toBeNull()
  })

  it('CPU 打牌後に canRon=true なら CPU ループを停止し「ロン」「見逃し」ボタンを出す (Issue #35)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    // N5: cpuTurnDelayMs を 0 にして同期的にループを回す
    const appInst = new App(fakeApp, { cpuTurnDelayMs: 0 })
    let currentPlayerId = 0
    let canRonState = false
    const cpuTurnLog: number[] = []

    const bridge = createBridgeMock({
      isCurrentPlayerHuman: () => currentPlayerId === 0,
      isCurrentPlayerCpu: () => currentPlayerId !== 0,
      getCurrentPlayerId: () => currentPlayerId,
      drawTile: () => false,
      discardTile: () => {
        // 人間打牌成功 → 次プレイヤー (CPU 1) へ
        currentPlayerId = 1
        return true
      },
      executeCpuTurn: () => {
        cpuTurnLog.push(currentPlayerId)
        // 1 巡 CPU が打ったら canRon を true にして次プレイヤーへ
        canRonState = true
        currentPlayerId = (currentPlayerId + 1) % 4
        return '5m'
      },
      canRon: () => canRonState,
      getLastDiscarder: () => 1,
    })

    appInst.startGame(bridge, 0)

    // 人間打牌で CPU ターン開始 → 最初の CPU 打牌後 canRon=true → ループ停止
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    clickActionButton('discard')

    // CPU は 1 回だけ実行されてから停止しているはず
    expect(cpuTurnLog).toEqual([1])
    expect(appInst.pendingRonChance).not.toBeNull()
    expect(appInst.pendingRonChance?.from).toBe(1)

    // 「ロン」「見逃し」ボタン両方表示
    expect(findActionButton('ron')).toBeTruthy()
    expect(findActionButton('ron-skip')).toBeTruthy()
    // 通常の「打牌」ボタンは出ていない
    expect(findActionButton('discard')).toBeNull()
  })

  it('「見逃し」ボタンで pendingRonChance がクリアされて CPU ターンが再開する (Issue #35)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    // N5: cpuTurnDelayMs=0 でループ再開を同期的に確認
    const appInst = new App(fakeApp, { cpuTurnDelayMs: 0 })
    let currentPlayerId = 0
    let canRonState = false
    const cpuTurnLog: number[] = []

    const bridge = createBridgeMock({
      isCurrentPlayerHuman: () => currentPlayerId === 0,
      isCurrentPlayerCpu: () => currentPlayerId !== 0,
      getCurrentPlayerId: () => currentPlayerId,
      drawTile: () => false,
      discardTile: () => {
        currentPlayerId = 1
        return true
      },
      executeCpuTurn: () => {
        cpuTurnLog.push(currentPlayerId)
        if (cpuTurnLog.length === 1) {
          canRonState = true
        } else {
          canRonState = false
        }
        currentPlayerId = (currentPlayerId + 1) % 4
        return '5m'
      },
      canRon: () => canRonState,
      getLastDiscarder: () => 1,
    })

    appInst.startGame(bridge, 0)
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    clickActionButton('discard')

    expect(appInst.pendingRonChance).not.toBeNull()
    clickActionButton('ron-skip')
    expect(appInst.pendingRonChance).toBeNull()
    // CPU ターンが再開して 2 巡目以降が実行された
    expect(cpuTurnLog.length).toBeGreaterThan(1)
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
    expect(findActionButton('discard')).toBeTruthy()
    expect(findActionButton('riichi')).toBeNull()
    expect(findActionButton('riichi-discard')).toBeNull()

    app.startGame(
      createBridgeMock({
        drawTile: () => false,
        canRiichi: () => true,
      }),
      0
    )
    expect(findActionButton('discard')).toBeTruthy()
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    expect(findActionButton('discard')).toBeNull()
    expect(findActionButton('riichi-discard')).toBeTruthy()
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
    clickActionButton('riichi-discard')

    expect(riichiCount).toBe(1)
    expect(app.selectedHandIndex).toBe(0)
    expect(findActionButton('riichi-discard')).toBeTruthy()
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
    clickActionButton('discard')

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
    clickActionButton('discard')

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
    clickActionButton('discard')

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
    clickActionButton('discard')

    expect(app.eventLog.some(entry => entry.includes('あなた が 1m を打牌'))).toBe(true)
    expect(app.eventLog.some(entry => entry.includes('CPU 南 がツモ'))).toBe(true)
    expect(app.eventLog.some(entry => entry.includes('CPU 南 が 5m を打牌'))).toBe(true)
    expect(app.eventLog.some(entry => entry.includes('あなた がツモ'))).toBe(true)
    expect(app.eventLog.some(entry => entry.includes('思考中'))).toBe(false)
  })

  // ==================== Round loop (Issue #27) ====================

  it('山牌切れで対局継続中なら resolveDraw → 中間結果シーン → nextRound で復帰', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    let wall = 0
    let resolveDrawCalled: number[] | null = null
    let nextRoundCalled = false
    let isGameOverVal = false

    const bridge = createBridgeMock({
      drawTile: () => false,
      discardTile: () => true,
      isGameOver: () => isGameOverVal,
      getWallCount: () => wall,
      resolveDraw: (idx: number[]) => {
        resolveDrawCalled = idx
      },
      nextRound: () => {
        nextRoundCalled = true
        wall = 69
        return true
      },
      getRound: () => 2,
      getHonba: () => 0,
      getDealer: () => 1,
      getRiichiSticks: () => 0,
      getLastOutcomeJson: () =>
        JSON.stringify({ kind: 'draw', tenpaiPlayers: [] }),
    } as unknown as Partial<import('./wasm').WasmGameBridge>)

    app.startGame(bridge, 0)
    // 打牌して finalizeGameIfNeeded を起こす
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    clickActionButton('discard')

    // 中間結果シーンに切り替わったこと
    expect((stage.children[0] as Container).label).toBe('round-result-scene')
    expect(resolveDrawCalled).toEqual([])
    expect(app.pendingRoundOutcome?.kind).toBe('draw')

    // 「次局へ」ボタン押下
    const nextBtn = (stage.children[0] as Container).getChildByLabel(
      'round-result-next-button'
    ) as Container
    nextBtn.emit('pointertap', {} as never)
    expect(nextRoundCalled).toBe(true)
    expect(app.pendingRoundOutcome).toBeNull()
    // 卓に復帰している
    expect((stage.children[0] as Container).label).toBe('game-table')

    // sanity: isGameOver を立てれば終局画面に進む
    isGameOverVal = true
  })

  it('山牌切れ時は computeTenpaiPlayers の結果を resolveDraw に渡す (M1 ノーテン罰符防止)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    let wall = 0
    let resolveDrawArg: number[] | null = null
    let computeTenpaiCalled = 0

    const bridge = createBridgeMock({
      drawTile: () => false,
      discardTile: () => true,
      isGameOver: () => false,
      getWallCount: () => wall,
      computeTenpaiPlayers: () => {
        computeTenpaiCalled += 1
        return [0, 2]
      },
      resolveDraw: (idx: number[]) => {
        resolveDrawArg = idx
      },
      nextRound: () => {
        wall = 69
        return true
      },
      getRound: () => 2,
      getHonba: () => 0,
      getDealer: () => 1,
      getRiichiSticks: () => 0,
      getLastOutcomeJson: () =>
        JSON.stringify({ kind: 'draw', tenpaiPlayers: [0, 2] }),
    } as unknown as Partial<import('./wasm').WasmGameBridge>)

    app.startGame(bridge, 0)
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    clickActionButton('discard')

    expect(computeTenpaiCalled).toBeGreaterThanOrEqual(1)
    expect(resolveDrawArg).toEqual([0, 2])
    expect((stage.children[0] as Container).label).toBe('round-result-scene')
  })

  it('nextRound が false を返したら result-scene に進む', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    let wall = 0
    let isGameOverVal = false

    const bridge = createBridgeMock({
      drawTile: () => false,
      discardTile: () => true,
      isGameOver: () => isGameOverVal,
      getWallCount: () => wall,
      resolveDraw: () => undefined,
      nextRound: () => {
        // 終局相当
        isGameOverVal = true
        return false
      },
      getRound: () => 4,
      getHonba: () => 0,
      getDealer: () => 0,
      getRiichiSticks: () => 0,
      getLastOutcomeJson: () =>
        JSON.stringify({ kind: 'draw', tenpaiPlayers: [] }),
    } as unknown as Partial<import('./wasm').WasmGameBridge>)

    app.startGame(bridge, 0)
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    clickActionButton('discard')
    expect((stage.children[0] as Container).label).toBe('round-result-scene')

    const nextBtn = (stage.children[0] as Container).getChildByLabel(
      'round-result-next-button'
    ) as Container
    nextBtn.emit('pointertap', {} as never)
    expect((stage.children[0] as Container).label).toBe('result-scene')
    expect(app.resultMessage).toBeTruthy()
  })

  it('eventLog は EVENT_LOG_LIMIT 件を上限に古い順から切り詰める', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const appendLog = (app as unknown as { appendLog: (message: string) => void }).appendLog

    const overflowCount = EVENT_LOG_LIMIT + 2
    for (let index = 1; index <= overflowCount; index += 1) {
      appendLog.call(app, `log-${index}`)
    }

    expect(app.eventLog).toHaveLength(EVENT_LOG_LIMIT)
    expect(app.eventLog[0]).toBe(`log-${overflowCount - EVENT_LOG_LIMIT + 1}`)
    expect(app.eventLog[app.eventLog.length - 1]).toBe(`log-${overflowCount}`)
  })
})
