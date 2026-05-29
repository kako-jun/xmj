//! シナリオテスト基盤 (Issue #66) の動作確認用統合テスト。
//!
//! 本テストの目的は「Scenario / ScenarioRunner / `tile!` マクロが
//! 仕様通りに局面を組み立て、`draw` / `discard` / `try_tsumo` / `try_ron` /
//! `availability` が一通り動く」ことだけを保証する。
//!
//! 役判定そのもの (例: 天和=#51 / 海底=#50) は別 Issue で実装するため、
//! 本テストでは **「ScoringResult が返ってくる / yaku 一覧が取得できる」までを
//! assert** し、特定の役が含まれるかは TODO コメントだけ残す。

use xmj_core::game::Length;
use xmj_core::scenario::{Scenario, ScenarioRunner};
use xmj_core::scoring::Yaku;
use xmj_core::tile;
use xmj_core::tile::{Honor, Suit, Tile};

/// 親 (dealer=0) に「タンヤオ + 平和」が即成立する 14 枚を仕込み、
/// `try_tsumo` で `ScoringResult` が返ることを確認する。
///
/// この手は門前ツモ・タンヤオ・平和あたりが成立するはず。
/// 「特定の役名が含まれる」までは個別役 Issue で締めるので、本テストでは
/// `result.han >= 1` と `result.total_points > 0` までを assert する。
///
/// TODO(#51): 天和パスの検証は #51 で `Yaku::Tenhou` 検出が入ったら追加する。
#[test]
fn try_tsumo_returns_scoring_result_for_clear_winning_hand() {
    let mut s = Scenario::default();
    s.length = Length::Hanchan;
    s.dealer = 0;
    // 14 枚和了形: 234m / 234p / 234s / 567s / 88m (タンヤオ + 平和形)
    s.hands[0] = Some(vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s), tile!(7s),
        tile!(8m), tile!(8m),
    ]);
    let mut r = ScenarioRunner::from_scenario(s);

    // current_player は dealer と同じになる。
    assert_eq!(r.game.current_player, 0);
    let av = r.availability();
    assert!(av.can_tsumo, "14 枚和了形なら can_tsumo は true");

    let result = r.try_tsumo().expect("和了形 14 枚なので tsumo 成立");
    assert!(result.han >= 1, "han が 1 以上計算される (タンヤオ等)");
    assert!(result.total_points > 0, "親ツモ和了なので得点 > 0");
    assert!(
        !result.yaku.is_empty(),
        "役一覧が返ってくる (中身は別 Issue で詳細チェック)"
    );

    // resolve_win 経由で `last_outcome` に Win が積まれる
    assert!(r.game.last_outcome.is_some());
}

/// `try_ron` 経由でロン和了が成立する経路を確認する。
///
/// シナリオ:
///   - p1 (子・南家) のテンパイ 13 枚 (タンヤオ平和待ち)
///   - p0 (親・東家) が当たり牌を打牌する
///   - p1 が `try_ron(1)` で和了 → ScoringResult が返る
#[test]
fn try_ron_returns_scoring_result_when_player_can_win() {
    let mut s = Scenario::default();
    s.dealer = 0;
    // p1 は 13 枚テンパイ: 234m / 234p / 234s / 567s / 88m から 1 枚抜く形にする。
    // 8m を 1 枚抜いて「8m 待ち」(単騎 8m / シャンポン 8m) のテンパイにしておく。
    s.hands[1] = Some(vec![
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s), tile!(7s),
        tile!(8m),
    ]);
    // p0 は実体無しでよいが、打牌できる手牌を持たせる
    s.hands[0] = Some(vec![
        tile!(1m), tile!(1m), tile!(1m),
        tile!(2m), tile!(5p), tile!(6p),
        tile!(7p), tile!(8p), tile!(9p),
        tile!(1s), tile!(9s), tile!(ton),
        tile!(nan), tile!(8m),  // 親初期 14 枚
    ]);

    let mut r = ScenarioRunner::from_scenario(s);
    assert_eq!(r.game.current_player, 0);
    // 親 (p0) が 8m を打牌
    assert!(r.discard(tile!(8m)), "p0 は 8m を持っているので打牌できる");
    assert_eq!(r.game.last_discard, Some(tile!(8m)));
    assert_eq!(r.game.current_player, 1, "next_player で南家に進む");

    let av = r.availability();
    assert!(av.can_ron[1], "p1 は 8m でロン可能");

    let result = r.try_ron(1).expect("p1 が 8m でロン成立");
    assert!(result.han >= 1);
    assert!(
        result.total_points > 0,
        "ロン和了点数が計算される (中身は #54 ドラ等の Issue で詳細確認)"
    );
    assert!(r
        .log()
        .iter()
        .any(|m| m.contains("p1 ron on 8m from p0")));
}

/// 山牌を末尾 (= 次にツモる牌) で制御できることを示す。
/// 「海底ツモ (#50)」のような状況役テストで、`wall.len()` を 1 にした状態で
/// `draw` を呼び切れることまでを確認する。役自体の検証は #50 で行う。
#[test]
fn wall_override_controls_next_draw() {
    let mut s = Scenario::default();
    s.dealer = 0;
    // 親初期 14 枚 (このシナリオでは p0 が即ツモせず 1 枚捨てて p1 に進める)
    s.hands[0] = Some(vec![
        tile!(1m), tile!(2m), tile!(3m),
        tile!(4m), tile!(5m), tile!(6m),
        tile!(7m), tile!(8m), tile!(9m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(ton), tile!(haku),
    ]);
    s.hands[1] = Some(vec![
        tile!(5p), tile!(5p), tile!(6p),
        tile!(7p), tile!(8p), tile!(2s),
        tile!(3s), tile!(4s), tile!(5s),
        tile!(6s), tile!(7s), tile!(nan),
        tile!(nan),
    ]);
    // 山牌は 1 枚だけ (海底想定)。p1 がツモる牌 = tile!(9p)。
    s.wall = vec![tile!(9p)];

    let mut r = ScenarioRunner::from_scenario(s);
    // p0 が haku を打牌 → p1 の手番
    assert!(r.discard(tile!(haku)));
    assert_eq!(r.game.current_player, 1);
    assert_eq!(r.game.wall.len(), 1);

    let drawn = r.draw().expect("wall に 1 枚あるのでツモれる");
    assert_eq!(drawn, tile!(9p), "シナリオで指定した牌がツモられる");
    assert_eq!(r.game.wall.len(), 0, "ツモ後の山牌は 0 枚");

    // TODO(#50): ここで `try_tsumo` が成立して `result.yaku.contains(&Yaku::Haitei)`
    // を確認するパターンを追加する。Haitei は #50 で実装するため本 Issue ではスコープ外。
    //
    // ただし「山切れ後の動作」インフラ自体は本 Issue で出来上がっているので、
    // wall.len() の遷移までは ここ で観測する。
    let _ = Yaku::Haitei; // referenced for future test
}

/// `availability()` が `can_pon` を正しく拾えるかを確認する。
///
/// p0 が `tile!(nan)` を打牌したとき、`nan` を 2 枚持つ p2 は can_pon[2]=true、
/// それ以外は false になることを assert する。
#[test]
fn availability_can_pon_reflects_other_players_hand() {
    let mut s = Scenario::default();
    s.dealer = 0;
    s.hands[0] = Some(vec![
        tile!(1m), tile!(2m), tile!(3m),
        tile!(4m), tile!(5m), tile!(6m),
        tile!(7m), tile!(8m), tile!(9m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(haku), tile!(nan),
    ]);
    // p2 が nan を 2 枚持つ
    s.hands[2] = Some(vec![
        tile!(nan), tile!(nan),
        tile!(1p), tile!(1p),
        tile!(5p), tile!(6p), tile!(7p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s), tile!(haku),
    ]);
    // p1 / p3 の手牌も明示する。Scenario::build() は未指定の手牌を
    // Game::new のランダム配牌のまま残すため、余った 4 枚目の nan が p1/p3 に
    // 配られて can_pon[1] / can_pon[3] が偶発的に true になりうる (フラキー)。
    // nan を 1 枚も含まない決定論的な手牌で固定する。
    s.hands[1] = Some(vec![
        tile!(1s), tile!(2s), tile!(3s),
        tile!(4s), tile!(5s), tile!(6s),
        tile!(7s), tile!(8s), tile!(9s),
        tile!(1p), tile!(2p), tile!(3p),
        tile!(4p),
    ]);
    s.hands[3] = Some(vec![
        tile!(1m), tile!(2m), tile!(3m),
        tile!(4m), tile!(5m), tile!(6m),
        tile!(7m), tile!(8m), tile!(9m),
        tile!(5p), tile!(6p), tile!(7p),
        tile!(8p),
    ]);

    let mut r = ScenarioRunner::from_scenario(s);
    // 親が nan を打牌
    assert!(r.discard(tile!(nan)));
    let av = r.availability();
    assert!(!av.can_pon[0], "打牌者自身はポン不可");
    assert!(av.can_pon[2], "p2 は nan 2 枚持ちでポン可能");
    // p1 / p3 は nan を 1 枚も持っていないので false
    assert!(!av.can_pon[1]);
    assert!(!av.can_pon[3]);
}

/// 牌マクロが期待通り tile_type を生成することの sanity check。
/// 詳細は scenario.rs 側にもあるが、外部 API 経由で再確認する。
#[test]
fn tile_macro_external_sanity_check() {
    use xmj_core::tile::TileType;
    let t = tile!(5p);
    match t.tile_type {
        TileType::Number { suit: Suit::Pin, value: 5 } => {}
        _ => panic!("tile!(5p) should be 5p"),
    }
    let h = tile!(chun);
    match h.tile_type {
        TileType::Honor(Honor::Chun) => {}
        _ => panic!("tile!(chun) should be Chun honor"),
    }
    // unused 変数を 1 つ作って `Tile` を返すことだけ確認 (型 import チェック含む)。
    let _: Tile = tile!(9s);
}

