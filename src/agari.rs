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
    let mut results: Vec<(Decomposition, MachiKind)> = Vec::new();
    if tiles.len() != 14 {
        return results;
    }
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

    for &pair_tile in &unique {
        let cnt = sorted.iter().filter(|t| **t == pair_tile).count();
        if cnt < 2 {
            continue;
        }
        // 雀頭として 2 枚抜く
        let mut rest = sorted.clone();
        remove_n(&mut rest, &pair_tile, 2);

        // rest から 4 面子を取り出す全パターン
        let mut acc: Vec<Mentsu> = Vec::new();
        let mut mentsu_lists: Vec<Vec<Mentsu>> = Vec::new();
        collect_mentsu(&rest, &mut acc, &mut mentsu_lists);

        for ml in mentsu_lists {
            let dec = Decomposition {
                pair: pair_tile,
                mentsu: [ml[0], ml[1], ml[2], ml[3]],
            };
            // この分解で winning_tile がどう使われるかを判定。
            // 単騎: winning_tile == pair_tile かつ pair の 2 枚目として使われた
            // シャンポン: winning_tile が刻子の 3 枚目として使われた (かつ別の刻子に同種牌がある必要はない、刻子の 3 枚目という解釈)
            // 順子待ち: winning_tile が順子の一部
            //
            // 「winning_tile を取り除いた 13 枚 + 戻す」という考え方ではなく、
            // 分解結果上、winning_tile の出現位置から判定する。
            let kinds = classify_machi(&dec, winning_tile);
            for k in kinds {
                results.push((dec.clone(), k));
            }
        }
    }

    // 同一 (分解, 待ち形) の重複を除く
    let mut dedup: Vec<(Decomposition, MachiKind)> = Vec::new();
    for r in results {
        if !dedup.iter().any(|(d, k)| decomp_eq(d, &r.0) && *k == r.1) {
            dedup.push(r);
        }
    }
    dedup
}

fn decomp_eq(a: &Decomposition, b: &Decomposition) -> bool {
    if a.pair != b.pair {
        return false;
    }
    // 面子の集合として一致するか
    let mut bm: Vec<Mentsu> = b.mentsu.to_vec();
    for m in a.mentsu.iter() {
        if let Some(pos) = bm.iter().position(|x| x == m) {
            bm.remove(pos);
        } else {
            return false;
        }
    }
    bm.is_empty()
}

/// 分解上、winning_tile がどの待ち形に該当するかの候補を返す。
///
/// 1 つの分解でも winning_tile の用途が複数解釈できる場合があるため Vec で返す。
/// 例: 1m 1m 1m + winning 1m の刻子は、ペアを 1m1m とすればシャンポン、
///     刻子に組み込めばツモった 1m が「刻子の 3 枚目」となるが、形上は刻子完成。
///     ここでは「刻子の最後の 1 枚」はシャンポンとは扱わない (刻子としては既に完成しており、
///     待ちはペア側の 1m である) — 分解列挙時にペアと刻子の組み合わせは別分解として出る。
fn classify_machi(dec: &Decomposition, winning_tile: &Tile) -> Vec<MachiKind> {
    let mut kinds: Vec<MachiKind> = Vec::new();

    // 単騎: pair == winning_tile
    if dec.pair == *winning_tile {
        kinds.push(MachiKind::Tanki);
    }

    for m in dec.mentsu.iter() {
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
                        // 順子の最小牌 — 12_3 待ちなど。
                        // 完成順子 (a,a+1,a+2) で winning=a。
                        // value == 7 のとき (7,8,9) → 辺張 (789 の 7 待ち)
                        // value >= 1 で v-1 が無い場合 (今ここでは順子は完成しているので情報不足)
                        // 簡略化: value == 7 → 辺張、それ以外 → 両面 (両面・嵌張の区別は他の解釈でカバー)
                        if value == 7 {
                            kinds.push(MachiKind::Penchan);
                        } else {
                            kinds.push(MachiKind::Ryanmen);
                        }
                    } else if winning_tile == &t3 {
                        // 順子の最大牌 — _12 3 待ちなど。
                        // value == 1 → (1,2,3) で winning=3 は辺張 (123 の 3 待ち)
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

    kinds
}

/// `rest` 牌列 (12 枚 = 4 面子) から面子 4 つを取り出す全パターン。
fn collect_mentsu(
    rest: &[Tile],
    acc: &mut Vec<Mentsu>,
    out: &mut Vec<Vec<Mentsu>>,
) {
    if rest.is_empty() {
        if acc.len() == 4 {
            out.push(acc.clone());
        }
        return;
    }
    let head = rest[0];

    // 刻子
    let cnt = rest.iter().filter(|t| **t == head).count();
    if cnt >= 3 {
        let mut next = rest.to_vec();
        remove_n(&mut next, &head, 3);
        acc.push(Mentsu::Koutsu(head));
        collect_mentsu(&next, acc, out);
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
                collect_mentsu(&next, acc, out);
                acc.pop();
            }
        }
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
    diff_total == 1 && extras.iter().filter(|&&x| x == 1).count() == 1
        && {
            let _ = s; // suit は確定済みなので未使用警告抑制
            true
        }
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
        // 1m 1m 1m / 2m 2m 2m / 3p 3p 3p / 4s 4s / 5p 5p 5p
        // 4s 4s 雀頭 + 残り 1m/2m/3p/5p 全部刻子。シャンポン待ちは無し (雀頭は 4s 固定)。
        // シャンポン例にするには: 1m1m1m 2m2m2m 3p3p3p 4s4s 5p5p で winning=4s or 5p
        let tiles: Vec<Tile> = vec![
            tm(1), tm(1), tm(1),
            tm(2), tm(2), tm(2),
            tp(3), tp(3), tp(3),
            ts(4), ts(4),
            tp(5), tp(5), tp(5),
        ];
        // ↑ これだと刻子は 1m,2m,3p,5p の 4 つ + 雀頭 4s で四暗刻単騎 (4s 単騎) になってしまう。
        // シャンポンを作るには: 1m1m1m 2m2m2m 3p3p3p 5p5p + 待ち 4s4s か 5p5p ペアのどちらかを刻子にする → 5p5p5p ならシャンポン待ちは消える。
        // 簡略化のため、シャンポン形は別途構築:
        //   1m1m1m 2m2m2m 3p3p3p + 4s4s + 5p5p で 13 枚 + 4s で 4s4s4s 完成 = シャンポン
        let tiles2: Vec<Tile> = vec![
            tm(1), tm(1), tm(1),
            tm(2), tm(2), tm(2),
            tp(3), tp(3), tp(3),
            ts(4), ts(4), ts(4),
            tp(5), tp(5),
        ];
        let winning = ts(4);
        // 4s は刻子の 3 枚目として組み込める → シャンポン (5p5p 雀頭, 1m/2m/3p/4s 刻子)
        // または 5p5p に対しても刻子待ち? 14 枚中 5p は 2 枚のみ。シャンポン待ち = 5p4s シャンポンではなく、
        // 5p5p 雀頭 + 4s 刻子完成 = 単一解釈
        // 本来の四暗刻シャンポン待ちは: 1m1m1m 2m2m2m 3p3p3p 4s4s 5p5p で待ち 4s/5p (シャンポン)
        // ここでは 4s ツモ和了/ロン両方を検証
        assert!(is_suuankou(&tiles2, &winning, true), "シャンポン待ちツモは四暗刻成立");
        assert!(!is_suuankou(&tiles2, &winning, false), "シャンポン待ちロンは四暗刻不成立 (三暗刻に格下げ)");
        // 未使用変数警告抑制
        let _ = tiles;
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
}
