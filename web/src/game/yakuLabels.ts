// 役 (Yaku) の Rust Debug 表記 → 日本語表示用ラベル変換 (Issue #27 follow-up)
//
// Rust 側 `scoring.rs` の `Yaku` enum は `format!("{:?}", Yaku)` の Debug 表記で
// JSON に流れる (`Riichi`, `Tanyao`, `Yakuhai(Ton)` など)。UI 側で
// そのまま表示すると読みづらいので、日本語ラベルに変換する。
//
// 役の網羅は `src/scoring.rs` の `Yaku` enum 全バリアントに対応。
// バリアントが追加されたらここに追記すること。マップにない key は
// console.warn しつつ key をそのまま返す (passthrough)。

/** Rust 側 `Yaku` Debug 表記 → 日本語ラベル。 */
export const YAKU_LABELS: Record<string, string> = {
  // 一飜役
  Riichi: '立直',
  Ippatsu: '一発',
  Tsumo: '門前清自摸和',
  Tanyao: '断幺九',
  Pinfu: '平和',
  Iipeikou: '一盃口',
  Haitei: '海底撈月',
  Houtei: '河底撈魚',
  Rinshan: '嶺上開花',
  Chankan: '槍槓',
  DoubleRiichi: 'ダブル立直',

  // 二飜役
  Chanta: '混全帯幺九',
  SanshokuDoujun: '三色同順',
  Ittsu: '一気通貫',
  Toitoi: '対々和',
  Sanankou: '三暗刻',
  SanshokuDoukou: '三色同刻',
  Sankantsu: '三槓子',
  Chiitoitsu: '七対子',
  Shousangen: '小三元',
  Honroutou: '混老頭',

  // 三飜役
  Honitsu: '混一色',
  Junchan: '純全帯幺九',
  Ryanpeikou: '二盃口',

  // 六飜役
  Chinitsu: '清一色',

  // 役満
  Kokushi: '国士無双',
  Suuankou: '四暗刻',
  Daisangen: '大三元',
  Tsuuiisou: '字一色',
  Shousuushii: '小四喜',
  Daisuushii: '大四喜',
  Ryuuiisou: '緑一色',
  Chinroutou: '清老頭',
  Chuuren: '九蓮宝燈',
  Suukantsu: '四槓子',
  Tenhou: '天和',
  Chiihou: '地和',
}

/**
 * 役牌 (`Yakuhai(Ton)` のような括弧付き Debug 表記) を分解して翻訳する。
 * 括弧内は風牌名 (Ton/Nan/Shaa/Pei) と三元牌 (Haku/Hatsu/Chun)。
 */
const YAKUHAI_INNER_LABELS: Record<string, string> = {
  Ton: '東',
  Nan: '南',
  Shaa: '西',
  Pei: '北',
  Haku: '白',
  Hatsu: '發',
  Chun: '中',
}

/**
 * Rust 側の Yaku Debug 表記を日本語ラベルに変換する。
 * マップにない key は console.warn して key をそのまま返す。
 */
export const yakuLabel = (key: string): string => {
  // 役牌のように括弧付き ("Yakuhai(Ton)") の場合は分解する
  const yakuhaiMatch = /^Yakuhai\((.+)\)$/.exec(key)
  if (yakuhaiMatch) {
    const inner = yakuhaiMatch[1]
    const innerLabel = YAKUHAI_INNER_LABELS[inner] ?? inner
    return `役牌(${innerLabel})`
  }
  const label = YAKU_LABELS[key]
  if (label !== undefined) return label
  // eslint-disable-next-line no-console
  console.warn(`yakuLabel: 未登録の役キー: ${key}`)
  return key
}
