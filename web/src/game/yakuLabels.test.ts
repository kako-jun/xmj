import { describe, it, expect, vi } from 'vitest'
import { yakuLabel } from './yakuLabels'

describe('yakuLabel', () => {
  it('既知のキー (Riichi/Tanyao/Toitoi) は日本語ラベルに変換される', () => {
    expect(yakuLabel('Riichi')).toBe('立直')
    expect(yakuLabel('Tanyao')).toBe('断幺九')
    expect(yakuLabel('Toitoi')).toBe('対々和')
  })

  it('役満 (Suuankou/Daisangen/Kokushi) も網羅されている', () => {
    expect(yakuLabel('Suuankou')).toBe('四暗刻')
    expect(yakuLabel('Daisangen')).toBe('大三元')
    expect(yakuLabel('Kokushi')).toBe('国士無双')
  })

  it('役牌 (Yakuhai(Ton)) は括弧内を分解して翻訳する', () => {
    expect(yakuLabel('Yakuhai(Ton)')).toBe('役牌(東)')
    expect(yakuLabel('Yakuhai(Haku)')).toBe('役牌(白)')
  })

  it('未登録のキーは warn しつつそのまま返す (passthrough)', () => {
    const spy = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    try {
      expect(yakuLabel('UnknownYaku')).toBe('UnknownYaku')
      expect(spy).toHaveBeenCalled()
    } finally {
      spy.mockRestore()
    }
  })
})
