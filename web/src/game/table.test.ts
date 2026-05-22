// 卓シーンの最小スモークテスト (Issue #83 副露表示)
//
// jsdom 環境では Pixi の Canvas 経路が動かないため、`createTableScene` で
// 描画自体は走らせず「meld 入りの GameState を渡しても例外を投げない + Container
// ツリーに副露ブロックが追加される」ことだけ確認する。
//
// 詳細な視覚検証は別エージェントが後段で組む観点表テストに任せる。

import { describe, it, expect } from 'vitest'
import type { Container } from 'pixi.js'
import { createMeldGroup, createTableScene } from './table'
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

// createMeldGroup を直接呼んで描画構造を検証する (#83 レビュー指摘 #6)
describe('createMeldGroup direct', () => {
  // 共通定数: 描画 scale = 1、TILE.width = 40 / TILE.height = 56。
  const SCALE = 1

  it('minkan (上家から、fromOffset=3) は sideways スプライトが左端 x ≈ 0 に並ぶ', () => {
    const minkan: MeldGroup = {
      kind: 'minkan',
      tiles: [
        { suit: 'sou', value: 7 },
        { suit: 'sou', value: 7 },
        { suit: 'sou', value: 7 },
        { suit: 'sou', value: 7 },
      ],
      fromOffset: 3,
      claimedIndex: 0,
    }
    const group = createMeldGroup(minkan, SCALE)
    // children は 4 枚並び。rotation === Math.PI/2 の sprite が 1 枚あり、それが先頭スロット (左端)。
    const sideways = (group.children as Container[]).filter(
      c => Math.abs(c.rotation - Math.PI / 2) < 1e-6
    )
    expect(sideways.length).toBe(1)
    // sideways 牌の中心 x = cursorX + tileH/2 = 0 + 56/2 = 28。これは左端スロットを意味する。
    expect(sideways[0]!.x).toBeCloseTo(56 / 2, 5)
    // 4 枚並び (= minkan): 子要素の総数 = 4
    expect(group.children.length).toBe(4)
  })

  it('ankan は中 2 枚 (i=1, i=2) が裏向き sprite (label="back") になる', () => {
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
    const group = createMeldGroup(ankan, SCALE)
    const children = group.children as Container[]
    expect(children.length).toBe(4)
    // 端 (i=0, 3) は表向き = label が CUI コード ("hk" = 白)。
    expect(children[0]!.label).not.toBe('back')
    expect(children[3]!.label).not.toBe('back')
    // 中 (i=1, 2) は裏向き = createTileBackGraphics の出力 (label === 'back')。
    expect(children[1]!.label).toBe('back')
    expect(children[2]!.label).toBe('back')
  })

  it('kakan は stack 牌が claimed の上 (y ≈ -tileW/2 - tileW) に置かれる', () => {
    const kakan: MeldGroup = {
      kind: 'kakan',
      tiles: [
        { suit: 'pin', value: 5 },
        { suit: 'pin', value: 5 },
        { suit: 'pin', value: 5 },
        { suit: 'pin', value: 5 },
      ],
      fromOffset: 1, // 下家から → sideways は右端 (= 3 スロット中の pos=2)
      claimedIndex: 0,
    }
    const group = createMeldGroup(kakan, SCALE)
    // sideways (= rotation Math.PI/2) は 2 枚出るはず: 1 枚は base 並びの sideways、もう 1 枚は stack。
    const rotated = (group.children as Container[]).filter(
      c => Math.abs(c.rotation - Math.PI / 2) < 1e-6
    )
    expect(rotated.length).toBe(2)
    // stack 牌の y = -tileW/2 - tileW = -40/2 - 40 = -60。
    // (もう 1 枚の sideways は base 並びにあって y = -tileW/2 = -20)
    const ys = rotated.map(c => c.y).sort((a, b) => a - b)
    expect(ys[0]).toBeCloseTo(-40 / 2 - 40, 5)
    expect(ys[1]).toBeCloseTo(-40 / 2, 5)
  })
})
