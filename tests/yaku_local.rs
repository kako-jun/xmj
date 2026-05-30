//! #58 ローカル役満 / ローカル役の回帰テスト。
//!
//! allow_local_yakuman フラグ有効時のみ付与される人和 / 大車輪 / 四連刻 / 百万石 /
//! 三連刻 を検証する。デフォルト (false) では付与されないことも確認する。

use xmj_core::hand::Hand;
use xmj_core::scoring::{ScoringContext, ScoringEngine, Yaku};
use xmj_core::tile;
use xmj_core::tile::Tile;

fn score(hand_tiles: Vec<Tile>, winning: Tile, ctx: ScoringContext) -> Option<xmj_core::scoring::ScoringResult> {
    let mut hand = Hand::new();
    for t in hand_tiles {
        hand.add_tile(t);
    }
    ScoringEngine::calculate_score_with_context(&hand, &winning, &ctx)
}

fn local_ctx() -> ScoringContext {
    ScoringContext {
        is_tsumo: false,
        allow_local_yakuman: true,
        ..ScoringContext::default()
    }
}

// ============ 大車輪 (筒子 2-8 七対子形) ============

#[test]
fn daisharin_detected() {
    // 22p33p44p55p66p77p88p、和了 8p。
    let hand = vec![
        tile!(2p), tile!(2p), tile!(3p), tile!(3p),
        tile!(4p), tile!(4p), tile!(5p), tile!(5p),
        tile!(6p), tile!(6p), tile!(7p), tile!(7p),
        tile!(8p),
    ];
    let r = score(hand, tile!(8p), local_ctx()).expect("大車輪で和了");
    assert!(r.yaku.contains(&Yaku::Daisharin), "大車輪: {:?}", r.yaku);
    assert_eq!(r.yakuman_count, 1);
}

#[test]
fn daisharin_not_awarded_when_local_disabled() {
    let hand = vec![
        tile!(2p), tile!(2p), tile!(3p), tile!(3p),
        tile!(4p), tile!(4p), tile!(5p), tile!(5p),
        tile!(6p), tile!(6p), tile!(7p), tile!(7p),
        tile!(8p),
    ];
    // allow_local_yakuman = false (デフォルト) → 大車輪は付かず、七対子+清一色等になる
    let ctx = ScoringContext { is_tsumo: false, ..ScoringContext::default() };
    let r = score(hand, tile!(8p), ctx).expect("七対子清一色で和了");
    assert!(!r.yaku.contains(&Yaku::Daisharin), "ローカル無効では大車輪なし");
    assert_eq!(r.yakuman_count, 0);
}

// ============ 百万石 (萬子のみ・数字合計 100 以上) ============

#[test]
fn hyakumangoku_detected() {
    // 萬子のみで合計 >= 100 になる清一色手。
    // 9m*3 + 8m*3 + 7m*3 + 6m*3 + 9m*2 = 27+24+21+18+18 = 108 >= 100。
    // 構成: 999m 888m 777m 666m + 99m? 9m が 5 枚は不可。別構成:
    // 999m(27) 999m 不可。567m 順子は数が少ない。確実に 100 超える刻子手:
    // 9m9m9m 8m8m8m 7m7m7m 6m6m6m 5m5m = 27+24+21+18+10 = 100。和了 5m。
    let hand = vec![
        tile!(9m), tile!(9m), tile!(9m),
        tile!(8m), tile!(8m), tile!(8m),
        tile!(7m), tile!(7m), tile!(7m),
        tile!(6m), tile!(6m), tile!(6m),
        tile!(5m),
    ];
    let r = score(hand, tile!(5m), local_ctx()).expect("百万石で和了");
    assert!(r.yaku.contains(&Yaku::Hyakumangoku), "百万石: {:?}", r.yaku);
}

#[test]
fn hyakumangoku_rejected_when_sum_below_100() {
    // 低い萬子で合計 < 100 → 百万石なし (清一色などにはなる)。
    // 234m 234m 234m 234m 11m? 構成: 1m1m 2m3m4m ×4 → 1+1 + (2+3+4)*4 = 2+36 = 38。
    let hand = vec![
        tile!(1m), tile!(1m),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(5m), tile!(6m),
    ];
    let r = score(hand, tile!(7m), local_ctx()).expect("清一色で和了");
    assert!(!r.yaku.contains(&Yaku::Hyakumangoku), "合計 100 未満は百万石なし");
}

// ============ 三連刻 (2 飜) / 四連刻 (役満) ============

#[test]
fn sanrenkou_detected() {
    // 222m 333m 444m (連続 3 刻子) + 678p + 9s9s、和了 9s。
    let hand = vec![
        tile!(2m), tile!(2m), tile!(2m),
        tile!(3m), tile!(3m), tile!(3m),
        tile!(4m), tile!(4m), tile!(4m),
        tile!(6p), tile!(7p), tile!(8p),
        tile!(9s),
    ];
    let r = score(hand, tile!(9s), local_ctx()).expect("三連刻で和了");
    assert!(r.yaku.contains(&Yaku::Sanrenkou), "三連刻: {:?}", r.yaku);
    assert_eq!(r.yakuman_count, 0, "三連刻は役満ではない (2 飜)");
}

#[test]
fn suurenkou_detected() {
    // 222m 333m 444m 555m (連続 4 刻子) + 9m9m、和了 9m。
    let hand = vec![
        tile!(2m), tile!(2m), tile!(2m),
        tile!(3m), tile!(3m), tile!(3m),
        tile!(4m), tile!(4m), tile!(4m),
        tile!(5m), tile!(5m), tile!(5m),
        tile!(9m),
    ];
    let r = score(hand, tile!(9m), local_ctx()).expect("四連刻で和了");
    assert!(r.yaku.contains(&Yaku::Suurenkou), "四連刻: {:?}", r.yaku);
    // この手は 4 暗刻の単騎和了 (9m9m 雀頭待ち) でもあるため、四暗刻単騎 (ダブル役満) +
    // 四連刻 (単役満) = 3 倍役満になる (#147 OSS対比で四暗刻単騎ダブルを実装)。
    assert!(r.yaku.contains(&Yaku::Suuankou), "4 暗刻なので四暗刻も成立: {:?}", r.yaku);
    assert_eq!(r.yakuman_count, 3, "四暗刻単騎(ダブル) + 四連刻 = 3 倍役満");
    // 役満成立時は三連刻 (通常役) はチェックされない (役満ブロックで return)
    assert!(!r.yaku.contains(&Yaku::Sanrenkou));
}

// ============ 人和 (ctx フラグ) ============

#[test]
fn renhou_detected_via_flag() {
    // 通常の和了形 + is_renhou フラグ → 人和役満。
    let hand = vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(5m), tile!(6m), tile!(7m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(6s), tile!(7s),
        tile!(9s), tile!(9s),
    ];
    let ctx = ScoringContext {
        is_tsumo: false,
        allow_local_yakuman: true,
        is_renhou: true,
        ..ScoringContext::default()
    };
    let r = score(hand, tile!(8s), ctx).expect("人和で和了");
    assert!(r.yaku.contains(&Yaku::Renhou), "人和: {:?}", r.yaku);
    assert_eq!(r.yakuman_count, 1);
}
