//! #108 監査修正の回帰テスト。
//!
//! 旧実装でスタブ (常に false/None) だった構造役と、スタブだった符計算の正しさを
//! 検証する。これらの役は「役成立を assert するテストが存在しない」ことが #108 監査で
//! 判明したため新規追加した。

use xmj_core::hand::{Hand, Meld, MeldType};
use xmj_core::scoring::{ScoringContext, ScoringEngine, ScoringResult, Yaku};
use xmj_core::tile;
use xmj_core::tile::{Honor, Tile};

/// 13 枚の手牌 + 和了牌を門前ロン (デフォルト ctx) で点数化する。
fn score_menzen(hand_tiles: Vec<Tile>, winning: Tile, ctx: ScoringContext) -> ScoringResult {
    let mut hand = Hand::new();
    for t in hand_tiles {
        hand.add_tile(t);
    }
    ScoringEngine::calculate_score_with_context(&hand, &winning, &ctx)
        .expect("和了形が成立しスコアが返る")
}

fn ron_ctx() -> ScoringContext {
    ScoringContext {
        is_tsumo: false,
        round_wind: Honor::Ton,
        seat_wind: Honor::Nan,
        ..ScoringContext::default()
    }
}

fn tsumo_ctx() -> ScoringContext {
    ScoringContext {
        is_tsumo: true,
        round_wind: Honor::Ton,
        seat_wind: Honor::Nan,
        ..ScoringContext::default()
    }
}

// ============ 一盃口 / 二盃口 ============

#[test]
fn iipeikou_detected() {
    // 234m 234m (一盃口) + 567p + 789s + 1s1s 雀頭、和了 1s 単騎ロン。
    let hand = vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(5p), tile!(6p), tile!(7p),
        tile!(7s), tile!(8s), tile!(9s),
        tile!(1s),
    ];
    let r = score_menzen(hand, tile!(1s), ron_ctx());
    assert!(r.yaku.contains(&Yaku::Iipeikou), "一盃口: {:?}", r.yaku);
    assert!(!r.yaku.contains(&Yaku::Ryanpeikou));
}

#[test]
fn ryanpeikou_beats_chiitoitsu() {
    // 234m 234m 567p 567p + 9s9s。七対子形でもあるが二盃口 (3飜) が優先される。
    let hand = vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(5p), tile!(6p), tile!(7p),
        tile!(5p), tile!(6p), tile!(7p),
        tile!(9s),
    ];
    let r = score_menzen(hand, tile!(9s), ron_ctx());
    assert!(r.yaku.contains(&Yaku::Ryanpeikou), "二盃口: {:?}", r.yaku);
    assert!(!r.yaku.contains(&Yaku::Iipeikou));
    assert!(!r.yaku.contains(&Yaku::Chiitoitsu), "二盃口が七対子より優先");
}

// ============ 三色同順 / 一気通貫 ============

#[test]
fn sanshoku_doujun_detected() {
    // 234m 234p 234s + 567m + 9p9p、和了 9p。
    let hand = vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5m), tile!(6m), tile!(7m),
        tile!(9p),
    ];
    let r = score_menzen(hand, tile!(9p), ron_ctx());
    assert!(r.yaku.contains(&Yaku::SanshokuDoujun), "三色同順: {:?}", r.yaku);
}

#[test]
fn ittsu_detected() {
    // 123m 456m 789m + 234p + 5s5s、和了 5s。
    let hand = vec![
        tile!(1m), tile!(2m), tile!(3m),
        tile!(4m), tile!(5m), tile!(6m),
        tile!(7m), tile!(8m), tile!(9m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(5s),
    ];
    let r = score_menzen(hand, tile!(5s), ron_ctx());
    assert!(r.yaku.contains(&Yaku::Ittsu), "一気通貫: {:?}", r.yaku);
}

// ============ チャンタ / 純チャン ============

#[test]
fn chanta_detected() {
    // 123m 789p 123s + 東東東 + 9m9m、和了 9m。字牌を含むのでチャンタ。
    let hand = vec![
        tile!(1m), tile!(2m), tile!(3m),
        tile!(7p), tile!(8p), tile!(9p),
        tile!(1s), tile!(2s), tile!(3s),
        tile!(ton), tile!(ton), tile!(ton),
        tile!(9m),
    ];
    let r = score_menzen(hand, tile!(9m), ron_ctx());
    assert!(r.yaku.contains(&Yaku::Chanta), "チャンタ: {:?}", r.yaku);
    assert!(!r.yaku.contains(&Yaku::Junchan));
}

#[test]
fn junchan_detected() {
    // 123m 789m 123p 789p + 1s1s、和了 1s。字牌なし純チャン。
    let hand = vec![
        tile!(1m), tile!(2m), tile!(3m),
        tile!(7m), tile!(8m), tile!(9m),
        tile!(1p), tile!(2p), tile!(3p),
        tile!(7p), tile!(8p), tile!(9p),
        tile!(1s),
    ];
    let r = score_menzen(hand, tile!(1s), ron_ctx());
    assert!(r.yaku.contains(&Yaku::Junchan), "純チャン: {:?}", r.yaku);
    assert!(!r.yaku.contains(&Yaku::Chanta));
}

// ============ 三色同刻 / 小三元 ============

#[test]
fn sanshoku_doukou_detected() {
    // 222m 222p 222s + 567s + 9p9p、和了 9p。
    let hand = vec![
        tile!(2m), tile!(2m), tile!(2m),
        tile!(2p), tile!(2p), tile!(2p),
        tile!(2s), tile!(2s), tile!(2s),
        tile!(5s), tile!(6s), tile!(7s),
        tile!(9p),
    ];
    let r = score_menzen(hand, tile!(9p), ron_ctx());
    assert!(r.yaku.contains(&Yaku::SanshokuDoukou), "三色同刻: {:?}", r.yaku);
}

#[test]
fn shousangen_detected() {
    // 白白白 發發發 中中 + 234m + 567p、和了 中 (タンキ)。
    let hand = vec![
        tile!(haku), tile!(haku), tile!(haku),
        tile!(hatsu), tile!(hatsu), tile!(hatsu),
        tile!(chun), tile!(chun),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(5p), tile!(6p), // 13 枚にするため 567p のうち 5p6p
    ];
    let r = score_menzen(hand, tile!(7p), ron_ctx());
    assert!(r.yaku.contains(&Yaku::Shousangen), "小三元: {:?}", r.yaku);
    // 白・發 の役牌 2 つも付く
    assert!(r.yaku.contains(&Yaku::Yakuhai(Honor::Haku)));
    assert!(r.yaku.contains(&Yaku::Yakuhai(Honor::Hatsu)));
}

// ============ 三暗刻 (手牌内暗刻) + ロン格下げ ============

#[test]
fn sanankou_hand_internal_tsumo() {
    // 111m 222p 333s (3 暗刻) + 456m + 9s9s、ツモ 9s。
    let hand = vec![
        tile!(1m), tile!(1m), tile!(1m),
        tile!(2p), tile!(2p), tile!(2p),
        tile!(3s), tile!(3s), tile!(3s),
        tile!(4m), tile!(5m), tile!(6m),
        tile!(9s),
    ];
    let r = score_menzen(hand, tile!(9s), tsumo_ctx());
    assert!(r.yaku.contains(&Yaku::Sanankou), "三暗刻 (手牌内暗刻): {:?}", r.yaku);
}

#[test]
fn sanankou_ron_shanpon_downgrades() {
    // 發發發(暗刻・役牌) 111m(暗刻) + 33s (シャンポン) + 4m4m + 567s。
    // 和了 3s で 333s 完成。役牌發で常に点数は成立する。
    // ロンなら 333s は明刻 → 暗刻 2 つ (發・1m) で三暗刻不成立。
    // ツモなら 333s 暗刻 → 暗刻 3 つで三暗刻成立。
    let hand = vec![
        tile!(hatsu), tile!(hatsu), tile!(hatsu),
        tile!(1m), tile!(1m), tile!(1m),
        tile!(3s), tile!(3s),
        tile!(4m), tile!(4m),
        tile!(5s), tile!(6s), tile!(7s),
    ];
    let r_ron = score_menzen(hand.clone(), tile!(3s), ron_ctx());
    assert!(
        !r_ron.yaku.contains(&Yaku::Sanankou),
        "ロンシャンポンは当たり刻子が明刻 → 三暗刻不成立: {:?}",
        r_ron.yaku
    );
    let r_tsumo = score_menzen(hand, tile!(3s), tsumo_ctx());
    assert!(
        r_tsumo.yaku.contains(&Yaku::Sanankou),
        "ツモなら 333s 暗刻 → 三暗刻成立: {:?}",
        r_tsumo.yaku
    );
}

// ============ 平和の風牌雀頭除外 (#5) ============

#[test]
fn pinfu_with_guest_pair() {
    // 全順子 + 5m5m (非役牌) 雀頭 + 両面待ち → 平和成立。
    // 2p3p4p 5p6p7p 2s3s4s 6s7s + 5m5m、和了 8s (両面)。
    let hand = vec![
        tile!(2p), tile!(3p), tile!(4p),
        tile!(5p), tile!(6p), tile!(7p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(6s), tile!(7s),
        tile!(5m), tile!(5m),
    ];
    let r = score_menzen(hand, tile!(8s), ron_ctx());
    assert!(r.yaku.contains(&Yaku::Pinfu), "非役牌雀頭は平和成立: {:?}", r.yaku);
    assert_eq!(r.fu, 30, "平和ロン = 30 符");
}

#[test]
fn pinfu_rejected_with_round_wind_pair() {
    // 同じ全順子形だが雀頭が場風 (東) → 平和不成立。
    // 平和を弾くと役なしになり得るのでツモ (門前ツモ役) で点数を成立させ、平和不在を確認する。
    let hand = vec![
        tile!(2p), tile!(3p), tile!(4p),
        tile!(5p), tile!(6p), tile!(7p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(6s), tile!(7s),
        tile!(ton), tile!(ton),
    ];
    // round_wind = Ton。東は場風役牌なので雀頭にすると平和不成立。
    let r = score_menzen(hand, tile!(8s), tsumo_ctx());
    assert!(
        !r.yaku.contains(&Yaku::Pinfu),
        "場風雀頭は平和不成立: {:?}",
        r.yaku
    );
}

// ============ 符計算 (#3) ============

#[test]
fn fu_pinfu_tsumo_is_20() {
    let hand = vec![
        tile!(2p), tile!(3p), tile!(4p),
        tile!(5p), tile!(6p), tile!(7p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(6s), tile!(7s),
        tile!(5m), tile!(5m),
    ];
    let r = score_menzen(hand, tile!(8s), tsumo_ctx());
    assert!(r.yaku.contains(&Yaku::Pinfu));
    assert_eq!(r.fu, 20, "平和ツモ = 20 符");
}

// ============ 喰いタン toggle (#129) ============

#[test]
fn open_tanyao_toggle() {
    // チー 345m + 2p3p4p 5p6p7p 6s7s8s 2s2s、和了 8s。全牌 2-8 → 喰いタン形。
    let make = || {
        let mut hand = Hand::new();
        for t in [
            tile!(2p), tile!(3p), tile!(4p),
            tile!(5p), tile!(6p), tile!(7p),
            tile!(6s), tile!(7s),
            tile!(2s), tile!(2s),
        ] {
            hand.add_tile(t);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Chi,
            tiles: vec![tile!(3m), tile!(4m), tile!(5m)],
            is_open: true,
            ..Default::default()
        });
        hand
    };

    // allow_open_tanyao = true (デフォルト) → 喰いタン成立
    let ctx_on = ScoringContext {
        allow_open_tanyao: true,
        ..ScoringContext::default()
    };
    let r_on = ScoringEngine::calculate_score_with_context(&make(), &tile!(8s), &ctx_on)
        .expect("喰いタンありなら和了");
    assert!(r_on.yaku.contains(&Yaku::Tanyao), "喰いタン有効: {:?}", r_on.yaku);

    // allow_open_tanyao = false → 非門前タンヤオは付かず役なし → None
    let ctx_off = ScoringContext {
        allow_open_tanyao: false,
        ..ScoringContext::default()
    };
    let r_off = ScoringEngine::calculate_score_with_context(&make(), &tile!(8s), &ctx_off);
    assert!(
        r_off.is_none(),
        "喰いタン無効なら役なしで和了不可: {:?}",
        r_off.map(|r| r.yaku)
    );
}

// ============ 暗槓は門前を崩さない (#108 is_menzen 修正) ============

#[test]
fn ankan_keeps_menzen_tsumo() {
    // 暗槓 5555s + 234m 234p 678m + 9p9p、ツモ 9p。
    // 暗槓は門前を保つので門前ツモが付く (旧実装では非門前扱いで役が消えていた)。
    let mut hand = Hand::new();
    for t in [
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(6m), tile!(7m), tile!(8m),
        tile!(9p),
    ] {
        hand.add_tile(t);
    }
    hand.add_meld(Meld {
        meld_type: MeldType::Kan,
        tiles: vec![tile!(5s), tile!(5s), tile!(5s), tile!(5s)],
        is_open: false, // 暗槓
        ..Default::default()
    });
    let r = ScoringEngine::calculate_score_with_context(&hand, &tile!(9p), &tsumo_ctx())
        .expect("暗槓ありでも門前ツモで和了");
    assert!(
        r.yaku.contains(&Yaku::Tsumo),
        "暗槓は門前を崩さないので門前ツモが付く: {:?}",
        r.yaku
    );
}

#[test]
fn fu_sanankou_terminal_anko_tanki_tsumo() {
    // 111m(么九暗刻) 222p(中張暗刻) 333s(中張暗刻) + 456m + 9s9s 単騎、ツモ 9s。
    // 符: 基本 20 + ツモ 2 + 111m(暗刻么九 8) + 222p(暗刻中張 4) + 333s(暗刻中張 4)
    //     + 単騎待ち 2 = 40 符。
    let hand = vec![
        tile!(1m), tile!(1m), tile!(1m),
        tile!(2p), tile!(2p), tile!(2p),
        tile!(3s), tile!(3s), tile!(3s),
        tile!(4m), tile!(5m), tile!(6m),
        tile!(9s),
    ];
    let r = score_menzen(hand, tile!(9s), tsumo_ctx());
    assert!(r.yaku.contains(&Yaku::Sanankou));
    assert_eq!(r.fu, 40, "20+2+8+4+4+2 = 40 符: {:?}", r.yaku);
}
