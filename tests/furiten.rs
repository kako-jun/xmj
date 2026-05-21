//! フリテン判定 (Issue #56) のシナリオテスト。
//!
//! 検証する 3 種類のフリテン:
//!   1. 通常フリテン: 自分の捨て牌に自分の待ち牌のいずれかが含まれているとロン不可
//!   2. 同巡フリテン: ロンを 1 度見逃したら自分の次のツモまでロン不可
//!   3. 立直後フリテン: 立直後にロン見逃しが発生したら局終了まで永続フリテン
//!
//! 比較は赤ドラ無視 (tile_type のみ) で行う。`Player::is_furiten` の戻り値と
//! `Player::notify_ron_skipped` / `draw_tile` のフラグ遷移を直接検証する。

use xmj_core::hand::Hand;
use xmj_core::player::Player;
use xmj_core::tile::{Suit, Tile};

/// 8m 単騎テンパイの 13 枚を作る。
/// 構成: 234m 234p 234s 567s 8m → 8m 単騎待ち
fn build_tenpai_tanki_8m() -> Hand {
    let mut hand = Hand::new();
    for t in [
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
    ] {
        hand.add_tile(t);
    }
    hand
}

/// 自分の捨て牌に待ち牌そのものがある → 通常フリテン (`is_furiten` = true)。
#[test]
fn furiten_self_discard_blocks_ron() {
    let mut player = Player::new(0, "P".into());
    player.hand = build_tenpai_tanki_8m();
    // 待ち牌 8m を一度捨てている状況を作る
    // discard_tile は手牌から該当牌を除く必要があるので、一旦ツモ→打牌の経路を再現する
    player.draw_tile(Tile::new_number(Suit::Man, 8, false));
    assert!(player.discard_tile(Tile::new_number(Suit::Man, 8, false)));
    // 自分のツモで skipped_ron_this_turn が false になっているはず (通常フリテン経路の検証)
    assert!(!player.skipped_ron_this_turn);
    assert!(!player.permanent_furiten);

    assert!(
        player.is_furiten(),
        "自分の捨て牌に待ち牌 8m があるのでフリテン"
    );
}

/// 自分の捨て牌に待ち牌が無ければ非フリテン (待ちと無関係な牌を捨てた状態)。
#[test]
fn no_furiten_when_discards_unrelated() {
    let mut player = Player::new(0, "P".into());
    player.hand = build_tenpai_tanki_8m();
    // 関係ない牌 (2m) を捨てる: ツモ → 打牌
    player.draw_tile(Tile::new_number(Suit::Man, 2, false));
    assert!(player.discard_tile(Tile::new_number(Suit::Man, 2, false)));

    assert!(
        !player.is_furiten(),
        "待ち牌 (8m) と無関係な捨て牌しかないので非フリテン"
    );
}

/// 待ち牌と赤ドラの同種牌は赤フラグを無視して同等視する。
/// 待ち = 8m (通常牌)、捨て牌に 8m (赤) が紛れていたら通常フリテン。
#[test]
fn furiten_treats_red_tile_as_same() {
    let mut player = Player::new(0, "P".into());
    player.hand = build_tenpai_tanki_8m();
    // 赤 8m を捨てる (待ち = 8m と tile_type 一致 / is_red のみ違う)
    let red_8m = Tile::new_number(Suit::Man, 8, true);
    player.draw_tile(red_8m);
    assert!(player.discard_tile(red_8m));

    assert!(
        player.is_furiten(),
        "赤 8m は通常 8m と同等視されるのでフリテン"
    );
}

/// `notify_ron_skipped` を呼ぶと同巡フリテンに入り、`draw_tile` で解除される。
#[test]
fn furiten_temporary_after_skip_and_clears_on_next_draw() {
    let mut player = Player::new(0, "P".into());
    player.hand = build_tenpai_tanki_8m();

    assert!(!player.is_furiten(), "初期状態は非フリテン");
    player.notify_ron_skipped();
    assert!(
        player.skipped_ron_this_turn,
        "見逃しで同巡フリテンフラグが立つ"
    );
    assert!(
        player.is_furiten(),
        "同巡フリテン中はロン不可"
    );
    assert!(
        !player.permanent_furiten,
        "立直していなければ永続フリテンにはならない"
    );

    // 自分の次のツモで同巡フリテン解除
    player.draw_tile(Tile::new_number(Suit::Pin, 7, false));
    assert!(
        !player.skipped_ron_this_turn,
        "ツモで同巡フリテン解除"
    );
    // ツモ後は手牌 14 枚なので compute_machi_tiles は空 → 通常フリテンも false
    assert!(!player.is_furiten(), "ツモで通常進行に戻る");
}

/// 立直済みでロンを見逃したら永続フリテン (局終了まで解除されない)。
#[test]
fn furiten_permanent_after_riichi_skip() {
    let mut player = Player::new(0, "P".into());
    player.hand = build_tenpai_tanki_8m();
    player.is_riichi = true;

    player.notify_ron_skipped();
    assert!(player.permanent_furiten, "立直後の見逃しで永続フリテン");
    assert!(player.is_furiten(), "永続フリテン中はロン不可");

    // ツモしても永続フリテンは解けない
    player.draw_tile(Tile::new_number(Suit::Pin, 7, false));
    assert!(
        !player.skipped_ron_this_turn,
        "同巡フリテンフラグはツモで解けるが..."
    );
    assert!(
        player.permanent_furiten,
        "永続フリテンはツモでも解けない"
    );
    assert!(player.is_furiten(), "永続フリテン継続");
}

/// `reset_for_next_round` で全てのフリテンフラグが解除される。
#[test]
fn furiten_flags_clear_on_next_round() {
    let mut player = Player::new(0, "P".into());
    player.hand = build_tenpai_tanki_8m();
    player.is_riichi = true;
    player.notify_ron_skipped();
    assert!(player.permanent_furiten);
    assert!(player.skipped_ron_this_turn);

    player.reset_for_next_round();
    assert!(
        !player.permanent_furiten,
        "局を跨ぐと永続フリテンも解除される"
    );
    assert!(!player.skipped_ron_this_turn, "同巡フリテンも解除");
    assert!(!player.is_riichi, "リーチフラグも解除");
}

/// テンパイしていない (`compute_machi_tiles` が空) 状態は通常フリテン非該当。
/// ただし `skipped_ron_this_turn` / `permanent_furiten` が立っていればフリテン扱い。
#[test]
fn no_normal_furiten_when_not_tenpai() {
    let mut player = Player::new(0, "P".into());
    // バラバラ 13 枚 (待ち無し)
    for t in [
        Tile::new_number(Suit::Man, 1, false),
        Tile::new_number(Suit::Man, 3, false),
        Tile::new_number(Suit::Man, 5, false),
        Tile::new_number(Suit::Man, 7, false),
        Tile::new_number(Suit::Pin, 2, false),
        Tile::new_number(Suit::Pin, 4, false),
        Tile::new_number(Suit::Pin, 6, false),
        Tile::new_number(Suit::Pin, 8, false),
        Tile::new_number(Suit::Sou, 1, false),
        Tile::new_number(Suit::Sou, 3, false),
        Tile::new_number(Suit::Sou, 5, false),
        Tile::new_number(Suit::Sou, 7, false),
        Tile::new_number(Suit::Sou, 9, false),
    ] {
        player.hand.add_tile(t);
    }
    // 待ち牌があるかのように見せたいので何か 1 つ捨てておく
    player.draw_tile(Tile::new_number(Suit::Man, 2, false));
    assert!(player.discard_tile(Tile::new_number(Suit::Man, 2, false)));

    assert!(
        !player.is_furiten(),
        "テンパイしていない手は通常フリテン非該当"
    );

    // 見逃しフラグだけは効く (テンパイ判定とは独立)
    player.notify_ron_skipped();
    assert!(
        player.is_furiten(),
        "テンパイしていなくても skipped_ron_this_turn が立てばフリテン扱い"
    );
}
