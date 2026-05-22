//! 暗槓 / 加槓 (小明槓) と槍槓発火配線の統合テスト (Issue #46)。
//!
//! Rust core 側 `Game::do_ankan` / `start_shouminkan` / `complete_shouminkan` /
//! `cancel_shouminkan` の単体挙動と、加槓中の牌で他家がロンしたときに
//! `ScoringContext.is_chankan=true` 経路で `Yaku::Chankan` が点数に乗ることを検証する。
//!
//! `cargo test` (default feature) で動く。wasm bridge 自体の薄ラッパは
//! 役割上関数呼び出し転送だけなので、ここでは core API の挙動を直接 assert する。

use xmj_core::game::{Game, GameMode, Length};
use xmj_core::hand::{Hand, Meld, MeldType};
use xmj_core::scoring::{ScoringEngine, Yaku};
use xmj_core::tile;
use xmj_core::tile::Tile;

fn fresh_game() -> Game {
    Game::new_with_mode_and_length(
        vec!["P1".into(), "P2".into(), "P3".into(), "P4".into()],
        GameMode::Standard,
        Length::Hanchan,
    )
}

/// 暗槓: 手牌に 4 枚揃いがあれば `do_ankan` で副露が立つ + 嶺上ツモ + 槓ドラ追加。
#[test]
fn test_do_ankan_basic() {
    let mut game = fresh_game();
    // 山牌の末尾 2 枚: 1 つは槓ドラ表示牌、もう 1 つは嶺上ツモになる
    game.wall = vec![tile!(9p), tile!(2s)];

    let mut hand = Hand::new();
    for t in [
        tile!(2m), tile!(3m), tile!(4m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(5p),
        tile!(8m), tile!(8m), tile!(8m), tile!(8m),
    ] {
        hand.add_tile(t);
    }
    game.players[0].hand = hand;
    game.current_player = 0;
    game.dealer = 0;

    let dora_count_before = game.dora_indicators.len();
    assert!(
        game.can_ankan(0).contains(&tile!(8m)),
        "can_ankan が 8m を返す"
    );
    assert!(game.do_ankan(0, tile!(8m)), "暗カン 8m 成立");
    assert!(game.last_was_rinshan, "嶺上開花フラグが立つ");
    assert_eq!(
        game.dora_indicators.len(),
        dora_count_before + 1,
        "槓ドラ表示牌が 1 枚増える"
    );
    // 暗槓副露が 1 つ立つ
    let melds = game.players[0].hand.get_melds();
    assert_eq!(melds.len(), 1);
    assert!(matches!(melds[0].meld_type, MeldType::Kan));
    assert!(!melds[0].is_open, "暗槓は is_open=false");
    // 嶺上ツモが手牌に入っている (元 11 枚 - 4 枚 + 1 枚 = 8 枚)
    assert_eq!(game.players[0].hand.get_tiles().len(), 8);
}

/// テストヘルパー: 手牌に Pon 副露を仕込む (Hand::add_meld が hand から該当牌を
/// 自動的に取り除く仕様に合わせて、副露分の 3 枚も pre-add してから add_meld する)。
fn install_pon(hand: &mut Hand, tile: Tile) {
    for _ in 0..3 {
        hand.add_tile(tile);
    }
    hand.add_meld(Meld {
        meld_type: MeldType::Pon,
        tiles: vec![tile, tile, tile],
        is_open: true,
        ..Default::default()
    });
}

/// 加槓: 既存の Pon meld と同じ牌が手牌にあれば加槓候補に上がる。
#[test]
fn test_can_shouminkan_returns_pon_matching_tile() {
    let mut game = fresh_game();
    // 手牌に 5m を 1 枚、副露に Pon(5m) を仕込む
    let mut hand = Hand::new();
    hand.add_tile(tile!(2p));
    install_pon(&mut hand, tile!(5m));
    // Pon を立てた後で「加槓用の 5m」を 1 枚追加 (4 枚目)
    hand.add_tile(tile!(5m));
    game.players[0].hand = hand;

    let candidates = game.can_shouminkan(0);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0], tile!(5m));

    // 別の牌は候補に入らない
    assert!(!candidates.contains(&tile!(2p)));
}

/// 加槓 2 段階フロー: start → (誰もロンしなければ) complete で Kan に書き換わる。
#[test]
fn test_shouminkan_complete_writes_pon_to_kan() {
    let mut game = fresh_game();
    // 槓ドラ + 嶺上ツモ用に山牌の末尾 2 枚を仕込む
    game.wall = vec![tile!(1m), tile!(2m)];

    let mut hand = Hand::new();
    hand.add_tile(tile!(8p));
    install_pon(&mut hand, tile!(7p));
    hand.add_tile(tile!(7p)); // 加槓用の 4 枚目
    game.players[0].hand = hand;
    game.current_player = 0;

    assert!(game.start_shouminkan(0, tile!(7p)), "加槓宣言成功");
    assert_eq!(game.pending_chankan, Some(tile!(7p)), "pending_chankan が立つ");
    // この時点では meld はまだ Pon のまま (槍槓窓口中)
    {
        let melds = game.players[0].hand.get_melds();
        assert!(matches!(melds[0].meld_type, MeldType::Pon));
        assert_eq!(melds[0].tiles.len(), 3);
    }

    let dora_before = game.dora_indicators.len();
    assert!(game.complete_shouminkan(0, tile!(7p)), "加槓完了");
    assert!(game.pending_chankan.is_none(), "完了で pending_chankan クリア");
    assert!(game.last_was_rinshan, "嶺上開花候補フラグが立つ");
    assert_eq!(
        game.dora_indicators.len(),
        dora_before + 1,
        "槓ドラが 1 枚追加"
    );

    let melds = game.players[0].hand.get_melds();
    assert_eq!(melds.len(), 1);
    assert!(matches!(melds[0].meld_type, MeldType::Kan), "Pon → Kan");
    assert_eq!(melds[0].tiles.len(), 4, "4 枚構成");
    assert!(melds[0].is_open, "加槓は明 (is_open=true)");
}

/// 加槓宣言を cancel すると pending_chankan が None に戻り、meld は Pon のまま。
#[test]
fn test_shouminkan_cancel_keeps_pon() {
    let mut game = fresh_game();
    let mut hand = Hand::new();
    install_pon(&mut hand, tile!(haku));
    hand.add_tile(tile!(haku)); // 加槓用の 4 枚目
    game.players[0].hand = hand;

    assert!(game.start_shouminkan(0, tile!(haku)));
    game.cancel_shouminkan();
    assert!(game.pending_chankan.is_none());
    // meld は Pon のまま
    let melds = game.players[0].hand.get_melds();
    assert!(matches!(melds[0].meld_type, MeldType::Pon));
    // 手牌の haku もまだ残っている (cancel では除去しない)
    assert!(game
        .players[0]
        .hand
        .get_tiles()
        .iter()
        .any(|t| *t == tile!(haku)));
}

/// 加槓中の牌で他家がロンしたとき、`build_scoring_context(is_tsumo=false)` で
/// is_chankan=true が立ち、`Yaku::Chankan` が役に含まれる。
#[test]
fn test_chankan_yaku_on_shouminkan_ron() {
    let mut game = fresh_game();

    // p1 (子) の手牌を Chankan 待ち形に仕込む。
    // 23m + 234p + 234s + 567s + 88m (= 13 枚、3m or 1m 単騎ではなく 1m/4m 両面待ち).
    // 加槓する牌は p0 が Pon(1m) 持ちで手牌の 1m を加槓する流れにする。
    // p1 の待ちは 23m → 1m/4m なので 1m で和了形成立。
    let mut p1_hand = Hand::new();
    for t in [
        tile!(2m), tile!(3m),
        tile!(2p), tile!(3p), tile!(4p),
        tile!(2s), tile!(3s), tile!(4s),
        tile!(5s), tile!(6s), tile!(7s),
        tile!(8m), tile!(8m),
    ] {
        p1_hand.add_tile(t);
    }
    game.players[1].hand = p1_hand;

    // p0 (加槓宣言者): 手牌に 1m を 1 枚 + Pon(1m) 副露済み
    let mut p0_hand = Hand::new();
    p0_hand.add_tile(tile!(9p));
    install_pon(&mut p0_hand, tile!(1m));
    p0_hand.add_tile(tile!(1m)); // 加槓用の 4 枚目
    game.players[0].hand = p0_hand;
    game.current_player = 0;

    // 加槓宣言で pending_chankan = Some(1m)
    assert!(game.start_shouminkan(0, tile!(1m)));

    // p1 がこの 1m でロンできることを確認
    assert!(
        game.players[1].can_win(&tile!(1m)),
        "p1 は 1m でロン和了形を作れる"
    );

    // p1 視点の ScoringContext を組む (ロン = is_tsumo=false)
    let ctx = game.build_scoring_context(1, false);
    assert!(
        ctx.is_chankan,
        "pending_chankan が立っているので is_chankan=true"
    );

    // 点数計算: 1m を加えると 123m / 234p / 234s / 567s / 88m → 平和形相当
    let hand = &game.players[1].hand;
    let result = ScoringEngine::calculate_score_with_context(hand, &tile!(1m), &ctx)
        .expect("和了点数計算成功");
    assert!(
        result.yaku.contains(&Yaku::Chankan),
        "Chankan が役に含まれる: {:?}",
        result.yaku
    );
}

/// 同じ牌が手牌にも副露 Pon にも無ければ can_shouminkan は空。
#[test]
fn test_can_shouminkan_empty_when_no_match() {
    let mut game = fresh_game();
    let mut hand = Hand::new();
    hand.add_tile(tile!(2m));
    install_pon(&mut hand, tile!(5p));
    // 手牌の 2m は 5p と無関係なので候補ゼロ
    game.players[0].hand = hand;
    assert!(game.can_shouminkan(0).is_empty());
}

/// Pon 副露が無いと can_shouminkan は空 (チー / カン 副露は対象外)。
#[test]
fn test_can_shouminkan_ignores_non_pon_melds() {
    let mut game = fresh_game();
    let mut hand = Hand::new();
    // Chi(3m,4m,5m) を先に仕込んでから 5m を手牌に 1 枚追加
    for t in [tile!(3m), tile!(4m), tile!(5m)] {
        hand.add_tile(t);
    }
    hand.add_meld(Meld {
        meld_type: MeldType::Chi,
        tiles: vec![tile!(3m), tile!(4m), tile!(5m)],
        is_open: true,
        ..Default::default()
    });
    hand.add_tile(tile!(5m));
    game.players[0].hand = hand;
    assert!(
        game.can_shouminkan(0).is_empty(),
        "Chi meld は加槓候補にしない"
    );
}
