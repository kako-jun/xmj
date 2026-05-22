import { describe, it, expect, vi } from 'vitest'
import { Container, Text } from 'pixi.js'
import { createModeSelectScene } from './modeSelectScene'
import type { GameMode } from './types'

const buildModes = () => [
  { key: 'tonpuusen' as GameMode, title: '東風戦', description: '東場のみ', enabled: true },
  { key: 'hanchan' as GameMode, title: '半荘戦', description: '東南両場', enabled: true },
]

describe('createModeSelectScene', () => {
  it('mode-select-scene ラベルと各カード・戻るボタンを持つ', () => {
    const scene = createModeSelectScene({
      selectedMode: 'tonpuusen',
      modes: buildModes(),
      onSelectMode: () => undefined,
      onBack: () => undefined,
    })

    expect(scene.label).toBe('mode-select-scene')
    expect(scene.getChildByLabel('mode-card-tonpuusen', true)).toBeTruthy()
    expect(scene.getChildByLabel('mode-card-hanchan', true)).toBeTruthy()
    expect(scene.getChildByLabel('mode-select-back')).toBeTruthy()
    // 「次へ」ボタンは廃止
    expect(scene.getChildByLabel('mode-select-confirm')).toBeFalsy()
  })

  it('disabled モードのカードはタップを受け付けない', () => {
    const modes = [
      { key: 'tonpuusen' as GameMode, title: '東風戦', description: '', enabled: true },
      { key: 'hanchan' as GameMode, title: '半荘戦', description: '', enabled: false },
    ]
    const scene = createModeSelectScene({
      selectedMode: 'tonpuusen',
      modes,
      onSelectMode: () => undefined,
      onBack: () => undefined,
    })

    const hanchan = scene.getChildByLabel('mode-card-hanchan', true) as Container
    expect(hanchan.eventMode).not.toBe('static')

    const tonpuusen = scene.getChildByLabel('mode-card-tonpuusen', true) as Container
    expect(tonpuusen.eventMode).toBe('static')
  })

  it('「選択中」テキストが選択モードのカードに表示される', () => {
    const scene = createModeSelectScene({
      selectedMode: 'tonpuusen',
      modes: buildModes(),
      onSelectMode: () => undefined,
      onBack: () => undefined,
    })

    const tonpuusen = scene.getChildByLabel('mode-card-tonpuusen', true) as Container
    const texts = tonpuusen.children.filter((c): c is Text => c instanceof Text)
    expect(texts.some(t => t.text === '選択中')).toBe(true)
  })

  it('カードタップで onSelectMode が即発火し、戻るで onBack', () => {
    const onSelectMode = vi.fn()
    const onBack = vi.fn()
    const scene = createModeSelectScene({
      selectedMode: 'tonpuusen',
      modes: buildModes(),
      onSelectMode,
      onBack,
    })

    ;(scene.getChildByLabel('mode-card-hanchan', true) as Container).emit(
      'pointertap',
      {} as never
    )
    expect(onSelectMode).toHaveBeenCalledWith('hanchan')

    ;(scene.getChildByLabel('mode-select-back') as Container).emit('pointertap', {} as never)
    expect(onBack).toHaveBeenCalledTimes(1)
  })
})
