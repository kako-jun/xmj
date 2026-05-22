//! 役満 / 大物手の ScoringEngine 経路を検証する統合テスト。
//!
//! Issue #42 (国士 13 面 / 九蓮 9 面 ダブル役満) /
//! #51 (天和 / 地和) /
//! #52 (大四喜 / 小四喜 / 四槓子 / 三槓子 / 混老頭)
//! を `ScoringEngine::calculate_score_with_context` で直接組み立てて、
//! `ScoringResult.yakuman_count` と `yaku` が正しいことを assert する。
//!
//! `ScenarioRunner::try_tsumo` は `extract_agari_with_context` で「最高得点解釈」を
//! 選ぶので、複数解釈がある手 (例: 国士 13 面 / 純正九蓮 / 大四喜) でも自動で
//! ダブル役満解釈が選ばれる。**天和 / 地和は `ScoringContext` 側のフラグ判定**
//! のため、`ScenarioRunner` 経由で `Game::build_scoring_context` を踏むテストが
//! 必要 (シナリオ初手の状態で try_tsumo)。

use xmj_core::hand::{Hand, Meld, MeldType};
use xmj_core::scenario::{Scenario, ScenarioRunner};
use xmj_core::scoring::{ScoringContext, ScoringEngine, ScoringResult, Yaku};
use xmj_core::tile;
use xmj_core::tile::Tile;

// =============================================================================
// ヘルパ
// =============================================================================

/// 14 枚 (tiles 13 枚 + winning_tile 1 枚) を ScoringEngine に渡して結果を取る。
/// `is_tsumo=true`, `is_dealer=false` のデフォルト、コンテキストは Default。
fn score_default(tiles: Vec<Tile>, winning_tile: Tile) -> Option<ScoringResult> {
    let mut hand = Hand::new();
    for t in tiles {
        hand.add_tile(t);
    }
    let ctx = ScoringContext::default();
    ScoringEngine::calculate_score_with_context(&hand, &winning_tile, &ctx)
}

/// 副露付き手牌を組む。`melds` を hand に追加した後、残りの 13-3*melds 枚に和了牌
/// を加える。和了牌は呼び出し側で別に渡す。
fn score_with_melds(
    tiles: Vec<Tile>,
    melds: Vec<Meld>,
    winning_tile: Tile,
) -> Option<ScoringResult> {
    let mut hand = Hand::new();
    for t in tiles {
        hand.add_tile(t);
    }
    for m in melds {
        // add_meld は内部で hand から該当牌を抜こうとするが、tiles に入れていないので
        // remove は失敗するだけ (no-op)。push されるだけで OK。
        hand.add_meld(m);
    }
    let ctx = ScoringContext::default();
    ScoringEngine::calculate_score_with_context(&hand, &winning_tile, &ctx)
}

fn kan_meld(tile: Tile, is_open: bool) -> Meld {
    Meld {
        meld_type: MeldType::Kan,
        tiles: vec![tile, tile, tile, tile],
        is_open,
    }
}

fn pon_meld(tile: Tile) -> Meld {
    Meld {
        meld_type: MeldType::Pon,
        tiles: vec![tile, tile, tile],
        is_open: true,
    }
}

// =============================================================================
// #42 国士無双
// =============================================================================

/// 13 面待ち国士: 手牌 13 枚に 13 種么九が 1 枚ずつ、和了牌が么九のいずれか →
/// `yakuman_count == 2` (ダブル役満)。
#[test]
fn test_kokushi_13_mendachi() {
    // 13 種 1 枚ずつ + 9m を和了牌
    let tiles = vec![
        tile!(1m), tile!(9m),
        tile!(1p), tile!(9p),
        tile!(1s), tile!(9s),
        tile!(ton), tile!(nan), tile!(shaa), tile!(pei),
        tile!(haku), tile!(hatsu), tile!(chun),
    ];
    let result = score_default(tiles, tile!(9m)).expect("国士 13 面で和了");
    assert!(
        result.yaku.contains(&Yaku::Kokushi),
        "Kokushi 含む: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 2,
        "13 面待ち国士はダブル役満 (yakuman_count=2): {}",
        result.yakuman_count
    );
}

/// 通常国士: 13 種揃わず 1 種が 2 枚 → `yakuman_count == 1` (単役満)。
#[test]
fn test_kokushi_single_yakuman() {
    // 1m が 2 枚、hatsu 欠落の手で、hatsu を和了牌にする (通常国士)
    let tiles = vec![
        tile!(1m), tile!(1m), tile!(9m),
        tile!(1p), tile!(9p),
        tile!(1s), tile!(9s),
        tile!(ton), tile!(nan), tile!(shaa), tile!(pei),
        tile!(haku), tile!(chun),
    ];
    let result = score_default(tiles, tile!(hatsu)).expect("通常国士で和了");
    assert!(
        result.yaku.contains(&Yaku::Kokushi),
        "Kokushi 含む: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 1,
        "通常国士は単役満 (yakuman_count=1): {}",
        result.yakuman_count
    );
}

// =============================================================================
// #42 九蓮宝燈
// =============================================================================

/// 純正九蓮: 1112345678999 (同色) + 同色任意で和了 → `yakuman_count == 2`。
#[test]
fn test_chuuren_9_mendachi() {
    // 13 枚: 1m*3, 2m, 3m, 4m, 5m, 6m, 7m, 8m, 9m*3、和了牌 = 5m (純正)
    let tiles = vec![
        tile!(1m), tile!(1m), tile!(1m),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(5m), tile!(6m), tile!(7m), tile!(8m),
        tile!(9m), tile!(9m), tile!(9m),
    ];
    let result = score_default(tiles, tile!(5m)).expect("純正九蓮で和了");
    assert!(
        result.yaku.contains(&Yaku::Chuuren),
        "Chuuren 含む: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 2,
        "純正九蓮はダブル役満 (yakuman_count=2): {}",
        result.yakuman_count
    );
}

/// 通常九蓮: 1112345678999 でない九蓮 → `yakuman_count == 1`。
///
/// 例: 1m*3, 2m, 3m, 4m, 5m, 6m, 7m, 8m, 9m*2 (13 枚) + 9m 和了 で
/// 和了形は 1112345678999 になるが、和了前の手牌が「1112345678999」では
/// ないので純正 9 面待ちにはならない。
#[test]
fn test_chuuren_normal() {
    let tiles = vec![
        tile!(1m), tile!(1m), tile!(1m),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(5m), tile!(6m), tile!(7m), tile!(8m),
        tile!(9m), tile!(9m),
        tile!(5m),
    ];
    let result = score_default(tiles, tile!(9m)).expect("通常九蓮で和了");
    assert!(
        result.yaku.contains(&Yaku::Chuuren),
        "Chuuren 含む: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 1,
        "通常九蓮は単役満 (yakuman_count=1): {}",
        result.yakuman_count
    );
}

// =============================================================================
// #51 天和 / 地和
// =============================================================================

/// 天和: 親が配牌時点でツモ和了 → Yaku::Tenhou + yakuman_count == 1。
///
/// Scenario で 14 枚の和了形を親に仕込み、誰も discard していない / 鳴いていない /
/// 山牌から誰もツモっていない状態で `try_tsumo` を呼ぶ。
/// `Game::build_scoring_context` が `is_tenhou=true` をセットする。
#[test]
fn test_tenhou() {
    let mut s = Scenario::default();
    s.dealer = 0;
    // 14 枚和了形: 234m + 234p + 234s + 567s + 8m8m (雀頭)
    s.hands[0] = Some(vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s), tile!(7s),
        tile!(8m), tile!(8m),
    ]);
    let mut r = ScenarioRunner::from_scenario(s);

    // 初期状態: any_call_made_this_round=false, draws_this_round=0, p0 の discards 空
    let result = r.try_tsumo().expect("親配牌時和了");
    assert!(
        result.yaku.contains(&Yaku::Tenhou),
        "Tenhou が含まれる: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 1,
        "天和は単役満: {}",
        result.yakuman_count
    );
}

/// 地和: 子の第一ツモで和了 → Yaku::Chiihou + yakuman_count == 1。
///
/// `Game::current_player = dealer` がデフォルトなので、子和了をテストするには
/// `current_player` と `dealer` を別席にする。dealer=0, current_player=1 (子=南) で
/// 第一ツモ済み (= draws_this_round=1 = seat_offset) を模す。
#[test]
fn test_chiihou() {
    let mut s = Scenario::default();
    s.dealer = 0;
    // 14 枚和了形を p1 に仕込む
    s.hands[1] = Some(vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s), tile!(7s),
        tile!(8m), tile!(8m),
    ]);
    let mut r = ScenarioRunner::from_scenario(s);
    // p1 が「第 1 ツモ済み」状態を作る。
    // Game::build_scoring_context は p1 (seat_offset=1) の chiihou 条件として
    //   draws_this_round == 1 を要求する。
    r.game.current_player = 1;
    r.game.draws_this_round = 1;

    let result = r.try_tsumo().expect("子第一ツモ和了");
    assert!(
        result.yaku.contains(&Yaku::Chiihou),
        "Chiihou が含まれる: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 1,
        "地和は単役満: {}",
        result.yakuman_count
    );
}

/// 地和は鳴き発生 (any_call_made_this_round=true) で不成立。
#[test]
fn test_chiihou_blocked_by_call() {
    let mut s = Scenario::default();
    s.dealer = 0;
    s.hands[1] = Some(vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s), tile!(7s),
        tile!(8m), tile!(8m),
    ]);
    let mut r = ScenarioRunner::from_scenario(s);
    r.game.current_player = 1;
    r.game.draws_this_round = 1;
    // 鳴き発生済み → Chiihou 阻害
    r.game.any_call_made_this_round = true;

    let result = r.try_tsumo().expect("和了形は成立");
    assert!(
        !result.yaku.contains(&Yaku::Chiihou),
        "鳴き発生済みなので Chiihou は付かない: {:?}",
        result.yaku
    );
}

// =============================================================================
// #52 大四喜 / 小四喜
// =============================================================================

/// 大四喜: 4 種の風牌すべてを刻子 (一部副露 OK) → Yaku::Daisuushii + yakuman_count >= 2。
///
/// 構成: 東ポン (副露) + 南刻 + 西刻 + 北刻 + 5m 雀頭。13 枚相当 + 和了牌で 14。
/// 1 つを副露にして四暗刻が同時成立しないようにする (純粋な大四喜のみで yakuman_count==2)。
#[test]
fn test_daisuushii() {
    // 副露: 東 (ポン)
    // 手牌: 南*3, 西*3, 北*3, 5m (雀頭の片割れ)、和了牌 = 5m
    let melds = vec![pon_meld(tile!(ton))];
    let tiles = vec![
        tile!(nan), tile!(nan), tile!(nan),
        tile!(shaa), tile!(shaa), tile!(shaa),
        tile!(pei), tile!(pei), tile!(pei),
        tile!(5m),
    ];
    let result = score_with_melds(tiles, melds, tile!(5m)).expect("大四喜で和了");
    assert!(
        result.yaku.contains(&Yaku::Daisuushii),
        "Daisuushii 含む: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 2,
        "大四喜単体はダブル役満 (yakuman_count=2): {}",
        result.yakuman_count
    );
}

/// 小四喜: 3 種の風牌刻子 + 1 種の風牌雀頭 → Yaku::Shousuushii + yakuman_count == 1。
///
/// 構成: 東刻 + 南刻 + 西刻 + 234m (順子) + 北*2 (雀頭)。
#[test]
fn test_shousuushii() {
    let tiles = vec![
        tile!(ton), tile!(ton), tile!(ton),
        tile!(nan), tile!(nan), tile!(nan),
        tile!(shaa), tile!(shaa), tile!(shaa),
        tile!(2m), tile!(3m), tile!(4m),
        tile!(pei),
    ];
    let result = score_default(tiles, tile!(pei)).expect("小四喜で和了");
    assert!(
        result.yaku.contains(&Yaku::Shousuushii),
        "Shousuushii 含む: {:?}",
        result.yaku
    );
    assert!(
        !result.yaku.contains(&Yaku::Daisuushii),
        "小四喜のとき大四喜は付かない: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 1,
        "小四喜は単役満: {}",
        result.yakuman_count
    );
}

// =============================================================================
// #52 四槓子 / 三槓子
// =============================================================================

/// 四槓子: 4 つの槓子 + 雀頭 → Yaku::Suukantsu + yakuman_count == 1。
///
/// すべて副露槓子で構成し、残り手牌は雀頭 1 枚 + 和了牌 1 枚 (単騎)。
#[test]
fn test_suukantsu() {
    // 4 つの暗槓 + 5m 雀頭 (5m + 和了 5m で単騎)
    let melds = vec![
        kan_meld(tile!(1m), false),
        kan_meld(tile!(2p), false),
        kan_meld(tile!(3s), false),
        kan_meld(tile!(haku), false),
    ];
    let tiles = vec![tile!(5m)];
    let result = score_with_melds(tiles, melds, tile!(5m)).expect("四槓子で和了");
    assert!(
        result.yaku.contains(&Yaku::Suukantsu),
        "Suukantsu 含む: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 1,
        "四槓子は単役満: {}",
        result.yakuman_count
    );
}

/// 三槓子: 3 つの槓子 → Yaku::Sankantsu + han +2 (役満ではない)。
#[test]
fn test_sankantsu() {
    // 3 つの暗槓 + 残り手牌 4 枚 (1 面子 + 雀頭) + 和了牌
    // 残り: 2m 3m 4m (順子) + 5p 5p (雀頭)、和了牌 = 4m
    let melds = vec![
        kan_meld(tile!(1s), false),
        kan_meld(tile!(2s), false),
        kan_meld(tile!(haku), false),
    ];
    let tiles = vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(5p), tile!(5p),
    ];
    let result = score_with_melds(tiles, melds, tile!(4m))
        .expect("三槓子で和了");
    assert!(
        result.yaku.contains(&Yaku::Sankantsu),
        "Sankantsu 含む: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 0,
        "三槓子は役満ではない: {}",
        result.yakuman_count
    );
}

// =============================================================================
// #52 混老頭
// =============================================================================

/// 混老頭: すべて 1/9/字 + 字牌を含む → Yaku::Honroutou + han +2 (非役満)。
///
/// 構成: 東ポン (副露で四暗刻回避) + 1m*3 + 9p*3 + 中*3 + 9s*2 雀頭。和了牌 = 9s。
/// 全構成牌が 1/9/字で字牌を含むので Honroutou 成立。数牌 1/9 も含むので字一色 (役満) 不成立。
/// 副露ありなので四暗刻 (役満) 不成立。
#[test]
fn test_honroutou() {
    let melds = vec![pon_meld(tile!(ton))];
    let tiles = vec![
        tile!(1m), tile!(1m), tile!(1m),
        tile!(9p), tile!(9p), tile!(9p),
        tile!(chun), tile!(chun), tile!(chun),
        tile!(9s),
    ];
    let result = score_with_melds(tiles, melds, tile!(9s)).expect("混老頭で和了");
    assert!(
        result.yaku.contains(&Yaku::Honroutou),
        "Honroutou 含む: {:?}",
        result.yaku
    );
    assert_eq!(
        result.yakuman_count, 0,
        "混老頭は役満ではない: {}",
        result.yakuman_count
    );
}
