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
import { tileToCuiCode } from './types'
import { EVENT_LOG_LIMIT } from './constants'

const tileToCuiCodeForTest = (tile: Tile): string => tileToCuiCode(tile)

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
    canPon: () => false,
    canKan: () => false,
    canChi: () => false,
    doPon: () => false,
    doKan: () => false,
    doChi: () => false,
    // Issue #46: 暗槓 / 加槓 API。デフォルトは候補なし。
    canAnkan: () => [] as Tile[],
    canShouminkan: () => [] as Tile[],
    doAnkan: () => false,
    startShouminkan: () => ({ ok: false, candidates: [] }),
    completeShouminkan: () => false,
    cancelShouminkan: () => undefined,
    resolveWinChankan: () => null,
    skipRon: () => undefined,
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


const getSceneButton = (stage: Container, label: string): Container => {
  // 新仕様: 卓 + dice overlay のように複数 scene が stage 直下に並ぶことがあるので
  // すべての children を走査して該当ラベルの最初の Container を返す。
  for (const child of stage.children) {
    const found = (child as Container).getChildByLabel?.(label)
    if (found) return found as Container
  }
  return null as unknown as Container
}

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
  // mode-select-confirm 廃止: カードタップで即進行する形に変更したのでこのテスト経路は dice 直接呼び出しに統合
  // (本来はカードタップだが、テストは showDiceRollScene を直接呼ぶ)
  app.showDiceRollScene(roll)
  // dice overlay は startNewGame 成功時のみ出る。bridge factory が例外を投げる
  // テストケース (title-scene へ戻る) では overlay が無いことが期待値。
  // ボタンが見つかれば tap、無ければそのフロー (失敗フロー) として呼び出し側が assert する。
  const startBtn = getSceneButton(stage, 'dice-roll-start-button')
  if (startBtn) startBtn.emit('pointertap', {} as never)
  // 呼び出し側で「started フラグ」を見たい場合は app.bridge が non-null かを確認すること。
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

  it('卓中央には lastDiscard を描画しない (鳴き対象は河で強調する方針)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)
    const state = initWithState({
      phase: 'game',
      lastDiscard: { suit: 'man', value: 7 },
    })

    app.showInitialTable(state)

    const table = getTable(stage)
    expect(table.getChildByLabel('meld-target-tile')).toBeNull()
    expect(table.getChildByLabel('last-discard')).toBeNull()
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

    // モード選択 → 場決め (新仕様: dice は対局画面の上に overlay 表示)
    // テスト用に明示的な dice を注入する
    app.showDiceRollScene({ d1: 2, d2: 1 })
    // 卓が先に描画され、その上に dice overlay が乗る (children[0]=game-table, children[last]=dice-roll-scene)
    expect((stage.children[0] as Container).label).toBe('game-table')
    expect((stage.children[stage.children.length - 1] as Container).label).toBe('dice-roll-scene')
    expect(app.selectedHumanSeat).toBe(1)

    // dice overlay の「対局を始める」で overlay が消える (対局はすでに始まっている)
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
    // 半荘戦カードでも tonpuusen でもどちらでも良いが、カードタップで即 dice へ進む
    getModeCard(stage, 'tonpuusen').emit('pointertap', {} as never)

    expect(rollDice).toHaveBeenCalledTimes(1)
    expect(app.selectedHumanSeat).toBe(0)
    // 卓の上に dice overlay
    expect((stage.children[stage.children.length - 1] as Container).label).toBe('dice-roll-scene')
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

  it('東風戦・半荘戦どちらのカードもタップ可能 (両モードとも enabled)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp, { createBridge: () => createBridgeMock() })

    app.showTitleScene()
    getSceneButton(stage, 'title-start-button').emit('pointertap', {} as never)
    expect(getModeCard(stage, 'tonpuusen').eventMode).toBe('static')
    expect(getModeCard(stage, 'hanchan').eventMode).toBe('static')
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

    // 新しい UX: ツモ後 canRiichi=true → リーチ確認モーダルが出る
    expect(app.pendingDecision?.kind).toBe('riichi-prompt')
    clickActionButton('riichi') // モーダルで「リーチ」を選択
    expect(app.riichiArmed).toBe(true)
    expect(app.pendingDecision).toBeNull()

    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    clickActionButton('riichi-discard')

    expect(declareRiichiCount).toBe(1)
    expect(discardCount).toBe(1)
    expect(cpuCount).toBe(3)
    expect(drawCount).toBe(1)
    // 新仕様: ツモ後はデフォルトで「ツモ牌」が選択された状態になるため selectedHandIndex は非 null
    expect(app.selectedHandIndex).not.toBe(null)
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
    // mode-select-confirm 廃止: カードタップで即進行する形に変更したのでこのテスト経路は dice 直接呼び出しに統合
  // (本来はカードタップだが、テストは showDiceRollScene を直接呼ぶ)
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
    expect(app.pendingDecision?.kind).toBe('riichi-prompt')
    clickActionButton('riichi')

    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    clickActionButton('riichi-discard')

    expect(riichiCount).toBe(1)
    expect(cpuCount).toBe(0)
    expect(app.selectedHandIndex).toBe(0)
    expect(app.gameState?.currentTurn).toBe(0)
    expect(app.gameState?.players[0].hand).toHaveLength(14)
    expect(app.gameState?.players[0].discards).toHaveLength(0)
    // 打牌失敗時は riichiArmed のままで再試行できる
    expect(app.riichiArmed).toBe(true)
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
    expect(appInst.pendingDecision).not.toBeNull()
    expect(appInst.pendingDecision?.kind).toBe('meld-call')
    if (appInst.pendingDecision?.kind === 'meld-call') {
      expect(appInst.pendingDecision.from).toBe(1)
      expect(appInst.pendingDecision.canRon).toBe(true)
    }

    // 「ロン」「見逃し」ボタン両方表示
    expect(findActionButton('ron')).toBeTruthy()
    expect(findActionButton('meld-skip')).toBeTruthy()
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

    expect(appInst.pendingDecision).not.toBeNull()
    clickActionButton('meld-skip')
    expect(appInst.pendingDecision).toBeNull()
    // CPU ターンが再開して 2 巡目以降が実行された
    expect(cpuTurnLog.length).toBeGreaterThan(1)
  })

  it('明槓宣言後、嶺上ツモ牌が justDrawnTile に反映される (Issue #48)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    // cpuTurnDelayMs=0 で CPU 打牌を同期的に進める
    const appInst = new App(fakeApp, { cpuTurnDelayMs: 0 })

    // CPU 1 が 5m を打牌した瞬間: 人間 (player 0) は 5m を 3 枚持っていて canKan=true。
    // do_kan 後は 5m が 3 枚副露へ移り、嶺上から 中 (cn = dragon 3) を引いた状態にする。
    const handBefore = '1m 5m 5m 5m 7p 8p 9p 2s 3s 4s 5s 6s to'
    const handAfter = '1m 7p 8p 9p 2s 3s 4s 5s 6s to cn'
    const stateBefore = `Round: 1 | Wall: 60 tiles
Dora indicators: 5p
 親 あなた (25000点): ${handBefore}
  河:
> CPU 南 (25000点): 1p 1p 2p 2p 3p 3p 4s 5s 6s na na ht cn
  河: 5m
   CPU 西 (25000点): 4m 5m 6m 7m 8m 9m 3p 4p 5p 6p 7p 8p pe
   CPU 北 (25000点): 1s 1s 2s 2s 3s 3s 4m 4m 5m 5m 6m 6m sa
Last discard: 5m
`
    const stateAfter = `Round: 1 | Wall: 58 tiles
Dora indicators: 5p 1p
>親 あなた (25000点): ${handAfter}
  河:
   CPU 南 (25000点): 1p 1p 2p 2p 3p 3p 4s 5s 6s na na ht cn
  河: 5m
   CPU 西 (25000点): 4m 5m 6m 7m 8m 9m 3p 4p 5p 6p 7p 8p pe
   CPU 北 (25000点): 1s 1s 2s 2s 3s 3s 4m 4m 5m 5m 6m 6m sa
`

    let currentPlayerId = 0
    let kanCalled = false
    const cpuTurnLog: number[] = []
    let canKanState = false

    const bridge = createBridgeMock({
      getGameStateJson: () => (kanCalled ? stateAfter : stateBefore),
      getCurrentHandString: () => (kanCalled ? handAfter : handBefore),
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
        // CPU 1 が 5m を打って canKan=true → ループ停止
        canKanState = true
        currentPlayerId = (currentPlayerId + 1) % 4
        return '5m'
      },
      canKan: () => canKanState,
      doKan: () => {
        kanCalled = true
        canKanState = false
        // do_kan 後は手番が宣言者 (人間) に戻る
        currentPlayerId = 0
        return true
      },
      getLastDiscarder: () => 1,
    })

    appInst.startGame(bridge, 0)

    // 人間打牌で CPU ターン開始 → CPU 1 が 5m 打牌 → canKan=true でループ停止
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    clickActionButton('discard')

    expect(appInst.pendingDecision?.kind).toBe('meld-call')
    if (appInst.pendingDecision?.kind === 'meld-call') {
      expect(appInst.pendingDecision.canKan).toBe(true)
    }

    // 「カン」ボタンで明槓を確定
    clickActionButton('kan')

    // do_kan 後、嶺上ツモ牌 (中 = dragon value 3) が justDrawnTile にセットされている
    expect(appInst.pendingDecision).toBeNull()
    expect(appInst.justDrawnTile).not.toBeNull()
    expect(appInst.justDrawnTile).toEqual({ suit: 'dragon', value: 3 })
  })

  it('canRiichi=false のときは「リーチ」モーダルが出ず通常の打牌 UI のまま', () => {
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
    expect(app.pendingDecision).toBeNull()
  })

  it('canRiichi=true ならツモ直後に「リーチ / リーチしない」モーダルが出る', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    app.startGame(
      createBridgeMock({
        drawTile: () => false,
        canRiichi: () => true,
      }),
      0
    )
    expect(app.pendingDecision?.kind).toBe('riichi-prompt')
    expect(findActionButton('riichi')).toBeTruthy()
    expect(findActionButton('riichi-skip')).toBeTruthy()
    // モーダル中は通常の打牌ボタンは出ない
    expect(findActionButton('discard')).toBeNull()
    // 「リーチしない」を押せばモーダルが消えて通常 UI に戻る
    clickActionButton('riichi-skip')
    expect(app.pendingDecision).toBeNull()
    expect(app.riichiDeclinedThisTurn).toBe(true)
    expect(findActionButton('discard')).toBeTruthy()
  })

  it('canAnkan が空でないツモ後、self-kan-prompt モーダルが出て暗槓ボタンを押すと doAnkan が呼ばれる (Issue #46)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    const ankanTile: Tile = { suit: 'man', value: 5 }
    let doAnkanCalled: { idx: number; tile: Tile } | null = null
    let ankanDone = false

    app.startGame(
      createBridgeMock({
        drawTile: () => false,
        canRiichi: () => false,
        canAnkan: () => (ankanDone ? [] : [ankanTile]),
        canShouminkan: () => [],
        doAnkan: (idx, tile) => {
          doAnkanCalled = { idx, tile }
          ankanDone = true
          return true
        },
      }),
      0
    )
    expect(app.pendingDecision?.kind).toBe('self-kan-prompt')
    if (app.pendingDecision?.kind === 'self-kan-prompt') {
      expect(app.pendingDecision.ankan).toEqual([ankanTile])
      expect(app.pendingDecision.shouminkan).toEqual([])
    }
    // 暗槓ボタンが出ている
    expect(findActionButton('self-ankan')).toBeTruthy()
    expect(findActionButton('self-kan-skip')).toBeTruthy()
    // 通常打牌は出ない
    expect(findActionButton('discard')).toBeNull()

    clickActionButton('self-ankan')
    expect(doAnkanCalled).toEqual({ idx: 0, tile: ankanTile })
    expect(app.pendingDecision).toBeNull()
  })

  it('canShouminkan が空でないツモ後、加槓ボタンで startShouminkan + completeShouminkan が呼ばれる (Issue #46)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    const tile: Tile = { suit: 'pin', value: 7 }
    const startCalls: Array<{ idx: number; tile: Tile }> = []
    const completeCalls: Array<{ idx: number; tile: Tile }> = []
    let shouminkanDone = false

    app.startGame(
      createBridgeMock({
        drawTile: () => false,
        canRiichi: () => false,
        canAnkan: () => [],
        canShouminkan: () => (shouminkanDone ? [] : [tile]),
        startShouminkan: (idx, t) => {
          startCalls.push({ idx, tile: t })
          return { ok: true, candidates: [] }
        },
        completeShouminkan: (idx, t) => {
          completeCalls.push({ idx, tile: t })
          shouminkanDone = true
          return true
        },
      }),
      0
    )
    expect(app.pendingDecision?.kind).toBe('self-kan-prompt')
    expect(findActionButton('self-shouminkan')).toBeTruthy()

    clickActionButton('self-shouminkan')
    expect(startCalls).toEqual([{ idx: 0, tile }])
    expect(completeCalls).toEqual([{ idx: 0, tile }])
    expect(app.pendingDecision).toBeNull()
  })

  it('self-kan-prompt で「カンしない」を押すと canRiichi=true なら riichi-prompt に進む (Issue #46)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    const ankanTile: Tile = { suit: 'man', value: 5 }

    app.startGame(
      createBridgeMock({
        drawTile: () => false,
        canRiichi: () => true,
        canAnkan: () => [ankanTile],
        canShouminkan: () => [],
      }),
      0
    )
    expect(app.pendingDecision?.kind).toBe('self-kan-prompt')
    clickActionButton('self-kan-skip')
    // カンを断ったあと、立直可能なら今度は riichi-prompt
    expect(app.pendingDecision?.kind).toBe('riichi-prompt')
  })

  it('canAnkan が空 / canShouminkan も空ならツモ後にカンモーダルは出ない (Issue #46)', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    app.startGame(
      createBridgeMock({
        drawTile: () => false,
        canRiichi: () => false,
        canAnkan: () => [],
        canShouminkan: () => [],
      }),
      0
    )
    expect(app.pendingDecision).toBeNull()
    expect(findActionButton('discard')).toBeTruthy()
  })

  it('リーチを armed 状態でも declareRiichi が false なら状態を維持して再試行できる', () => {
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
    expect(app.pendingDecision?.kind).toBe('riichi-prompt')
    clickActionButton('riichi')
    expect(app.riichiArmed).toBe(true)

    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    clickActionButton('riichi-discard')

    expect(riichiCount).toBe(1)
    expect(app.selectedHandIndex).toBe(0)
    expect(app.riichiArmed).toBe(true)
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

  // ============================================================================
  // Issue #62: armed リーチのキャンセル
  // ============================================================================

  it('Issue #62: armed リーチ中に Esc 相当のキャンセルを呼ぶと armed 状態を解除する', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    const bridge = createBridgeMock({
      drawTile: () => true,
      canRiichi: () => true,
      // declareRiichi が呼ばれてはいけない (まだ「立直して打牌」していない)
      declareRiichi: () => {
        throw new Error('declareRiichi must not be called when disarming')
      },
    })

    app.startGame(bridge, 0)
    expect(app.pendingDecision?.kind).toBe('riichi-prompt')
    clickActionButton('riichi')
    expect(app.riichiArmed).toBe(true)

    // 牌を 1 つ選んだ状態にしてから取り消し
    getHandTile(stage, '1m-0').emit('pointertap', {} as never)
    expect(app.selectedHandIndex).not.toBeNull()

    // Esc 相当: handleHotkeyCancel を直接呼ぶ
    ;(app as unknown as { handleHotkeyCancel: () => void }).handleHotkeyCancel()

    expect(app.riichiArmed).toBe(false)
    expect(app.selectedHandIndex).toBeNull()
    expect(app.eventLog).toContain('リーチをキャンセル')
    // armed が外れたので「立直やめる」ボタンは消えている
    expect(findActionButton('riichi-cancel')).toBeNull()
    // 通常の「打牌」ボタンに戻っている
    expect(findActionButton('discard')).toBeTruthy()
    expect(findActionButton('riichi-discard')).toBeNull()
  })

  it('Issue #62: armed リーチ中は「立直やめる」ボタンが出て、クリックすると armed が解除される', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    const bridge = createBridgeMock({
      drawTile: () => true,
      canRiichi: () => true,
      declareRiichi: () => {
        throw new Error('declareRiichi must not be called when disarming')
      },
    })

    app.startGame(bridge, 0)
    clickActionButton('riichi')
    expect(app.riichiArmed).toBe(true)

    // ボタンが出ていることを確認
    expect(findActionButton('riichi-cancel')).toBeTruthy()

    clickActionButton('riichi-cancel')

    expect(app.riichiArmed).toBe(false)
    expect(app.selectedHandIndex).toBeNull()
    expect(app.eventLog).toContain('リーチをキャンセル')
    expect(findActionButton('riichi-cancel')).toBeNull()
  })

  // ============================================================================
  // Issue #63: 同種牌タップは rightmost (ツモ牌分離スロット) に寄せる
  // ============================================================================

  it('Issue #63: 同種牌が複数あるとき、本体側 index タップでも selectedHandIndex は rightmost に寄る', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    // 1m が 3 枚並ぶ手牌
    const bridge = createBridgeMock({
      drawTile: () => true,
      getCurrentHandString: () => '1m 1m 1m 2p 3p 4p 5p 6p 7p 8p 9p 2s 3s 4s',
    })

    app.startGame(bridge, 0)
    // 念のため hand の構造を確認
    const hand = app.gameState!.players[0].hand
    expect(hand.length).toBeGreaterThanOrEqual(3)
    expect(tileToCuiCodeForTest(hand[0])).toBe('1m')
    expect(tileToCuiCodeForTest(hand[1])).toBe('1m')
    expect(tileToCuiCodeForTest(hand[2])).toBe('1m')

    // 本体側 (index=0) をタップ → rightmost (index=2) が選択される
    ;(app as unknown as { handleHandTileTap: (i: number) => void }).handleHandTileTap(0)
    expect(app.selectedHandIndex).toBe(2)
  })

  it('Issue #63: 異種牌が並んでいるときは coalesce せず、タップした index そのものが選択される', () => {
    const stage = new Container()
    const fakeApp = { stage } as unknown as import('pixi.js').Application
    const app = new App(fakeApp)

    const bridge = createBridgeMock({
      drawTile: () => true,
      getCurrentHandString: () => '1m 2m 3m 4m 5m 6m 7p 8p 9p 2s 3s 4s 5s 6s',
    })

    app.startGame(bridge, 0)
    const hand = app.gameState!.players[0].hand
    expect(tileToCuiCodeForTest(hand[0])).toBe('1m')
    expect(tileToCuiCodeForTest(hand[1])).toBe('2m')

    ;(app as unknown as { handleHandTileTap: (i: number) => void }).handleHandTileTap(0)
    expect(app.selectedHandIndex).toBe(0)
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
