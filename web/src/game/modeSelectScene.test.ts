import { describe, it, expect, vi } from 'vitest'
import { Container, Text } from 'pixi.js'
import { createModeSelectScene } from './modeSelectScene'
import type { GameMode } from './types'

const buildModes = () => [
  { key: 'tonpuusen' as GameMode, title: '東風戦', description: '東場のみ', enabled: true },
  { key: 'hanchan' as GameMode, title: '半荘戦', description: '東南両場', enabled: false },
]

describe('createModeSelectScene', () => {
  it('mode-select-scene ラベルと各カード・ボタンを持つ', () => {
    const scene = createModeSelectScene({
      selectedMode: 'tonpuusen',
      modes: buildModes(),
      onSelectMode: () => undefined,
      onConfirm: () => undefined,
      onBack: () => undefined,
    })

    expect(scene.label).toBe('mode-select-scene')
    expect(scene.getChildByLabel('mode-card-tonpuusen', true)).toBeTruthy()
    expect(scene.getChildByLabel('mode-card-hanchan', true)).toBeTruthy()
    expect(scene.getChildByLabel('mode-select-confirm')).toBeTruthy()
    expect(scene.getChildByLabel('mode-select-back')).toBeTruthy()
  })

  it('disabled モードのカードはタップを受け付けない', () => {
    const scene = createModeSelectScene({
      selectedMode: 'tonpuusen',
      modes: buildModes(),
      onSelectMode: () => undefined,
      onConfirm: () => undefined,
      onBack: () => undefined,
    })

    const hanchan = scene.getChildByLabel('mode-card-hanchan', true) as Container
    expect(hanchan.eventMode).not.toBe('static')

    const tonpuusen = scene.getChildByLabel('mode-card-tonpuusen', true) as Container
    expect(tonpuusen.eventMode).toBe('static')
  })

  it('選択中モードが disabled なら confirm ボタンは無効', () => {
    const onConfirm = vi.fn()
    const scene = createModeSelectScene({
      selectedMode: 'hanchan',
      modes: buildModes(),
      onSelectMode: () => undefined,
      onConfirm,
      onBack: () => undefined,
    })

    const confirm = scene.getChildByLabel('mode-select-confirm') as Container
    expect(confirm.eventMode).not.toBe('static')
    confirm.emit('pointertap', {} as never)
    expect(onConfirm).not.toHaveBeenCalled()
  })

  it('「選択中」テキストが選択モードのカードに表示される', () => {
    const scene = createModeSelectScene({
      selectedMode: 'tonpuusen',
      modes: buildModes(),
      onSelectMode: () => undefined,
      onConfirm: () => undefined,
      onBack: () => undefined,
    })

    const tonpuusen = scene.getChildByLabel('mode-card-tonpuusen', true) as Container
    const texts = tonpuusen.children.filter((c): c is Text => c instanceof Text)
    expect(texts.some(t => t.text === '選択中')).toBe(true)
  })

  it('onSelectMode / onConfirm / onBack コールバックが各操作で発火する', () => {
    const onSelectMode = vi.fn()
    const onConfirm = vi.fn()
    const onBack = vi.fn()
    const scene = createModeSelectScene({
      selectedMode: 'tonpuusen',
      modes: buildModes(),
      onSelectMode,
      onConfirm,
      onBack,
    })

    ;(scene.getChildByLabel('mode-card-tonpuusen', true) as Container).emit(
      'pointertap',
      {} as never
    )
    expect(onSelectMode).toHaveBeenCalledWith('tonpuusen')

    ;(scene.getChildByLabel('mode-select-confirm') as Container).emit('pointertap', {} as never)
    expect(onConfirm).toHaveBeenCalledTimes(1)

    ;(scene.getChildByLabel('mode-select-back') as Container).emit('pointertap', {} as never)
    expect(onBack).toHaveBeenCalledTimes(1)
  })
})
