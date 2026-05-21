//! `Hand::compute_machi_tiles` の副露対応 (Issue #43) 検証。
//!
//! 元実装は副露があると early-return で空 Vec を返していたため、
//! 鳴いた手のテンパイ表示・待ち列挙・フリテン判定 (Issue #56) が機能しなかった。
//! 本テストでは:
//!   1. ポン 1 つ + テンパイ手 → 期待する待ち牌が返ること
//!   2. カン (明槓) 1 つ + テンパイ手 → 同上
//!   3. 副露 2 つ + 残り 7 枚 → 同上
//!   4. 副露なしの既存挙動 (回帰確認) が変わっていないこと
//! を確認する。
//!
//! 「副露ありで `can_win` が成立する 1 枚集合 == compute_machi_tiles の結果」を
//! 期待する。`can_win` は副露込みの 4 面子 1 雀頭判定を `check_normal_win` で
//! 行うため、`compute_machi_tiles` は 34 種を試すだけで正しい結果になるはず。

use xmj_core::hand::{Hand, Meld, MeldType};
use xmj_core::tile::{Honor, Suit, Tile};

fn pon(tile: Tile) -> Meld {
    Meld {
        meld_type: MeldType::Pon,
        tiles: vec![tile, tile, tile],
        is_open: true,
    }
}

fn chi(suit: Suit, start: u8) -> Meld {
    Meld {
        meld_type: MeldType::Chi,
        tiles: vec![
            Tile::new_number(suit, start, false),
            Tile::new_number(suit, start + 1, false),
            Tile::new_number(suit, start + 2, false),
        ],
        is_open: true,
    }
}

fn kan_open(tile: Tile) -> Meld {
    Meld {
        meld_type: MeldType::Kan,
        tiles: vec![tile, tile, tile, tile],
        is_open: true,
    }
}

/// 手牌の各タイルを順番に `add_tile` で投入する。
fn build_hand(tiles: &[Tile], melds: Vec<Meld>) -> Hand {
    let mut hand = Hand::new();
    // meld の構成牌を一度 hand に入れてから add_meld で副露化 (構成牌は hand から除かれる)。
    // 「add_meld は構成牌を tiles から remove」前提なので、melds 構築用ダミー tile を先に入れる。
    for m in &melds {
        for t in &m.tiles {
            hand.add_tile(*t);
        }
    }
    for m in melds {
        hand.add_meld(m);
    }
    for t in tiles {
        hand.add_tile(*t);
    }
    hand
}

/// ポン 1m + 残り手牌 10 枚で待ち牌列挙が成立する。
/// 残り手牌: 234p 555p 789p 66s + 6s (10 枚) → 6s 単騎 (66s が雀頭の片割れ) ではなく
/// 234p / 555p / 789p / 6s 6s から残り 1 面子が必要。1m ポンと合わせて
/// 副露 1 + 234p + 555p + 789p + (待ちが雀頭 6s6s で完成) ... 違う、面子が 1 つ足りない。
///
/// 簡単のため確定形を採用: 副露 1m ポン (1 面子) + 234p 567p 789p (3 面子) + 6s + 6s (雀頭)
/// → 残り手牌 10 枚 = 234p 567p 789p 66s で構成済み、待ちは 0 (= 既に和了形)。
/// 代わりに 1 枚抜いた状態を仕込む: 234p 567p 789p 6s + (待ち) で待ち 6s 単騎。
/// 残り手牌 10 枚: 2p 3p 4p 5p 6p 7p 7p 8p 9p 6s → 234p / 567p / 789p / 6s 単騎 → 待ち 6s
#[test]
fn compute_machi_with_pon_meld_returns_tanki_wait() {
    let tiles = vec![
        Tile::new_number(Suit::Pin, 2, false),
        Tile::new_number(Suit::Pin, 3, false),
        Tile::new_number(Suit::Pin, 4, false),
        Tile::new_number(Suit::Pin, 5, false),
        Tile::new_number(Suit::Pin, 6, false),
        Tile::new_number(Suit::Pin, 7, false),
        Tile::new_number(Suit::Pin, 7, false),
        Tile::new_number(Suit::Pin, 8, false),
        Tile::new_number(Suit::Pin, 9, false),
        Tile::new_number(Suit::Sou, 6, false),
    ];
    let melds = vec![pon(Tile::new_number(Suit::Man, 1, false))];
    let hand = build_hand(&tiles, melds);

    let waits = hand.compute_machi_tiles();
    assert!(
        waits.contains(&Tile::new_number(Suit::Sou, 6, false)),
        "6s 単騎待ちが含まれる: {:?}",
        waits
    );
    // 6s 単騎の唯一待ちなので 1 枚だけ
    assert_eq!(waits.len(), 1, "単騎待ちなので待ち牌は 1 種類のみ: {:?}", waits);
}

/// 明槓 1 つ + 残り手牌 10 枚でもリャンメン待ちが正しく出る。
/// 副露: 9m 明槓
/// 残り手牌: 1p 2p 3p 4p 5p 6p 7p 8p 4s 4s → 待ちは 5s/6s ではなく、3s / 6s なし。
/// 整理: 123p 456p 78p + 44s → 78p は両面 (6p/9p の片方は 8p+9p で塞いでないので 6p/9p 待ち)
/// あれ、78p なら 6p か 9p 待ちのリャンメン。さらに 44s は雀頭。
/// よって面子 4 = カン(9m) + 123p + 456p + (78p+待ち)、雀頭 = 44s
/// 待ち = 6p または 9p (リャンメン待ち)
#[test]
fn compute_machi_with_kan_meld_returns_ryanmen_wait() {
    let tiles = vec![
        Tile::new_number(Suit::Pin, 1, false),
        Tile::new_number(Suit::Pin, 2, false),
        Tile::new_number(Suit::Pin, 3, false),
        Tile::new_number(Suit::Pin, 4, false),
        Tile::new_number(Suit::Pin, 5, false),
        Tile::new_number(Suit::Pin, 6, false),
        Tile::new_number(Suit::Pin, 7, false),
        Tile::new_number(Suit::Pin, 8, false),
        Tile::new_number(Suit::Sou, 4, false),
        Tile::new_number(Suit::Sou, 4, false),
    ];
    let melds = vec![kan_open(Tile::new_number(Suit::Man, 9, false))];
    let hand = build_hand(&tiles, melds);

    let waits = hand.compute_machi_tiles();
    assert!(
        waits.contains(&Tile::new_number(Suit::Pin, 6, false)),
        "リャンメン待ち 6p が含まれる: {:?}",
        waits
    );
    assert!(
        waits.contains(&Tile::new_number(Suit::Pin, 9, false)),
        "リャンメン待ち 9p が含まれる: {:?}",
        waits
    );
}

/// 副露 2 つ (ポン + チー) + 残り手牌 7 枚でも待ちが取れる。
/// 副露: 1m ポン + 4m5m6m チー
/// 残り手牌: 7s 8s 9s 中 中 中 5p (7 枚)
/// → 4 面子分: 1m ポン + 4m5m6m チー + 7s8s9s + 中刻子 = 4 面子完成、雀頭 = 5p 単騎
/// 待ち: 5p 単騎
#[test]
fn compute_machi_with_two_melds_returns_tanki_wait() {
    let tiles = vec![
        Tile::new_number(Suit::Sou, 7, false),
        Tile::new_number(Suit::Sou, 8, false),
        Tile::new_number(Suit::Sou, 9, false),
        Tile::new_honor(Honor::Chun),
        Tile::new_honor(Honor::Chun),
        Tile::new_honor(Honor::Chun),
        Tile::new_number(Suit::Pin, 5, false),
    ];
    let melds = vec![
        pon(Tile::new_number(Suit::Man, 1, false)),
        chi(Suit::Man, 4),
    ];
    let hand = build_hand(&tiles, melds);

    let waits = hand.compute_machi_tiles();
    assert!(
        waits.contains(&Tile::new_number(Suit::Pin, 5, false)),
        "5p 単騎待ちが含まれる: {:?}",
        waits
    );
    assert_eq!(waits.len(), 1, "単騎待ちなので 1 種のみ: {:?}", waits);
}

/// 副露なしの既存挙動 (回帰確認)。
/// 13 枚: 234m 234p 234s 567s 88m → 既にテンパイ。雀頭 = 88m、面子は 234m/234p/234s/567s で
/// 4 つ揃っているように見えるが 13 枚なので 1 面子分 1 牌足りない構成。
/// 整理: 22m 33m 44m 234p 234s 567s 88m 8m? → 違う、ちゃんと組む。
/// 単純化: 単騎待ち 13 枚 = 234m 234p 234s 567s 8m 8m → 雀頭 88m + 234m + 234p + 234s + 567s = 14 枚必要。
/// 13 枚に削減: 234m 234p 234s 567s 8m (= 13 枚) → 8m 単騎待ち
#[test]
fn compute_machi_without_melds_regression_tanki() {
    let tiles = vec![
        Tile::new_number(Suit::Man, 2, false),
        Tile::new_number(Suit::Man, 3, false),
        Tile::new_number(Suit::Man, 4, false),
        Tile::new_number(Suit::Pin, 2, false),
        Tile::new_number(Suit::Pin, 3, false),
        Tile::new_number(Suit::Pin, 4, false),
        Tile::new_number(Suit::Sou, 2, false),
        Tile::new_number(Suit::Sou, 3, false),
        Tile::new_number(Suit::Sou, 4, false),
        Tile::new_number(Suit::Sou, 5, false),
        Tile::new_number(Suit::Sou, 6, false),
        Tile::new_number(Suit::Sou, 7, false),
        Tile::new_number(Suit::Man, 8, false),
    ];
    let hand = build_hand(&tiles, vec![]);

    let waits = hand.compute_machi_tiles();
    assert!(
        waits.contains(&Tile::new_number(Suit::Man, 8, false)),
        "副露なし 8m 単騎待ち: {:?}",
        waits
    );
    assert_eq!(waits.len(), 1, "単騎なので 1 種: {:?}", waits);
}

/// 副露ありで「あと 1 枚足りない」非テンパイ手は空 Vec を返す。
/// 副露 1 (ポン 1m) + 残り手牌 9 枚 (10 枚未満) → 期待枚数違いなので Vec::new()
#[test]
fn compute_machi_with_meld_wrong_count_returns_empty() {
    // 9 枚しかない (本来 10 枚必要)
    let tiles = vec![
        Tile::new_number(Suit::Pin, 2, false),
        Tile::new_number(Suit::Pin, 3, false),
        Tile::new_number(Suit::Pin, 4, false),
        Tile::new_number(Suit::Pin, 5, false),
        Tile::new_number(Suit::Pin, 6, false),
        Tile::new_number(Suit::Pin, 7, false),
        Tile::new_number(Suit::Pin, 8, false),
        Tile::new_number(Suit::Sou, 4, false),
        Tile::new_number(Suit::Sou, 4, false),
    ];
    let melds = vec![pon(Tile::new_number(Suit::Man, 1, false))];
    let hand = build_hand(&tiles, melds);

    let waits = hand.compute_machi_tiles();
    assert!(
        waits.is_empty(),
        "期待枚数違いは空 Vec を返す: {:?}",
        waits
    );
}
