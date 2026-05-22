import { describe, it, expect } from 'vitest'
import {
  pickReadableFgColor,
  SEAT_COLORS,
  seatColorForPlayerId,
  seatWindForPlayerId,
} from './seatColors'

describe('pickReadableFgColor (汎用ユーティリティ)', () => {
  // 注: SEAT_COLORS の fg はハードコードしているため、この関数は外部 (custom) 色用。
  // 黒/白の自動選択がそれっぽく動くことだけ確認する。
  it('明るい色 (黄 #e8c94a) は黒文字', () => {
    expect(pickReadableFgColor('#e8c94a')).toBe('#1a1a1a')
  })
  it('暗い色 (濃紺 #1a1a3a) は白文字', () => {
    expect(pickReadableFgColor('#1a1a3a')).toBe('#ffffff')
  })
  it('真っ白は黒文字 / 真っ黒は白文字', () => {
    expect(pickReadableFgColor('#ffffff')).toBe('#1a1a1a')
    expect(pickReadableFgColor('#000000')).toBe('#ffffff')
  })
  it('不正な hex は黒文字 fallback', () => {
    expect(pickReadableFgColor('not-hex')).toBe('#1a1a1a')
    expect(pickReadableFgColor('#abc')).toBe('#1a1a1a')
  })
})

describe('SEAT_COLORS', () => {
  it('4 風すべて bg / fg / bgNumber を持つ', () => {
    for (const wind of ['east', 'south', 'west', 'north'] as const) {
      const c = SEAT_COLORS[wind]
      expect(c.bg).toMatch(/^#[0-9a-f]{6}$/)
      expect(c.fg).toMatch(/^#[0-9a-f]{6}$/)
      expect(typeof c.bgNumber).toBe('number')
      expect(c.bgNumber).toBeGreaterThanOrEqual(0)
      expect(c.bgNumber).toBeLessThanOrEqual(0xffffff)
    }
  })
  it('期待値 (CSS とハードコード一致しているか)', () => {
    expect(SEAT_COLORS.east.bg).toBe('#d65a4a')
    expect(SEAT_COLORS.east.fg).toBe('#1a1a1a')
    expect(SEAT_COLORS.south.bg).toBe('#e8c94a')
    expect(SEAT_COLORS.south.fg).toBe('#1a1a1a')
    expect(SEAT_COLORS.west.bg).toBe('#3a7ab8')
    expect(SEAT_COLORS.west.fg).toBe('#ffffff')
    expect(SEAT_COLORS.north.bg).toBe('#3f9a5b')
    expect(SEAT_COLORS.north.fg).toBe('#ffffff')
  })
})

describe('seatColorForPlayerId / seatWindForPlayerId', () => {
  it('player.id 0..3 が 東/南/西/北 にマップ (#95 風固定)', () => {
    expect(seatWindForPlayerId(0)).toBe('east')
    expect(seatWindForPlayerId(1)).toBe('south')
    expect(seatWindForPlayerId(2)).toBe('west')
    expect(seatWindForPlayerId(3)).toBe('north')
    expect(seatColorForPlayerId(0).bg).toBe('#d65a4a')
    expect(seatColorForPlayerId(1).bg).toBe('#e8c94a')
    expect(seatColorForPlayerId(2).bg).toBe('#3a7ab8')
    expect(seatColorForPlayerId(3).bg).toBe('#3f9a5b')
  })
})
