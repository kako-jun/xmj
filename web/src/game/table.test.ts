// 卓シーンの最小スモークテスト (Issue #83 副露表示)
//
// jsdom 環境では Pixi の Canvas 経路が動かないため、`createTableScene` で
// 描画自体は走らせず「meld 入りの GameState を渡しても例外を投げない + Container
// ツリーに副露ブロックが追加される」ことだけ確認する。
//
// 詳細な視覚検証は別エージェントが後段で組む観点表テストに任せる。

import { describe, it, expect } from 'vitest'
import { createTableScene } from './table'
import { createInitialGameState } from './state'
import type { GameState, MeldGroup } from './types'

const stateWithPonMeld = (): GameState => {
  const base = createInitialGameState()
  const meld: MeldGroup = {
    kind: 'pon',
    tiles: [
      { suit: 'man', value: 5 },
      { suit: 'man', value: 5 },
      { suit: 'man', value: 5 },
    ],
    fromOffset: 1,
    claimedIndex: 0,
  }
  base.players[0] = {
    ...base.players[0],
    hand: [
      { suit: 'pin', value: 1 },
      { suit: 'pin', value: 2 },
      { suit: 'pin', value: 3 },
      { suit: 'sou', value: 7 },
      { suit: 'sou', value: 8 },
      { suit: 'sou', value: 9 },
      { suit: 'wind', value: 1 },
      { suit: 'wind', value: 1 },
    ],
    melds: [meld],
  }
  base.phase = 'game'
  return base
}

describe('createTableScene with meld', () => {
  it('副露 (ポン) 付きの GameState を渡すと meld-row Container が追加される', () => {
    const scene = createTableScene(stateWithPonMeld(), { humanPlayerIndex: 0 })
    // root の children 内に label='meld-row' を持つ Container があれば pass。
    // 4 プレイヤー分追加されるので最低 1 件以上 (player 0 は pon、他 3 人は空 meld-row)。
    const meldRows = scene.children.filter(c => c.label === 'meld-row')
    expect(meldRows.length).toBeGreaterThanOrEqual(1)
    // pon meld を持つプレイヤーの meld-row には子要素がある (空 row は 0 件)。
    const nonEmptyMeldRow = meldRows.find(c => c.children.length > 0)
    expect(nonEmptyMeldRow).toBeDefined()
    expect(nonEmptyMeldRow?.children.length).toBeGreaterThanOrEqual(1)
  })

  it('暗槓 (ankan) 入りでも例外を投げない (中 2 枚が裏向き)', () => {
    const base = createInitialGameState()
    const ankan: MeldGroup = {
      kind: 'ankan',
      tiles: [
        { suit: 'dragon', value: 1 },
        { suit: 'dragon', value: 1 },
        { suit: 'dragon', value: 1 },
        { suit: 'dragon', value: 1 },
      ],
      fromOffset: null,
      claimedIndex: null,
    }
    base.players[0] = { ...base.players[0], melds: [ankan] }
    base.phase = 'game'
    expect(() => createTableScene(base, { humanPlayerIndex: 0 })).not.toThrow()
  })

  it('加槓 (kakan) は 4 枚分の tiles を持っていても描画できる', () => {
    const base = createInitialGameState()
    const kakan: MeldGroup = {
      kind: 'kakan',
      tiles: [
        { suit: 'pin', value: 2 },
        { suit: 'pin', value: 2 },
        { suit: 'pin', value: 2 },
        { suit: 'pin', value: 2 },
      ],
      fromOffset: 2,
      claimedIndex: 0,
    }
    base.players[0] = { ...base.players[0], melds: [kakan] }
    base.phase = 'game'
    expect(() => createTableScene(base, { humanPlayerIndex: 0 })).not.toThrow()
  })
})
