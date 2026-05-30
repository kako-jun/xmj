//! 面子分解ベースの構造役判定と符計算 (#108 監査修正)。
//!
//! `scoring.rs` の旧実装は一盃口 / 二盃口 / 三色同順 / 一気通貫 / チャンタ /
//! 純チャン / 三色同刻 / 小三元 / 三暗刻 をスタブ (常に false/None) で返しており、
//! 符計算も基本符 20 + ツモ 2 のみだった。本モジュールは `agari` の分解エンジン
//! (`enumerate_concealed_decomps`) を使い、雀頭 + 4 面子の構造から各役と符を
//! 正しく算出する。副露牌は固定面子として分解結果に結合する。
//!
//! live path: `scoring::ScoringEngine::calculate_score_with_context` から
//! `evaluate_best` が呼ばれ、複数の和了分解のうち最高得点 (han → fu) の解釈を返す。

use crate::agari::{enumerate_concealed_decomps, MachiKind, Mentsu};
use crate::hand::{Hand, MeldType};
use crate::scoring::{ScoringContext, Yaku};
use crate::tile::{Honor, Suit, Tile, TileType};

/// 符・暗刻判定のための面子 + 由来情報。
#[derive(Debug, Clone, Copy)]
struct SMentsu {
    mentsu: Mentsu,
    /// 暗 (手牌内の暗刻 / 暗槓) なら true。ロンで完成した刻子・副露は false。
    concealed: bool,
    /// 槓子なら true。
    is_kan: bool,
}

/// 構造役評価の結果。
#[derive(Debug, Clone, Default)]
pub struct StructResult {
    pub yaku: Vec<Yaku>,
    /// 構造役のみの飜数 (場役・状況役・ドラは含まない)。
    pub han: u32,
    /// この分解での符 (10 切り上げ済み)。
    pub fu: u32,
    pub is_pinfu: bool,
}

/// 和了形の全分解のうち最高得点 (構造役 han → fu) の解釈を返す。
///
/// 七対子 / 国士 / 役満は呼び出し側 (`scoring.rs`) で別途処理されるため、
/// 本関数は通常形 (4 面子 1 雀頭) のみを対象とする。分解が見つからなければ None。
pub fn evaluate_best(
    hand: &Hand,
    winning_tile: &Tile,
    ctx: &ScoringContext,
    is_menzen: bool,
) -> Option<StructResult> {
    let melds = hand.get_melds();
    let melds_needed = 4usize.checked_sub(melds.len())?;

    // 分解は赤ドラを正規化した牌で行われる (agari 側)。本関数内の刻子・牌比較も
    // 正規化牌に合わせるため、winning_tile を正規化したものを使う。
    let winning_norm = strip_red(winning_tile);
    let winning_tile = &winning_norm;

    // 副露を固定面子に変換。
    let mut fixed: Vec<SMentsu> = Vec::new();
    for m in melds {
        let first = match m.tiles.first() {
            Some(t) => strip_red(t),
            None => continue,
        };
        match m.meld_type {
            MeldType::Chi => {
                // tiles は順不同の可能性があるため最小値を start とする。
                let start = strip_red(&chi_start(&m.tiles).unwrap_or(first));
                fixed.push(SMentsu {
                    mentsu: Mentsu::Shuntsu(start),
                    concealed: false,
                    is_kan: false,
                });
            }
            MeldType::Pon => fixed.push(SMentsu {
                mentsu: Mentsu::Koutsu(first),
                concealed: false,
                is_kan: false,
            }),
            MeldType::Kan => fixed.push(SMentsu {
                mentsu: Mentsu::Koutsu(first),
                // 暗槓 (is_open=false) のみ暗刻扱い。
                concealed: !m.is_open,
                is_kan: true,
            }),
        }
    }

    // 手牌部分 (winning_tile を含む) を分解。
    let mut concealed_tiles = hand.get_tiles().clone();
    concealed_tiles.push(*winning_tile);

    let decomps = enumerate_concealed_decomps(&concealed_tiles, winning_tile, melds_needed);
    let mut best: Option<StructResult> = None;

    for (pair, ml, wait) in decomps {
        // この分解の手牌面子を SMentsu 化。ロンでシャンポン完成した刻子は明刻扱い。
        let mut sm: Vec<SMentsu> = Vec::new();
        let mut ron_koutsu_downgraded = false;
        for m in &ml {
            let mut concealed = true;
            if let Mentsu::Koutsu(t) = m {
                if !ctx.is_tsumo
                    && matches!(wait, MachiKind::Shanpon)
                    && t == winning_tile
                    && !ron_koutsu_downgraded
                {
                    // ロンで完成したシャンポンの当たり刻子 1 つだけを明刻に格下げ。
                    concealed = false;
                    ron_koutsu_downgraded = true;
                }
            }
            sm.push(SMentsu {
                mentsu: *m,
                concealed,
                is_kan: false,
            });
        }
        let mut all = fixed.clone();
        all.extend(sm.iter().copied());

        let mut yaku: Vec<Yaku> = Vec::new();
        let mut han = 0u32;

        // 一盃口 / 二盃口 (門前のみ)
        if is_menzen {
            let peikou = count_iipeikou(&all);
            if peikou >= 2 {
                yaku.push(Yaku::Ryanpeikou);
                han += 3;
            } else if peikou == 1 {
                yaku.push(Yaku::Iipeikou);
                han += 1;
            }
        }

        // 三色同順
        if has_sanshoku_doujun(&all) {
            yaku.push(Yaku::SanshokuDoujun);
            han += if is_menzen { 2 } else { 1 };
        }
        // 一気通貫
        if has_ittsu(&all) {
            yaku.push(Yaku::Ittsu);
            han += if is_menzen { 2 } else { 1 };
        }
        // チャンタ / 純チャン
        match chanta_kind(&all, &pair) {
            ChantaKind::Junchan => {
                yaku.push(Yaku::Junchan);
                han += if is_menzen { 3 } else { 2 };
            }
            ChantaKind::Chanta => {
                yaku.push(Yaku::Chanta);
                han += if is_menzen { 2 } else { 1 };
            }
            ChantaKind::None => {}
        }
        // 三色同刻
        if has_sanshoku_doukou(&all) {
            yaku.push(Yaku::SanshokuDoukou);
            han += 2;
        }
        // 小三元
        if is_shousangen(&all, &pair) {
            yaku.push(Yaku::Shousangen);
            han += 2;
        }
        // 三暗刻 (暗刻 3 つ。4 つは四暗刻として役満処理済みなのでここには来ない想定)
        let ankou = all
            .iter()
            .filter(|s| s.concealed && matches!(s.mentsu, Mentsu::Koutsu(_)))
            .count();
        if ankou >= 3 {
            yaku.push(Yaku::Sanankou);
            han += 2;
        }

        // #58 三連刻 (ローカル, 2 飜): 同一スーツ連続 3 刻子。
        // 四連刻 (役満) は scoring.rs の yakuman ブロックで先に確定するためここには来ない。
        if ctx.allow_local_yakuman {
            let koutsu: Vec<(Suit, u8)> = all
                .iter()
                .filter_map(|s| match s.mentsu {
                    Mentsu::Koutsu(t) => match t.tile_type {
                        TileType::Number { suit, value } => Some((suit, value)),
                        _ => None,
                    },
                    _ => None,
                })
                .collect();
            if longest_consecutive_same_suit(&koutsu) >= 3 {
                yaku.push(Yaku::Sanrenkou);
                han += 2;
            }
        }

        // 平和 (門前のみ)
        let is_pinfu = is_menzen && is_pinfu_decomp(&all, &pair, wait, ctx);
        if is_pinfu {
            yaku.push(Yaku::Pinfu);
            han += 1;
        }

        let fu = calc_fu(&all, &pair, wait, ctx, is_menzen, is_pinfu);

        let cand = StructResult {
            yaku,
            han,
            fu,
            is_pinfu,
        };
        best = match best {
            None => Some(cand),
            Some(prev) => {
                if (cand.han, cand.fu) > (prev.han, prev.fu) {
                    Some(cand)
                } else {
                    Some(prev)
                }
            }
        };
    }

    best
}

/// #58: 同一スーツの連続刻子 (連刻) の最大連続数を返す。
///
/// 副露の刻子 (ポン/カン) + 各分解の手牌側刻子を合わせ、man/pin/sou ごとに
/// 「連続する数値の刻子」の最長ランを求め、全分解の最大値を返す。
/// 三連刻 (n=3) / 四連刻 (n=4) 判定に使う。和了形が無ければ 0。
pub fn max_renkou_run(hand: &Hand, winning_tile: &Tile) -> usize {
    let melds = hand.get_melds();
    let melds_needed = match 4usize.checked_sub(melds.len()) {
        Some(n) => n,
        None => return 0,
    };
    // 副露の刻子値 (suit, value)
    let mut meld_koutsu: Vec<(Suit, u8)> = Vec::new();
    for m in melds {
        if matches!(m.meld_type, MeldType::Pon | MeldType::Kan) {
            if let Some(t) = m.tiles.first() {
                if let TileType::Number { suit, value } = t.tile_type {
                    meld_koutsu.push((suit, value));
                }
            }
        }
    }

    let winning_norm = strip_red(winning_tile);
    let mut concealed = hand.get_tiles().clone();
    concealed.push(winning_norm);
    let decomps = enumerate_concealed_decomps(&concealed, &winning_norm, melds_needed);

    let mut best = 0usize;
    for (_pair, ml, _wait) in decomps {
        let mut koutsu = meld_koutsu.clone();
        for m in &ml {
            if let Mentsu::Koutsu(t) = m {
                if let TileType::Number { suit, value } = t.tile_type {
                    koutsu.push((suit, value));
                }
            }
        }
        best = best.max(longest_consecutive_same_suit(&koutsu));
    }
    best
}

/// (suit, value) の刻子集合から、同一スーツで連続する数値の最長ランを返す。
fn longest_consecutive_same_suit(koutsu: &[(Suit, u8)]) -> usize {
    let mut best = 0usize;
    for suit in [Suit::Man, Suit::Pin, Suit::Sou] {
        let mut present = [false; 10]; // value 1..=9
        for (s, v) in koutsu {
            if *s == suit && (1..=9).contains(v) {
                present[*v as usize] = true;
            }
        }
        let mut run = 0usize;
        for v in 1..=9 {
            if present[v] {
                run += 1;
                best = best.max(run);
            } else {
                run = 0;
            }
        }
    }
    best
}

/// チー副露 tiles から順子の最小牌を返す。
fn chi_start(tiles: &[Tile]) -> Option<Tile> {
    let mut min: Option<Tile> = None;
    for t in tiles {
        if let TileType::Number { value, .. } = t.tile_type {
            match min {
                None => min = Some(*t),
                Some(m) => {
                    if let TileType::Number { value: mv, .. } = m.tile_type {
                        if value < mv {
                            min = Some(*t);
                        }
                    }
                }
            }
        }
    }
    min
}

/// 一盃口の数 (同一順子のペア数)。二盃口なら 2 を返す。
fn count_iipeikou(all: &[SMentsu]) -> u32 {
    let mut shuntsu: Vec<Tile> = Vec::new();
    for s in all {
        if let Mentsu::Shuntsu(start) = s.mentsu {
            shuntsu.push(start);
        }
    }
    let mut pairs = 0u32;
    let mut seen: Vec<(Tile, usize)> = Vec::new();
    for st in &shuntsu {
        if let Some(e) = seen.iter_mut().find(|(t, _)| t == st) {
            e.1 += 1;
        } else {
            seen.push((*st, 1));
        }
    }
    for (_, c) in seen {
        pairs += (c / 2) as u32;
    }
    pairs
}

/// 三色同順: ある数値 v で man/pin/sou すべてに Shuntsu(v) が存在。
fn has_sanshoku_doujun(all: &[SMentsu]) -> bool {
    for v in 1u8..=7 {
        let mut suits = [false; 3];
        for s in all {
            if let Mentsu::Shuntsu(start) = s.mentsu {
                if let TileType::Number { suit, value } = start.tile_type {
                    if value == v {
                        suits[suit_index(suit)] = true;
                    }
                }
            }
        }
        if suits.iter().all(|&b| b) {
            return true;
        }
    }
    false
}

/// 一気通貫: ある suit で start=1,4,7 の順子が揃う。
fn has_ittsu(all: &[SMentsu]) -> bool {
    for suit in [Suit::Man, Suit::Pin, Suit::Sou] {
        let mut starts = [false; 8]; // index = start value (1..=7)
        for s in all {
            if let Mentsu::Shuntsu(start) = s.mentsu {
                if let TileType::Number { suit: su, value } = start.tile_type {
                    if su == suit && (1..=7).contains(&value) {
                        starts[value as usize] = true;
                    }
                }
            }
        }
        if starts[1] && starts[4] && starts[7] {
            return true;
        }
    }
    false
}

enum ChantaKind {
    Junchan,
    Chanta,
    None,
}

/// チャンタ / 純チャン判定。
/// 全ての面子・雀頭が么九 (1/9/字) を含み、かつ順子を 1 つ以上含む場合に成立。
/// 字牌を含まない (= 全て老頭牌) なら純チャン、字牌を含むならチャンタ。
fn chanta_kind(all: &[SMentsu], pair: &Tile) -> ChantaKind {
    let mut has_shuntsu = false;
    let mut has_honor = false;

    // 雀頭
    if !tile_is_yaochu(pair) {
        return ChantaKind::None;
    }
    if matches!(pair.tile_type, TileType::Honor(_)) {
        has_honor = true;
    }

    for s in all {
        match s.mentsu {
            Mentsu::Shuntsu(start) => {
                has_shuntsu = true;
                // 順子が么九を含む = start==1 (123) or start==7 (789)
                if let TileType::Number { value, .. } = start.tile_type {
                    if value != 1 && value != 7 {
                        return ChantaKind::None;
                    }
                } else {
                    return ChantaKind::None;
                }
            }
            Mentsu::Koutsu(t) => {
                if !tile_is_yaochu(&t) {
                    return ChantaKind::None;
                }
                if matches!(t.tile_type, TileType::Honor(_)) {
                    has_honor = true;
                }
            }
        }
    }

    if !has_shuntsu {
        // 順子なし = 混老頭 (字あり) / 清老頭 (字なし) の領域なので、ここでは扱わない。
        return ChantaKind::None;
    }
    if has_honor {
        ChantaKind::Chanta
    } else {
        ChantaKind::Junchan
    }
}

/// 三色同刻: ある数値 v で man/pin/sou すべてに Koutsu(v) が存在。
fn has_sanshoku_doukou(all: &[SMentsu]) -> bool {
    for v in 1u8..=9 {
        let mut suits = [false; 3];
        for s in all {
            if let Mentsu::Koutsu(t) = s.mentsu {
                if let TileType::Number { suit, value } = t.tile_type {
                    if value == v {
                        suits[suit_index(suit)] = true;
                    }
                }
            }
        }
        if suits.iter().all(|&b| b) {
            return true;
        }
    }
    false
}

/// 小三元: 三元牌 2 種が刻子、1 種が雀頭。
fn is_shousangen(all: &[SMentsu], pair: &Tile) -> bool {
    let pair_sangen = match pair.tile_type {
        TileType::Honor(h) => matches!(h, Honor::Haku | Honor::Hatsu | Honor::Chun),
        _ => false,
    };
    if !pair_sangen {
        return false;
    }
    let mut sangen_koutsu = 0;
    for s in all {
        if let Mentsu::Koutsu(t) = s.mentsu {
            if let TileType::Honor(h) = t.tile_type {
                if matches!(h, Honor::Haku | Honor::Hatsu | Honor::Chun) {
                    sangen_koutsu += 1;
                }
            }
        }
    }
    sangen_koutsu == 2
}

/// 平和: 全て順子、雀頭が役牌でない (三元牌 / 場風 / 自風)、待ちが両面。
fn is_pinfu_decomp(all: &[SMentsu], pair: &Tile, wait: MachiKind, ctx: &ScoringContext) -> bool {
    if !all.iter().all(|s| matches!(s.mentsu, Mentsu::Shuntsu(_))) {
        return false;
    }
    if !matches!(wait, MachiKind::Ryanmen) {
        return false;
    }
    if pair_is_yakuhai(pair, ctx) {
        return false;
    }
    true
}

/// 符計算。10 切り上げ済みの値を返す。
fn calc_fu(
    all: &[SMentsu],
    pair: &Tile,
    wait: MachiKind,
    ctx: &ScoringContext,
    is_menzen: bool,
    is_pinfu: bool,
) -> u32 {
    if is_pinfu {
        // 平和ツモ 20 / 平和ロン 30 (門前ロン符 10)。
        return if ctx.is_tsumo { 20 } else { 30 };
    }

    let mut fu = 20u32; // 基本符
    if is_menzen && !ctx.is_tsumo {
        fu += 10; // 門前ロン
    }
    if ctx.is_tsumo {
        fu += 2; // ツモ符
    }

    // 雀頭役牌符
    fu += pair_yakuhai_fu(pair, ctx);

    // 面子符
    for s in all {
        if let Mentsu::Koutsu(t) = s.mentsu {
            let yaochu = tile_is_yaochu(&t);
            fu += match (s.is_kan, s.concealed, yaochu) {
                (true, true, true) => 32,  // 暗槓 么九
                (true, true, false) => 16, // 暗槓 中張
                (true, false, true) => 16, // 明槓 么九
                (true, false, false) => 8, // 明槓 中張
                (false, true, true) => 8,  // 暗刻 么九
                (false, true, false) => 4, // 暗刻 中張
                (false, false, true) => 4, // 明刻 么九
                (false, false, false) => 2, // 明刻 中張
            };
        }
    }

    // 待ち符
    fu += match wait {
        MachiKind::Kanchan | MachiKind::Penchan | MachiKind::Tanki => 2,
        MachiKind::Ryanmen | MachiKind::Shanpon => 0,
    };

    // 喰い平和形ロン (副露で 20 符のまま) は 30 符に切り上げる慣習。
    if fu == 20 && !ctx.is_tsumo {
        fu = 30;
    }

    // 10 の位に切り上げ
    ((fu + 9) / 10) * 10
}

/// 雀頭が役牌 (三元牌 / 場風 / 自風) か。
fn pair_is_yakuhai(pair: &Tile, ctx: &ScoringContext) -> bool {
    if let TileType::Honor(h) = pair.tile_type {
        matches!(h, Honor::Haku | Honor::Hatsu | Honor::Chun)
            || h == ctx.round_wind
            || h == ctx.seat_wind
    } else {
        false
    }
}

/// 雀頭役牌符 (三元牌 +2、場風 +2、自風 +2。連風牌は +4)。
fn pair_yakuhai_fu(pair: &Tile, ctx: &ScoringContext) -> u32 {
    let mut fu = 0;
    if let TileType::Honor(h) = pair.tile_type {
        if matches!(h, Honor::Haku | Honor::Hatsu | Honor::Chun) {
            fu += 2;
        }
        if h == ctx.round_wind {
            fu += 2;
        }
        if h == ctx.seat_wind {
            fu += 2;
        }
    }
    fu
}

fn tile_is_yaochu(t: &Tile) -> bool {
    match t.tile_type {
        TileType::Number { value, .. } => value == 1 || value == 9,
        TileType::Honor(_) => true,
    }
}

fn suit_index(s: Suit) -> usize {
    match s {
        Suit::Man => 0,
        Suit::Pin => 1,
        Suit::Sou => 2,
    }
}

/// 赤ドラ牌を通常牌に正規化する (is_red を落とす)。
fn strip_red(t: &Tile) -> Tile {
    match t.tile_type {
        TileType::Number { suit, value } => Tile::new_number(suit, value, false),
        TileType::Honor(_) => *t,
    }
}
