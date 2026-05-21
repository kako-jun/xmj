//! 状況役・立直系・場風自風・ドラの ScoringContext 経路を検証する統合テスト。
//!
//! Issue #49 (立直/一発/ダブル立直) / #50 (海底/河底/嶺上/槍槓) /
//! #53 (場風/自風 yakuhai + 暗刻三元) / #54 (ドラ/裏ドラ/槓ドラ/赤ドラ)
//! を ScenarioRunner で組み立てて、`ScoringResult` に正しく反映されることを assert する。
//!
//! 本テストは `cargo test` で動く (feature = "wasm" 不要)。

use xmj_core::game::{Game, GameMode, Length};
use xmj_core::hand::Hand;
use xmj_core::scenario::{Scenario, ScenarioRunner};
use xmj_core::scoring::{dora_indicator_to_dora, ScoringContext, ScoringEngine, Yaku};
use xmj_core::tile;
use xmj_core::tile::{Honor, Suit, Tile};

/// p0 13 枚テンパイ手 (タンヤオ平和形・4s / 7s 両面待ち) を仕込む。
/// 234m / 234p / 234s / 56s / 88m 雀頭 → 4s/7s で和了。
/// shanten_normal が pair 1 つ要求するので 88m 雀頭にしてある。
fn tenpai_hand_4s_7s_wait() -> Vec<Tile> {
    vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s),
        tile!(8m), tile!(8m),
    ]
}

/// 立直宣言 → 自家ツモ和了で Yaku::Riichi + Yaku::Tsumo (門前ツモ) が成立する。
#[test]
fn test_yaku_riichi() {
    let mut s = Scenario::default();
    s.length = Length::Hanchan;
    s.dealer = 0;
    s.hands[0] = Some(tenpai_hand_4s_7s_wait());
    // 次にツモる牌 = 7s (山牌は末尾から pop されるので末尾に置く)
    s.wall = vec![tile!(7s)];
    let mut r = ScenarioRunner::from_scenario(s);

    // ダブル立直回避のため、他家に 1 枚 discard させて第一巡を抜ける。
    // p1 の Discard を直接挿入する (Game の進行は変えない)。
    use xmj_core::player::Discard;
    r.game.players[1].discards.push(Discard {
        tile: tile!(1m),
        is_hidden: false,
    });

    // 親はテンパイ + 門前 + 1000 点以上で立直可能
    assert!(r.declare_riichi(), "親はテンパイ門前なので立直可能");
    assert!(!r.game.players[0].double_riichi, "他家 discard 済なので Double にはならない");

    // 立直後にツモ
    let drawn = r.draw().expect("wall に 1 枚あるのでツモる");
    assert_eq!(drawn, tile!(7s));

    let result = r.try_tsumo().expect("7s ツモで和了");
    assert!(result.yaku.contains(&Yaku::Riichi), "Riichi が含まれる: {:?}", result.yaku);
    assert!(result.yaku.contains(&Yaku::Tsumo), "門前ツモが含まれる: {:?}", result.yaku);
}

/// 立直宣言直後の自家ツモ和了で Yaku::Ippatsu が成立する。
#[test]
fn test_yaku_ippatsu() {
    let mut s = Scenario::default();
    s.dealer = 0;
    s.hands[0] = Some(tenpai_hand_4s_7s_wait());
    s.wall = vec![tile!(7s)];
    let mut r = ScenarioRunner::from_scenario(s);

    assert!(r.declare_riichi());
    // 立直宣言時点で `Player::declare_riichi` が `ippatsu=true` を立てている。
    assert!(r.game.players[0].ippatsu);

    let _ = r.draw().expect("7s をツモ");
    let result = r.try_tsumo().expect("和了");
    assert!(result.yaku.contains(&Yaku::Ippatsu), "Ippatsu が含まれる: {:?}", result.yaku);
}

/// ダブル立直: 第一巡 (誰も捨てていない、誰も鳴いていない) での立直宣言。
#[test]
fn test_yaku_double_riichi() {
    let mut s = Scenario::default();
    s.dealer = 0;
    s.hands[0] = Some(tenpai_hand_4s_7s_wait());
    s.wall = vec![tile!(7s)];
    let mut r = ScenarioRunner::from_scenario(s);

    // 第一巡: 全員 discard 0 で、誰も鳴いていない状態で立直宣言
    assert!(r.declare_riichi());
    assert!(r.game.players[0].double_riichi, "第一巡立直なのでダブル立直成立");

    let _ = r.draw().expect("7s をツモ");
    let result = r.try_tsumo().expect("和了");
    assert!(
        result.yaku.contains(&Yaku::DoubleRiichi),
        "DoubleRiichi が含まれる: {:?}",
        result.yaku
    );
    assert!(
        !result.yaku.contains(&Yaku::Riichi),
        "DoubleRiichi 成立時は Riichi を二重 push しない: {:?}",
        result.yaku
    );
}

/// 海底摸月: 山残り 0 のツモで Yaku::Haitei 成立。
#[test]
fn test_yaku_haitei() {
    let mut s = Scenario::default();
    s.dealer = 0;
    s.hands[0] = Some(tenpai_hand_4s_7s_wait());
    // 山牌を 1 枚だけにする → draw 後 wall.is_empty() で is_last_draw=true
    s.wall = vec![tile!(7s)];
    let mut r = ScenarioRunner::from_scenario(s);

    let _ = r.draw().expect("最後の 1 枚をツモ");
    assert!(r.game.is_last_draw, "draw 後に is_last_draw=true になる");

    let result = r.try_tsumo().expect("和了");
    assert!(
        result.yaku.contains(&Yaku::Haitei),
        "Haitei が含まれる: {:?}",
        result.yaku
    );
}

/// 河底撈魚: 山残り 0 の打牌に対するロンで Yaku::Houtei 成立。
#[test]
fn test_yaku_houtei() {
    let mut s = Scenario::default();
    s.dealer = 0;
    // p0 (親) は 14 枚スタート: 7s を打牌できる手にする
    s.hands[0] = Some(vec![
        tile!(1m), tile!(1m), tile!(1m),
        tile!(2m), tile!(5p), tile!(6p),
        tile!(7p), tile!(8p), tile!(9p),
        tile!(1s), tile!(9s), tile!(ton),
        tile!(nan), tile!(7s),
    ]);
    // p1 は 13 枚テンパイ (4s/7s 待ち)
    s.hands[1] = Some(tenpai_hand_4s_7s_wait());
    let mut r = ScenarioRunner::from_scenario(s);
    // Scenario::build は wall=[] では override しないため、build 後に直接 clear する。
    r.game.wall.clear();

    // 親が 7s を打牌 (山牌 0 → is_last_discard=true)
    assert!(r.discard(tile!(7s)));
    assert!(r.game.is_last_discard, "wall 空時の打牌は is_last_discard=true");

    let result = r.try_ron(1).expect("p1 が 7s でロン");
    assert!(
        result.yaku.contains(&Yaku::Houtei),
        "Houtei が含まれる: {:?}",
        result.yaku
    );
}

/// 嶺上開花: do_ankan 後の嶺上ツモで Yaku::Rinshan 成立。
///
/// 構成: 手牌 14 枚 (234m / 234p / 234s / 5p / 8m x4) + 暗カン 8m → 嶺上 5p で和了形
/// 234m / 234p / 234s / 5p5p (雀頭) + 暗カン 8m。
///
/// do_ankan は `self.wall.pop()` を 2 回呼ぶ (1 回目=槓ドラ、2 回目=嶺上ツモ)。
/// Vec::pop は最末尾を返すため、最末尾=槓ドラ、その手前=嶺上ツモ となる。
/// よって `wall = vec![<嶺上ツモ>, <槓ドラ>]` の並びにする。
#[test]
fn test_yaku_rinshan() {
    let mut game = Game::new_with_mode_and_length(
        vec!["P0".to_string(), "P1".to_string(), "P2".to_string(), "P3".to_string()],
        GameMode::Standard,
        Length::Hanchan,
    );
    // 山牌の末尾 2 枚: pop1=槓ドラ, pop2=嶺上ツモ
    game.wall = vec![tile!(5p), tile!(1m)];

    // p0 手牌 = 234m 234p 234s 5p (10 枚) + 8m x4 (暗カン用に手牌内に持つ)
    let mut hand = Hand::new();
    for t in [
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5p),
        tile!(8m), tile!(8m), tile!(8m), tile!(8m),
    ] {
        hand.add_tile(t);
    }
    game.players[0].hand = hand;
    game.current_player = 0;
    game.dealer = 0;

    // 暗槓 8m を実行 → wall から 2 枚 pop され、嶺上ツモが手牌に入る。
    // last_was_rinshan=true になる。
    assert!(game.do_ankan(0, tile!(8m)), "暗カン 8m 成立");
    assert!(game.last_was_rinshan, "do_ankan 後に last_was_rinshan=true");

    // 嶺上ツモで 5p が入って手牌完成: 234m / 234p / 234s / 5p5p + 暗カン 8m
    let hand_clone = game.players[0].hand.clone();
    use xmj_core::agari_extract::extract_agari_with_context;
    let (sub_hand, winning_tile) =
        extract_agari_with_context(&hand_clone, true, true).expect("和了形");
    let ctx = game.build_scoring_context(0, true);
    let result = ScoringEngine::calculate_score_with_context(&sub_hand, &winning_tile, &ctx)
        .expect("和了点数計算成功");
    assert!(
        result.yaku.contains(&Yaku::Rinshan),
        "Rinshan が含まれる: {:?}",
        result.yaku
    );
}

/// 場風 + 自風が両方東 (round=1, player=dealer) なら Yakuhai(Ton) が 2 回 push される。
#[test]
fn test_yaku_yakuhai_round_wind() {
    // round=1 (東場) で dealer=0 (東家)。p0 が東の刻子で和了 → 場風+自風 = Yakuhai 2 回。
    let mut s = Scenario::default();
    s.dealer = 0;
    s.round = 1;
    // 14 枚和了形: 東東東 + 234m + 234p + 234s + 55s
    s.hands[0] = Some(vec![
        tile!(ton), tile!(ton), tile!(ton),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(5s),
    ]);
    let mut r = ScenarioRunner::from_scenario(s);

    let result = r.try_tsumo().expect("和了");
    // 場風 + 自風 が両方 East → Yakuhai(East) が 2 回 push される
    let east_count = result
        .yaku
        .iter()
        .filter(|y| matches!(y, Yaku::Yakuhai(Honor::Ton)))
        .count();
    assert_eq!(east_count, 2, "場風東 + 自風東で Yakuhai が 2 回 push される: {:?}", result.yaku);
}

/// 手牌内に白の暗刻があるとき Yaku::Yakuhai(Haku) が含まれる (#53)。
#[test]
fn test_yaku_yakuhai_ankou_sangenpai() {
    // 14 枚和了形: 白白白 + 234m + 234p + 234s + 55s
    let mut s = Scenario::default();
    s.dealer = 0;
    s.hands[0] = Some(vec![
        tile!(haku), tile!(haku), tile!(haku),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(5s),
    ]);
    let mut r = ScenarioRunner::from_scenario(s);

    let result = r.try_tsumo().expect("和了");
    assert!(
        result.yaku.contains(&Yaku::Yakuhai(Honor::Haku)),
        "手牌内白暗刻で Yakuhai(Haku) が成立: {:?}",
        result.yaku
    );
}

/// ドラ表示 1m → ドラは 2m。手牌に 2m を 2 枚含む形で dora=2、han が +2 増える。
#[test]
fn test_yaku_dora() {
    let mut s = Scenario::default();
    s.dealer = 0;
    // 構成: 2m2m (雀頭) + 234p + 234s + 567s + 678m (順子)
    s.hands[0] = Some(vec![
        tile!(2m), tile!(2m),
        tile!(6m), tile!(7m), tile!(8m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s), tile!(7s),
    ]);
    // ドラ表示牌 = 1m → ドラは 2m
    s.dora_indicators = vec![tile!(1m)];
    let mut r = ScenarioRunner::from_scenario(s);

    // ドラ表示牌から実際のドラを算出 (sanity check)
    assert_eq!(dora_indicator_to_dora(&tile!(1m)), tile!(2m));

    let result = r.try_tsumo().expect("和了");
    assert_eq!(result.dora, 2, "2m を 2 枚保持しているので dora=2: yaku={:?}", result.yaku);
    // han にも dora ぶんが加算されているはず
    assert!(result.han >= 2, "ドラ 2 枚分が han に加算: han={}", result.han);
}

/// 赤 5m を 1 枚含む和了 → akadora=1、han +1。
#[test]
fn test_yaku_akadora() {
    let mut s = Scenario::default();
    s.dealer = 0;
    // 14 枚和了形: 234m + 234p + 234s + 567s + 5m5m (雀頭)。1 枚目の 5m を赤にする。
    let red_5m = Tile::new_number(Suit::Man, 5, true);
    s.hands[0] = Some(vec![
        tile!(2m), tile!(3m), tile!(4m),
        red_5m, tile!(5m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s), tile!(7s),
    ]);
    let mut r = ScenarioRunner::from_scenario(s);

    let result = r.try_tsumo().expect("和了");
    assert_eq!(result.akadora, 1, "赤 5m を 1 枚保持: akadora={}", result.akadora);
}

/// dora_indicator_to_dora のループ規則 (sanity check)
#[test]
fn test_dora_indicator_loops() {
    // 数牌 9 → 1 にループ
    assert_eq!(dora_indicator_to_dora(&tile!(9m)), tile!(1m));
    assert_eq!(dora_indicator_to_dora(&tile!(9p)), tile!(1p));
    assert_eq!(dora_indicator_to_dora(&tile!(9s)), tile!(1s));
    // 風牌: 東→南→西→北→東
    assert_eq!(dora_indicator_to_dora(&tile!(ton)), tile!(nan));
    assert_eq!(dora_indicator_to_dora(&tile!(nan)), tile!(shaa));
    assert_eq!(dora_indicator_to_dora(&tile!(shaa)), tile!(pei));
    assert_eq!(dora_indicator_to_dora(&tile!(pei)), tile!(ton));
    // 三元: 白→發→中→白
    assert_eq!(dora_indicator_to_dora(&tile!(haku)), tile!(hatsu));
    assert_eq!(dora_indicator_to_dora(&tile!(hatsu)), tile!(chun));
    assert_eq!(dora_indicator_to_dora(&tile!(chun)), tile!(haku));
}

/// ScoringContext::default() でも既存ロジックと同じ結果になることを sanity check する。
/// 旧 API `calculate_score(hand, tile, is_tsumo, is_dealer)` は新 API のラッパなので、
/// 同じ和了形に対して同じ得点 (役・han) が返るはず。
#[test]
fn test_legacy_api_compat() {
    // タンヤオのみの簡易ロン和了形を作って旧 API と新 API を比較
    let mut hand = Hand::new();
    for t in [
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s), tile!(7s),
        tile!(8m),
    ] {
        hand.add_tile(t);
    }
    let win = tile!(8m);

    let legacy = ScoringEngine::calculate_score(&hand, &win, false, false).expect("和了");
    let ctx = ScoringContext::default();
    let new = ScoringEngine::calculate_score_with_context(&hand, &win, &ctx).expect("和了");

    assert_eq!(legacy.han, new.han, "旧 API と新 API で han 一致");
    assert_eq!(legacy.fu, new.fu, "旧 API と新 API で fu 一致");
    assert_eq!(legacy.total_points, new.total_points, "旧 API と新 API で total_points 一致");
}
