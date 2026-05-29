//! 和了形の分解と待ち形判定。
//!
//! `Hand::can_win` は「和了形が存在するか」だけを返し、
//! 「どの面子分解で、winning_tile がどの待ち形に属するか」までは判定しない。
//! 本モジュールは副露なし 14 枚手に対して、可能な (雀頭, 4 面子) 分解を列挙し、
//! winning_tile の使われ方から待ち形 (両面 / 嵌張 / 辺張 / シャンポン / 単騎) を抽出する。
//!
//! Issue #34: winning_tile 推定の待ち形精度向上。
//! - extract_agari が「最初に和了する winning_tile」ではなく「最高得点の解釈」を選ぶ
//! - 平和（両面待ち）/ 四暗刻単騎 / 九蓮宝燈 9 面待ち の正確判定
//! - 13 枚手からの待ち牌列挙 (`Hand::compute_machi_tiles`)
//!
//! 副露あり手の待ち形列挙は本モジュールではスコープ外 (#33 で和了確定だけ済み)。

use crate::tile::{Tile, TileType, Suit, Honor};

/// 1 面子。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mentsu {
    /// 順子 (start_tile から +0/+1/+2)。数牌のみ。
    Shuntsu(Tile),
    /// 刻子。
    Koutsu(Tile),
}

/// 14 枚手の通常形分解 (副露なし)。
#[derive(Debug, Clone)]
pub struct Decomposition {
    pub pair: Tile,
    pub mentsu: [Mentsu; 4],
}

/// 待ち形。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachiKind {
    /// 両面待ち。順子 23 に 1/4 が付くなど (端を含まない)。
    Ryanmen,
    /// 嵌張待ち (例: 13 に 2)。
    Kanchan,
    /// 辺張待ち (例: 12 に 3、または 89 に 7)。
    Penchan,
    /// シャンポン待ち。
    Shanpon,
    /// 単騎待ち。
    Tanki,
}

/// 14 枚手と winning_tile から、可能な (分解, 待ち形) 組を全列挙する。
///
/// 注意: 副露ありの場合は `melds` を別途扱う必要があるため、本関数は
/// 「副露なしの 14 枚手」専用。`tiles.len() == 14` を前提とする。
pub fn enumerate_decompositions_with_wait(
    tiles: &[Tile],
    winning_tile: &Tile,
) -> Vec<(Decomposition, MachiKind)> {
    if tiles.len() != 14 {
        return Vec::new();
    }
    // 4 面子 1 雀頭 (副露なし) として一般版に委譲し、固定 4 面子配列に詰め直す。
    enumerate_concealed_decomps(tiles, winning_tile, 4)
        .into_iter()
        .map(|(pair, ml, kind)| {
            (
                Decomposition {
                    pair,
                    mentsu: [ml[0], ml[1], ml[2], ml[3]],
                },
                kind,
            )
        })
        .collect()
}

/// 手牌部分 (winning_tile を含む) を `(雀頭, melds_needed 個の面子, 待ち形)` に全分解する。
///
/// 副露あり手では `melds_needed = 4 - melds.len()` を渡す。手牌枚数は
/// `3 * melds_needed + 2` であることを前提とする (雀頭 2 + 面子 3×N)。
/// 副露牌は含めない。winning_tile は必ず手牌部分にツモ / ロンで合流するため
/// concealed 側に含まれる。
pub fn enumerate_concealed_decomps(
    tiles: &[Tile],
    winning_tile: &Tile,
    melds_needed: usize,
) -> Vec<(Tile, Vec<Mentsu>, MachiKind)> {
    let mut results: Vec<(Tile, Vec<Mentsu>, MachiKind)> = Vec::new();
    if tiles.len() != melds_needed * 3 + 2 {
        return results;
    }
    // 赤ドラは is_red が PartialEq に含まれるため、分解前に通常牌へ正規化する。
    // 正規化しないと「赤5 + 通常5」が別牌扱いになり雀頭/刻子が組めず和了形を取り逃す。
    // 赤ドラの枚数は scoring 側で別途カウントするので、構造判定では色を落としてよい。
    let tiles: Vec<Tile> = tiles.iter().map(|t| strip_red(t)).collect();
    let winning_norm = strip_red(winning_tile);
    let winning_tile = &winning_norm;
    if !tiles.contains(winning_tile) {
        return results;
    }

    // 雀頭候補ループ
    let mut sorted = tiles.to_vec();
    sort_tiles(&mut sorted);

    let unique: Vec<Tile> = {
        let mut v: Vec<Tile> = Vec::new();
        for t in &sorted {
            if !v.contains(t) {
                v.push(*t);
            }
        }
        v
    };

    let mut raw: Vec<(Tile, Vec<Mentsu>, MachiKind)> = Vec::new();
    for &pair_tile in &unique {
        let cnt = sorted.iter().filter(|t| **t == pair_tile).count();
        if cnt < 2 {
            continue;
        }
        // 雀頭として 2 枚抜く
        let mut rest = sorted.clone();
        remove_n(&mut rest, &pair_tile, 2);

        // rest から melds_needed 個の面子を取り出す全パターン
        let mut acc: Vec<Mentsu> = Vec::new();
        let mut mentsu_lists: Vec<Vec<Mentsu>> = Vec::new();
        collect_mentsu_n(&rest, melds_needed, &mut acc, &mut mentsu_lists);

        for ml in mentsu_lists {
            let kinds = classify_machi_list(&ml, &pair_tile, winning_tile);
            for k in kinds {
                raw.push((pair_tile, ml.clone(), k));
            }
        }
    }

    // 同一 (雀頭, 面子集合, 待ち形) の重複を除く
    for r in raw {
        if !results
            .iter()
            .any(|(p, m, k)| *p == r.0 && mentsu_set_eq(m, &r.1) && *k == r.2)
        {
            results.push(r);
        }
    }
    results
}

/// 面子リスト + 雀頭から winning_tile の待ち形候補を返す (面子数が 4 未満でも動く)。
fn classify_machi_list(mentsu: &[Mentsu], pair: &Tile, winning_tile: &Tile) -> Vec<MachiKind> {
    let mut kinds: Vec<MachiKind> = Vec::new();
    if *pair == *winning_tile {
        kinds.push(MachiKind::Tanki);
    }
    for m in mentsu.iter() {
        classify_one_mentsu(m, winning_tile, &mut kinds);
    }
    let mut uniq: Vec<MachiKind> = Vec::new();
    for k in kinds.into_iter() {
        if !uniq.contains(&k) {
            uniq.push(k);
        }
    }
    uniq
}

/// 面子集合の順序非依存比較。
fn mentsu_set_eq(a: &[Mentsu], b: &[Mentsu]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut bm: Vec<Mentsu> = b.to_vec();
    for m in a {
        if let Some(pos) = bm.iter().position(|x| x == m) {
            bm.remove(pos);
        } else {
            return false;
        }
    }
    bm.is_empty()
}

/// 1 つの面子に対し、winning_tile がどの待ち形に該当するかを `kinds` に push する。
fn classify_one_mentsu(m: &Mentsu, winning_tile: &Tile, kinds: &mut Vec<MachiKind>) {
    match m {
        Mentsu::Koutsu(t) => {
            if t == winning_tile {
                // 刻子に winning が含まれる = シャンポン候補。
                kinds.push(MachiKind::Shanpon);
            }
        }
        Mentsu::Shuntsu(start) => {
            if let TileType::Number { suit, value } = start.tile_type {
                let t1 = *start;
                let t2 = Tile::new_number(suit, value + 1, false);
                let t3 = Tile::new_number(suit, value + 2, false);
                if winning_tile == &t1 {
                    // 順子の最小牌。value == 7 → 辺張 (789 の 7)、それ以外 → 両面。
                    if value == 7 {
                        kinds.push(MachiKind::Penchan);
                    } else {
                        kinds.push(MachiKind::Ryanmen);
                    }
                } else if winning_tile == &t3 {
                    // 順子の最大牌。value == 1 → 辺張 (123 の 3)、それ以外 → 両面。
                    if value == 1 {
                        kinds.push(MachiKind::Penchan);
                    } else {
                        kinds.push(MachiKind::Ryanmen);
                    }
                } else if winning_tile == &t2 {
                    // 順子の真ん中 — 嵌張。
                    kinds.push(MachiKind::Kanchan);
                }
            }
        }
    }
}

/// `rest` 牌列から面子 `n` 個を取り出す全パターン。
fn collect_mentsu_n(
    rest: &[Tile],
    n: usize,
    acc: &mut Vec<Mentsu>,
    out: &mut Vec<Vec<Mentsu>>,
) {
    if acc.len() == n {
        if rest.is_empty() {
            out.push(acc.clone());
        }
        return;
    }
    if rest.is_empty() {
        return;
    }
    let head = rest[0];

    // 刻子
    let cnt = rest.iter().filter(|t| **t == head).count();
    if cnt >= 3 {
        let mut next = rest.to_vec();
        remove_n(&mut next, &head, 3);
        acc.push(Mentsu::Koutsu(head));
        collect_mentsu_n(&next, n, acc, out);
        acc.pop();
    }

    // 順子 (数牌のみ)
    if let TileType::Number { suit, value } = head.tile_type {
        if value <= 7 {
            let t2 = Tile::new_number(suit, value + 1, false);
            let t3 = Tile::new_number(suit, value + 2, false);
            if rest.contains(&t2) && rest.contains(&t3) {
                let mut next = rest.to_vec();
                remove_n(&mut next, &head, 1);
                remove_n(&mut next, &t2, 1);
                remove_n(&mut next, &t3, 1);
                acc.push(Mentsu::Shuntsu(head));
                collect_mentsu_n(&next, n, acc, out);
                acc.pop();
            }
        }
    }
}

/// 赤ドラ牌を通常牌に正規化する (is_red を落とす)。数牌のみ影響。
fn strip_red(t: &Tile) -> Tile {
    match t.tile_type {
        TileType::Number { suit, value } => Tile::new_number(suit, value, false),
        TileType::Honor(_) => *t,
    }
}

fn remove_n(v: &mut Vec<Tile>, tile: &Tile, n: usize) {
    let mut removed = 0;
    let mut i = 0;
    while i < v.len() && removed < n {
        if &v[i] == tile {
            v.remove(i);
            removed += 1;
        } else {
            i += 1;
        }
    }
}

pub(crate) fn sort_tiles(tiles: &mut Vec<Tile>) {
    tiles.sort_by(|a, b| {
        match (&a.tile_type, &b.tile_type) {
            (TileType::Number { suit: s1, value: v1 }, TileType::Number { suit: s2, value: v2 }) => {
                let so = |s: &Suit| match s {
                    Suit::Man => 0,
                    Suit::Pin => 1,
                    Suit::Sou => 2,
                };
                so(s1).cmp(&so(s2)).then(v1.cmp(v2))
            }
            (TileType::Honor(h1), TileType::Honor(h2)) => {
                let ho = |h: &Honor| match h {
                    Honor::Ton => 0,
                    Honor::Nan => 1,
                    Honor::Shaa => 2,
                    Honor::Pei => 3,
                    Honor::Haku => 4,
                    Honor::Hatsu => 5,
                    Honor::Chun => 6,
                };
                ho(h1).cmp(&ho(h2))
            }
            (TileType::Number { .. }, TileType::Honor(_)) => std::cmp::Ordering::Less,
            (TileType::Honor(_), TileType::Number { .. }) => std::cmp::Ordering::Greater,
        }
    });
}

// ============ 役判定用ヘルパー (winning_tile / 待ち形を考慮) ============

/// 平和形か。
/// - 副露なし (呼び出し側で保証)
/// - 全て順子 (4 つとも Shuntsu)
/// - 雀頭が役牌ではない (三元牌 / 場風 / 自風は本関数では役牌を「三元牌のみ」で簡略化)
/// - winning_tile が両面待ちで取り込まれている解釈が少なくとも 1 つある
pub fn is_pinfu_shape(tiles14: &[Tile], winning_tile: &Tile) -> bool {
    let decs = enumerate_decompositions_with_wait(tiles14, winning_tile);
    for (dec, kind) in &decs {
        if !matches!(kind, MachiKind::Ryanmen) {
            continue;
        }
        if !dec.mentsu.iter().all(|m| matches!(m, Mentsu::Shuntsu(_))) {
            continue;
        }
        // 雀頭が三元牌でない (簡略化: 場風・自風は本関数では検査対象外。
        // 場風/自風が分かる API が整備されたら拡張する)
        if let TileType::Honor(h) = dec.pair.tile_type {
            if matches!(h, Honor::Haku | Honor::Hatsu | Honor::Chun) {
                continue;
            }
        }
        return true;
    }
    false
}

/// 四暗刻判定 (副露なし 14 枚手専用)。
///
/// - 4 つの面子全てが刻子で、雀頭 1 つ
/// - 単騎和了でない場合 (= シャンポン / 順子待ち) は、ロンだと 1 つの刻子が明刻扱いになる
/// - 本関数はツモ和了および「単騎」解釈時の四暗刻を判定する
pub fn is_suuankou(tiles14: &[Tile], winning_tile: &Tile, is_tsumo: bool) -> bool {
    let decs = enumerate_decompositions_with_wait(tiles14, winning_tile);
    for (dec, kind) in &decs {
        if !dec.mentsu.iter().all(|m| matches!(m, Mentsu::Koutsu(_))) {
            continue;
        }
        match kind {
            MachiKind::Tanki => return true, // 四暗刻単騎 (ロンでも成立)
            MachiKind::Shanpon => {
                if is_tsumo {
                    return true;
                }
                // ロンだと和了牌を含む刻子が明刻になり、暗刻 3 つ + 明刻 1 つ → 三暗刻
                continue;
            }
            _ => continue,
        }
    }
    false
}

/// 九蓮宝燈 (純正含む) 形か。
/// 1112345678999 + どれか 1 枚の同色清一色。
///
/// # 注意
/// 副露を除いた**手牌のみ**を渡すこと (副露込み 14 枚を渡すと誤判定する)。
/// 九蓮宝燈は門前限定役のため、本来副露がある時点で成立しないが、呼び出し側で
/// `melds.is_empty()` を確認してから渡すこと。
pub fn is_chuuren(tiles14: &[Tile]) -> bool {
    if tiles14.len() != 14 {
        return false;
    }
    // 清一色 (字牌なし、1 種類の suit)
    let mut suit: Option<Suit> = None;
    for t in tiles14 {
        match t.tile_type {
            TileType::Number { suit: s, .. } => {
                if let Some(prev) = suit {
                    if prev != s {
                        return false;
                    }
                } else {
                    suit = Some(s);
                }
            }
            _ => return false,
        }
    }
    let s = match suit {
        Some(x) => x,
        None => return false,
    };
    // 各数の出現数を集計し、1112345678999 にどれか 1 枚追加した形か
    let mut counts = [0u8; 10]; // index 1..=9
    for t in tiles14 {
        if let TileType::Number { value, .. } = t.tile_type {
            counts[value as usize] += 1;
        }
    }
    // 1112345678999 = [_,3,1,1,1,1,1,1,1,3]
    let base = [0u8, 3, 1, 1, 1, 1, 1, 1, 1, 3];
    let mut diff_total = 0i32;
    let mut extras = [0i32; 10];
    for v in 1..=9 {
        let d = counts[v] as i32 - base[v] as i32;
        if d < 0 {
            return false;
        }
        extras[v] = d;
        diff_total += d;
    }
    // 1 種類が +1 多いだけ
    let _ = s; // suit は確定済み (上で early return 済み)。今後 suit を見たい拡張のために残す。
    diff_total == 1 && extras.iter().filter(|&&x| x == 1).count() == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tm(v: u8) -> Tile { Tile::new_number(Suit::Man, v, false) }
    fn tp(v: u8) -> Tile { Tile::new_number(Suit::Pin, v, false) }
    fn ts(v: u8) -> Tile { Tile::new_number(Suit::Sou, v, false) }
    fn th(h: Honor) -> Tile { Tile::new_honor(h) }

    #[test]
    fn pinfu_ryanmen_basic() {
        // 234m 234p 234s 567s + 22m + 待ち 3s or 6s (両面)
        // 14 枚: 2m 3m 4m 2p 3p 4p 2s 3s 4s 5s 6s 7s 2m 2m? いや雀頭を変える
        // 雀頭 2m 2m, 順子 3m4m5m? いや簡単に:
        // 1m 1m | 2p 3p 4p | 5p 6p 7p | 2s 3s 4s | 5s 6s 7s    winning 4s (両面 5s 待ちに 4s)? 4s だと嵌張になる
        // 改めて: 雀頭 1m 1m, 順子 234p / 567p / 234s, 待ち: 56s + 7s で 7s ロン → 両面
        let tiles: Vec<Tile> = vec![
            tm(1), tm(1),
            tp(2), tp(3), tp(4),
            tp(5), tp(6), tp(7),
            ts(2), ts(3), ts(4),
            ts(5), ts(6), ts(7),
        ];
        let winning = ts(7);
        assert!(is_pinfu_shape(&tiles, &winning), "両面 7s 待ちは平和成立");
    }

    #[test]
    fn pinfu_kanchan_fails() {
        // 雀頭 1m 1m, 順子 234p / 567p / 234s + 待ち牌 6s 嵌張 (5s 7s に 6s)
        let tiles: Vec<Tile> = vec![
            tm(1), tm(1),
            tp(2), tp(3), tp(4),
            tp(5), tp(6), tp(7),
            ts(2), ts(3), ts(4),
            ts(5), ts(6), ts(7),
        ];
        let winning = ts(6);
        // 6s は順子 567s の真ん中なので嵌張のみ。平和不成立。
        assert!(!is_pinfu_shape(&tiles, &winning), "嵌張 6s 待ちは平和不成立");
    }

    #[test]
    fn pinfu_yakuhai_pair_fails() {
        // 雀頭が中 (役牌)。形は順子 4 つだが平和不成立。
        let tiles: Vec<Tile> = vec![
            th(Honor::Chun), th(Honor::Chun),
            tp(2), tp(3), tp(4),
            tp(5), tp(6), tp(7),
            ts(2), ts(3), ts(4),
            ts(5), ts(6), ts(7),
        ];
        let winning = ts(7);
        assert!(!is_pinfu_shape(&tiles, &winning), "雀頭が三元牌なら平和不成立");
    }

    #[test]
    fn suuankou_tanki_ron_ok() {
        // 1m 1m 1m / 2m 2m 2m / 3p 3p 3p / 4s 4s 4s / 中 中 → 中単騎ロンで四暗刻単騎
        let tiles: Vec<Tile> = vec![
            tm(1), tm(1), tm(1),
            tm(2), tm(2), tm(2),
            tp(3), tp(3), tp(3),
            ts(4), ts(4), ts(4),
            th(Honor::Chun), th(Honor::Chun),
        ];
        let winning = th(Honor::Chun);
        assert!(is_suuankou(&tiles, &winning, false), "単騎ロンでも四暗刻単騎は成立");
    }

    #[test]
    fn suuankou_shanpon_ron_fails() {
        // シャンポン待ち四暗刻: 1m1m1m / 2m2m2m / 3p3p3p / 4s4s / 5p5p の聴牌から
        // 4s をツモ (or ロン)。winning=4s は刻子の 3 枚目として組み込まれ、雀頭は 5p5p。
        // ツモなら四暗刻成立、ロンなら 4s 刻子が明刻扱いになり三暗刻に格下げ。
        let tiles: Vec<Tile> = vec![
            tm(1), tm(1), tm(1),
            tm(2), tm(2), tm(2),
            tp(3), tp(3), tp(3),
            ts(4), ts(4), ts(4),
            tp(5), tp(5),
        ];
        let winning = ts(4);
        assert!(is_suuankou(&tiles, &winning, true), "シャンポン待ちツモは四暗刻成立");
        assert!(!is_suuankou(&tiles, &winning, false), "シャンポン待ちロンは四暗刻不成立 (三暗刻に格下げ)");
    }

    #[test]
    fn chuuren_pure() {
        // 1112345678999 + 5 → 14 枚で九蓮宝燈
        let tiles: Vec<Tile> = vec![
            tm(1), tm(1), tm(1), tm(2), tm(3), tm(4), tm(5), tm(5),
            tm(6), tm(7), tm(8), tm(9), tm(9), tm(9),
        ];
        assert!(is_chuuren(&tiles));
    }

    #[test]
    fn chuuren_9_men_each() {
        // 9 面待ち: 1112345678999 (13 枚) に 1..=9 のどれを加えても和了
        for extra in 1u8..=9u8 {
            let mut tiles: Vec<Tile> = vec![
                tm(1), tm(1), tm(1), tm(2), tm(3), tm(4), tm(5),
                tm(6), tm(7), tm(8), tm(9), tm(9), tm(9),
            ];
            tiles.push(tm(extra));
            assert!(is_chuuren(&tiles), "九蓮 9 面待ち: 追加 {} で成立", extra);
        }
    }

    #[test]
    fn chuuren_not_chinitsu_fails() {
        // 字牌混じり → 九蓮不成立
        let tiles: Vec<Tile> = vec![
            tm(1), tm(1), tm(1), tm(2), tm(3), tm(4), tm(5), tm(5),
            tm(6), tm(7), tm(8), tm(9), tm(9), th(Honor::Ton),
        ];
        assert!(!is_chuuren(&tiles));
    }

    #[test]
    fn kokushi_13_men_wait() {
        // 国士無双 13 面待ち: 13 種么九 1 枚ずつ (= 13 枚) からどの 1 枚を加えても和了
        use crate::hand::Hand;
        let yaochu: Vec<Tile> = vec![
            tm(1), tm(9), tp(1), tp(9), ts(1), ts(9),
            th(Honor::Ton), th(Honor::Nan), th(Honor::Shaa), th(Honor::Pei),
            th(Honor::Haku), th(Honor::Hatsu), th(Honor::Chun),
        ];
        for candidate in &yaochu {
            let mut hand = Hand::new();
            for t in &yaochu {
                hand.add_tile(*t);
            }
            assert!(hand.can_win(candidate), "国士 13 面待ち: {} で和了", candidate.to_string());
        }
    }

    #[test]
    fn compute_machi_tiles_ryanmen() {
        // 13 枚: 雀頭 1m1m + 234p + 567p + 234s + 56s → 4s/7s 両面待ち
        use crate::hand::Hand;
        let tiles: Vec<Tile> = vec![
            tm(1), tm(1),
            tp(2), tp(3), tp(4),
            tp(5), tp(6), tp(7),
            ts(2), ts(3), ts(4),
            ts(5), ts(6),
        ];
        let mut hand = Hand::new();
        for t in &tiles {
            hand.add_tile(*t);
        }
        let waits = hand.compute_machi_tiles();
        assert!(waits.contains(&ts(4)), "4s 待ち: {:?}", waits.iter().map(|t| t.to_string()).collect::<Vec<_>>());
        assert!(waits.contains(&ts(7)), "7s 待ち: {:?}", waits.iter().map(|t| t.to_string()).collect::<Vec<_>>());
    }

    #[test]
    fn enumerate_multi_machi() {
        // 234m / 234m (二盃口的) ではなく、両面・嵌張両解釈ありそうな簡単な例:
        // 雀頭 1m 1m, 順子 234p 567p 234s + 残り 4s 5s + 待ち 3s or 6s
        // ここでは winning=3s で 「3s4s5s 両面」と「2s3s4s の 4s 嵌張」両方は無理
        // (4s 5s 残しで 3s 来ても 4s5s+3s で順子完成、これは両面 3-6 のうち 3 側 → 両面のみ)
        let tiles: Vec<Tile> = vec![
            tm(1), tm(1),
            tp(2), tp(3), tp(4),
            tp(5), tp(6), tp(7),
            ts(2), ts(3), ts(4),
            ts(3), ts(4), ts(5),
        ];
        let winning = ts(3);
        let decs = enumerate_decompositions_with_wait(&tiles, &winning);
        assert!(!decs.is_empty(), "和了形が列挙できる");
        // 少なくとも 1 つは Ryanmen を含む (3s4s5s 順子で 3s が端)
        assert!(
            decs.iter().any(|(_, k)| matches!(k, MachiKind::Ryanmen)),
            "両面解釈が含まれる: {:?}", decs.iter().map(|(_, k)| *k).collect::<Vec<_>>()
        );
    }

    #[test]
    fn pinfu_ryanmen_preferred_over_kanchan() {
        // 両面・嵌張共存手で両面優先 → 平和成立する例。
        // 雀頭 1m1m, 順子 234m / 234p / 567p, 残り 5s6s7s + winning=5s。
        // 5s 単体解釈:
        //   (a) 5s6s7s 順子の最小 (両面: 4s/7s 待ちのうち)? 完成順子 567s で winning=5s は両面 4-7 のうち 4 側相当?
        //       value=5 で「順子の最小牌」かつ value!=7 → Ryanmen
        //   (b) 5s7s+6s? 本手には 6s が無いのでこの解釈は出ない
        // → 嵌張共存にはならないため別構成で:
        // 雀頭 1m1m, 順子 234m / 234p / 4s5s6s + 残り 5p6p7p + winning=6p
        //   (a) 5p6p7p 順子 winning=6p 真ん中 → Kanchan
        //   (b) 567p 順子の最小 winning=5p? いや winning=6p なので別分解 4p5p+5p6p7p? 4p 無い
        // 多面待ちの典型例で素直なものを作る:
        // 雀頭 1m1m, 順子 234m / 234p / 234s, 残り 3p4p5p + 5p6p7p の 7 枚を作るのは無理 (14 枚制約)
        //
        // 簡略化: 雀頭 1m1m / 234m / 234p / 678s / 4s 5s 6s + winning が両面 / 嵌張 同時解釈になる手:
        // 1m1m 234p 567p 345s 456s で winning=6s
        //   解釈 (a): 234p 567p 345s + 456s 完成、3-6 両面? 6s は 456s の最大 (Ryanmen) or 678s 想定?
        // 確実な例: 1m1m / 234p / 234s / 345s / 4s5s と winning=3s/6s
        // → 1m1m + 234p + 345s + 345s + 4s5s と winning=3s or 6s
        //   - winning=3s: (234s + 345s) 両面、もしくは (345s + 3s4s5s) 同分解の別解釈
        // テストを単純化: 多面解釈で「嵌張解釈と両面解釈が両方含まれ、平和が成立する」ことを確認
        let tiles: Vec<Tile> = vec![
            tm(1), tm(1),
            tp(2), tp(3), tp(4),
            tp(5), tp(6), tp(7),
            ts(3), ts(4), ts(5),
            ts(4), ts(5), ts(6),
        ];
        // winning=4s: 解釈 (a) 345s + 456s で 4s が完成順子の一部、嵌張 (3s5s + 4s) や 両面 (45s + ...) など共存
        // 重要なのは「両面解釈があれば平和成立」なので is_pinfu_shape が true を返す
        let winning = ts(4);
        let decs = enumerate_decompositions_with_wait(&tiles, &winning);
        let kinds: Vec<MachiKind> = decs.iter().map(|(_, k)| *k).collect();
        // 少なくとも両面解釈が含まれる
        assert!(
            kinds.iter().any(|k| matches!(k, MachiKind::Ryanmen)),
            "両面解釈が含まれる: {:?}", kinds
        );
        // 全順子 + 雀頭非役牌 + 両面解釈あり → 平和成立
        assert!(is_pinfu_shape(&tiles, &winning), "両面優先で平和成立: kinds={:?}", kinds);
    }
}
