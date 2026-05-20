import { describe, it, expect, vi } from 'vitest'
import { Container, Text } from 'pixi.js'
import { createDiceRollScene } from './diceRollScene'

describe('createDiceRollScene', () => {
  it('roll=null のとき "サイコロを振っています…" と "?" を表示する', () => {
    const scene = createDiceRollScene({
      roll: null,
      humanSeat: null,
      onComplete: () => undefined,
    })

    expect(scene.label).toBe('dice-roll-scene')
    expect(scene.getChildByLabel('dice-rolling-text', true)).toBeTruthy()
    expect(scene.getChildByLabel('dice-roll-start-button')).toBeNull()
    // ダイス自体は2つ存在する
    expect(scene.getChildByLabel('die-1', true)).toBeTruthy()
    expect(scene.getChildByLabel('die-2', true)).toBeTruthy()
  })

  it('roll が確定すると seat 結果と開始ボタンが出る', () => {
    const scene = createDiceRollScene({
      roll: { d1: 2, d2: 1 },
      humanSeat: 1,
      onComplete: () => undefined,
    })

    expect(scene.getChildByLabel('dice-seat-result', true)).toBeTruthy()
    const seatText = scene.getChildByLabel('dice-seat-result', true) as Text
    expect(seatText.text).toBe('あなたは南家からスタート')
    expect(scene.getChildByLabel('dice-roll-start-button')).toBeTruthy()
    expect(scene.getChildByLabel('dice-rolling-text', true)).toBeNull()
  })

  it('開始ボタンタップで onComplete が呼ばれる', () => {
    const onComplete = vi.fn()
    const scene = createDiceRollScene({
      roll: { d1: 5, d2: 5 },
      humanSeat: 0,
      onComplete,
    })

    const btn = scene.getChildByLabel('dice-roll-start-button') as Container
    btn.emit('pointertap', {} as never)
    expect(onComplete).toHaveBeenCalledTimes(1)
  })

  it.each([
    [{ d1: 1, d2: 1 } as const, 0, '東家'],
    [{ d1: 2, d2: 1 } as const, 1, '南家'],
    [{ d1: 2, d2: 2 } as const, 2, '西家'],
    [{ d1: 3, d2: 2 } as const, 3, '北家'],
  ])('roll=%j の表示が seat に対応する', (roll, humanSeat, seatName) => {
    const scene = createDiceRollScene({
      roll,
      humanSeat: humanSeat as 0 | 1 | 2 | 3,
      onComplete: () => undefined,
    })
    const seatText = scene.getChildByLabel('dice-seat-result', true) as Text
    expect(seatText.text).toBe(`あなたは${seatName}からスタート`)
  })
})
