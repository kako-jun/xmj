//! シナリオテスト基盤 (Issue #66)。
//!
//! 後続の役 Issue 群 (#49〜#61) では「特定の手牌・特定のツモ牌・特定の状況」を
//! 仕込んで役判定の組合せを検証する必要がある。本モジュールは
//!
//! - [`Scenario`]: 山牌・初期手牌・ドラ表示・親・本場などを「テスト用に上書き」する
//!   組立て器。`build()` で `Game` を返す。
//! - [`ScenarioRunner`]: 構築した `Game` を 1 手ずつ進めるラッパ。`draw` / `discard`
//!   /`try_tsumo` / `try_ron` などを呼び、内部に簡易ログを蓄える。
//! - [`tile!`] マクロ: テスト記述用の短い牌リテラル。`tile!(1m)` `tile!(haku)` 等。
//!
//! 設計方針:
//!
//! - **`#[cfg(feature = "wasm")]` 無しでコンパイルできる** こと。`cargo test` で
//!   そのまま動かす想定。判定ロジックは `Game` / `Player` / `Hand` の `pub` API
//!   と新設の `agari_extract` モジュールに依存する。
//! - **既存の `Game` のセマンティクスに干渉しない**。Scenario は `build()` 時に
//!   配牌・山牌・ドラを「直接書き換える」だけで、`Game` のメソッドには手を入れて
//!   いない。`ScenarioRunner` のラッパは既存 `wasm.rs` の流れ
//!   (`extract_agari_with_context` → `ScoringEngine::calculate_score`
//!   → `Game::resolve_win`) をそのまま辿る。
//!
//! TODO(#66 follow-up):
//! - `Game` に `can_tsumo(idx)` / `can_riichi(idx)` の薄いラッパを `pub` で生やすと、
//!   `ScenarioRunner::availability` から `wasm.rs` の判定をそのまま再利用できる。
//!   現状は `extract_agari` + `Game::can_riichi` 等を直接呼んで簡易に再実装している
//!   (#91: `Player::can_riichi` ではなく `Game::can_riichi` 経由で山牌残 4 枚以上の
//!   標準ルールを取り込む)。
//! - 役の追加 (#49-#61) は本モジュールには触らない。シナリオ側は「シナリオを組み立てて
//!   試行する」インフラだけを提供する。

use crate::agari_extract::extract_agari_with_context;
use crate::game::{Game, GameMode, Length, WinKind};
use crate::scoring::{ScoringEngine, ScoringResult};
use crate::tile::Tile;

/// シナリオ定義。`build()` で `Game` を組み立てる。
///
/// `Default` で構築すると Standard / Hanchan / dealer=0 / 通常初期化（ランダム山）の
/// 4 人ゲームになる。フィールドを書き換えてから `build()` を呼ぶことで「次にツモる牌」や
/// 「初期手牌」を仕込める。
///
/// # `wall` の意味
/// **末尾が次にツモられる牌** (`Vec::pop` 順)。長さは制限しない (元の `initialize_wall`
/// は 135 枚 = 山 + ドラ -1。テスト用には任意の長さで OK)。空の場合は通常初期化の
/// 山牌をそのまま使う。
///
/// # `hands` の意味
/// `Some(vec)` を渡したプレイヤーの手牌を完全上書きする (clear して push)。`None` は
/// 通常配牌のまま残す。**14 枚以上にしても assertion はかけない** ことで、
/// 「親初手 14 枚を仕込む」「副露込み 11 枚を仕込む」ような変則ケースをそのまま流せる。
///
/// # `dora_indicators` の意味
/// 通常初期化は山牌から 1 枚だけドラ表示として popする。Scenario で
/// 上書きしたい場合は空でない Vec を入れる。空の場合は通常初期化結果のまま。
pub struct Scenario {
    pub player_names: [String; 4],
    pub mode: GameMode,
    pub length: Length,
    pub wall: Vec<Tile>,
    pub hands: [Option<Vec<Tile>>; 4],
    pub dora_indicators: Vec<Tile>,
    pub dealer: usize,
    pub round: u32,
    pub honba: u32,
}

impl Default for Scenario {
    fn default() -> Self {
        Self {
            player_names: [
                "P0 東".to_string(),
                "P1 南".to_string(),
                "P2 西".to_string(),
                "P3 北".to_string(),
            ],
            mode: GameMode::Standard,
            length: Length::Hanchan,
            wall: Vec::new(),
            hands: [None, None, None, None],
            dora_indicators: Vec::new(),
            dealer: 0,
            round: 1,
            honba: 0,
        }
    }
}

impl Scenario {
    /// このシナリオから `Game` を組み立てる。
    ///
    /// 内部では `Game::new_with_mode_and_length` を呼んだ後、必要なフィールド
    /// (`wall` / 各 `Player.hand` / `dora_indicators` / `dealer` / `round` / `honba` /
    /// `current_player`) をシナリオ値で上書きする。
    ///
    /// `current_player` は `dealer` と同じにセットする。「自家ターンの開始時」を
    /// 想定したシナリオを書きやすくするため。
    pub fn build(self) -> Game {
        let mut game = Game::new_with_mode_and_length(
            self.player_names.to_vec(),
            self.mode,
            self.length,
        );
        if !self.wall.is_empty() {
            game.wall = self.wall;
        }
        for (i, h) in self.hands.into_iter().enumerate() {
            if let Some(tiles) = h {
                // 既存配牌をクリアして上書き。Hand::add_tile は内部で sort するので
                // 順不同で push して構わない。
                // `Hand::remove_tile` で全削除するとループ管理が手間なので、
                // 一旦 hand を新しく Default で置き換える方が単純。
                let player = &mut game.players[i];
                // tiles ベクタを clear するために remove_tile を繰り返す。
                // 既存 add_tile / remove_tile が pub API なのでこれで対応する。
                let snapshot: Vec<Tile> = player.hand.get_tiles().clone();
                for t in snapshot.iter() {
                    player.hand.remove_tile(t);
                }
                for t in tiles {
                    player.hand.add_tile(t);
                }
            }
        }
        if !self.dora_indicators.is_empty() {
            game.dora_indicators = self.dora_indicators;
        }
        game.dealer = self.dealer;
        game.round = self.round;
        game.honba = self.honba;
        game.current_player = self.dealer;
        // dealer フラグも整える (Game::new_with_mode_and_length は dealer=0 を真として
        // is_dealer をセットするので、シナリオで dealer を変えるならここで揃える)。
        for (i, p) in game.players.iter_mut().enumerate() {
            p.is_dealer = i == self.dealer;
        }
        game
    }
}

/// シナリオを 1 ステップずつ実行するランナー。
///
/// 内部に `Game` と簡易ログ (`Vec<String>`) を持ち、`draw` / `discard` / `try_tsumo`
/// 等を呼ぶたびにログを積む。和了系メソッドは `ScoringResult` をそのまま返すので、
/// テスト側で `result.yaku` / `result.han` / `result.total_points` を assert できる。
pub struct ScenarioRunner {
    pub game: Game,
    log: Vec<String>,
}

/// `availability()` のスナップショット。
///
/// テスト側は「この局面でロン/ポン/カン/チー/リーチ/ツモのどれが利用可能か」を
/// 一括で取得して assert したい場面が多い。`Game` 内部 API をまとめて呼ぶ薄いビュー。
#[derive(Debug, Clone)]
pub struct ActionAvailability {
    /// `current_player` がリーチ宣言可能か (`Game::can_riichi` 経由、#91)。
    pub can_riichi: bool,
    /// `current_player` がツモ和了可能か (14 枚 + `extract_agari` 成立)。
    pub can_tsumo: bool,
    /// 各プレイヤーが直前打牌に対してロン可能か (current_player 以外)。
    pub can_ron: [bool; 4],
    /// 各プレイヤーがポン可能か。
    pub can_pon: [bool; 4],
    /// 各プレイヤーが明槓可能か。
    pub can_kan: [bool; 4],
    /// 各プレイヤーがチー可能か (下家のみ true になり得る)。
    pub can_chi: [bool; 4],
}

impl ScenarioRunner {
    /// 既存 `Game` をラップ。
    pub fn new(game: Game) -> Self {
        Self {
            game,
            log: Vec::new(),
        }
    }

    /// `Scenario::build()` 経由でランナーを作るショートカット。
    pub fn from_scenario(s: Scenario) -> Self {
        Self::new(s.build())
    }

    /// ログを参照する。シナリオの「何が起きたか」をテストで覗くため。
    pub fn log(&self) -> &[String] {
        &self.log
    }

    /// ログに 1 行追記する。テスト本体から任意のマーカーを残したいケースのため pub。
    pub fn push_log(&mut self, msg: impl Into<String>) {
        self.log.push(msg.into());
    }

    /// 現在の局面で利用可能なアクションのスナップショット。
    ///
    /// `can_tsumo` は wasm.rs と同等の判定 (`tile_count == 14 && extract_agari.is_some()`)。
    /// `can_ron` は `last_discard` が存在 / 非闇牌 / 当該プレイヤーが`can_win` を満たす場合に true。
    pub fn availability(&self) -> ActionAvailability {
        let cur = self.game.current_player;
        // #91 fix: WasmGame と同じく Game::can_riichi 経由 (山牌残 4 枚以上を含む全条件で統一)
        let can_riichi = self.game.can_riichi(cur);

        let can_tsumo = {
            let hand = &self.game.players[cur].hand;
            hand.tile_count() == 14
                && extract_agari_with_context(hand, true, cur == self.game.dealer).is_some()
        };

        // 直前打牌者 = (current_player + 3) % 4。ロン可能なのは
        // 「直前打牌者以外」の 3 人。`cur` は次の手番のプレイヤーで、その人も
        // ロンできる (チャンスを逃さず即ロン宣言する可能性がある)。
        let last_discarder = (cur + 3) % 4;
        let can_ron: [bool; 4] = std::array::from_fn(|i| {
            if i == last_discarder {
                return false;
            }
            if self.game.last_discard_hidden {
                return false;
            }
            let Some(tile) = self.game.last_discard else {
                return false;
            };
            self.game.players[i].can_win(&tile)
        });

        let can_pon: [bool; 4] = std::array::from_fn(|i| self.game.can_pon(i));
        let can_kan: [bool; 4] = std::array::from_fn(|i| self.game.can_kan(i));
        let can_chi: [bool; 4] = std::array::from_fn(|i| self.game.can_chi(i));

        ActionAvailability {
            can_riichi,
            can_tsumo,
            can_ron,
            can_pon,
            can_kan,
            can_chi,
        }
    }

    /// `current_player` が山牌から 1 枚ツモる。
    ///
    /// 戻り値: 引けた牌 (`Some`) / 山切れ (`None`)。
    /// 引けたときは手牌に追加されログ ("p{idx} draws {tile}") を残す。
    pub fn draw(&mut self) -> Option<Tile> {
        let idx = self.game.current_player;
        let tile = self.game.draw_tile();
        if let Some(t) = tile {
            self.game.players[idx].draw_tile(t);
            self.push_log(format!("p{} draws {}", idx, t.to_string()));
        } else {
            self.push_log(format!("p{} cannot draw (wall empty)", idx));
        }
        tile
    }

    /// `current_player` が `tile` を打牌する。
    ///
    /// 内部は `Game::discard_tile` をそのまま呼ぶので、河に積まれ `last_discard`
    /// が更新され `next_player()` まで進む。
    pub fn discard(&mut self, tile: Tile) -> bool {
        let idx = self.game.current_player;
        let ok = self.game.discard_tile(tile);
        if ok {
            self.push_log(format!("p{} discards {}", idx, tile.to_string()));
        } else {
            self.push_log(format!("p{} cannot discard {}", idx, tile.to_string()));
        }
        ok
    }

    /// `current_player` のツモ和了を試みる。
    ///
    /// `extract_agari_with_context` で和了形を抽出し、`ScoringEngine::calculate_score`
    /// を回した結果を返す。和了形でなければ `None`。
    ///
    /// 成功時は `Game::resolve_win(winner, WinKind::Tsumo, ...)` を呼んで点数移動・
    /// `last_outcome` 設定まで一気に進める。
    pub fn try_tsumo(&mut self) -> Option<ScoringResult> {
        let winner = self.game.current_player;
        let is_dealer = winner == self.game.dealer;
        let hand_clone = self.game.players[winner].hand.clone();
        let (sub_hand, winning_tile) =
            extract_agari_with_context(&hand_clone, true, is_dealer)?;
        let ctx = self.game.build_scoring_context(winner, true);
        let result =
            ScoringEngine::calculate_score_with_context(&sub_hand, &winning_tile, &ctx)?;
        self.push_log(format!(
            "p{} tsumo with {} (han={}, fu={})",
            winner,
            winning_tile.to_string(),
            result.han,
            result.fu
        ));
        self.game.resolve_win(winner, WinKind::Tsumo, result.clone());
        Some(result)
    }

    /// `player` がロン和了を試みる。打牌者は `Game::last_discard` 由来。
    ///
    /// 内部仕様は `wasm.rs::resolve_win_ron` と同じ:
    ///   - `last_discard` が無ければ `None`
    ///   - 当該プレイヤーが当該牌で `can_win` でなければ `None`
    ///   - `ScoringEngine::calculate_score` 不成立なら `None`
    pub fn try_ron(&mut self, player: usize) -> Option<ScoringResult> {
        let winning_tile = self.game.last_discard?;
        if self.game.last_discard_hidden {
            return None;
        }
        let from = self
            .game
            .last_discarder
            .unwrap_or((self.game.current_player + 3) % 4);
        let ctx = self.game.build_scoring_context(player, false);
        let hand = &self.game.players[player].hand;
        if !hand.can_win(&winning_tile) {
            return None;
        }
        let result =
            ScoringEngine::calculate_score_with_context(hand, &winning_tile, &ctx)?;
        self.push_log(format!(
            "p{} ron on {} from p{} (han={}, fu={})",
            player,
            winning_tile.to_string(),
            from,
            result.han,
            result.fu
        ));
        self.game
            .resolve_win(player, WinKind::Ron { from }, result.clone());
        Some(result)
    }

    /// `current_player` のリーチ宣言。`Player::declare_riichi` 経由。
    pub fn declare_riichi(&mut self) -> bool {
        let idx = self.game.current_player;
        let ok = self.game.declare_riichi(idx);
        if ok {
            self.push_log(format!("p{} declares riichi", idx));
        }
        ok
    }

    /// `player` がポンを実行。
    pub fn pon(&mut self, player: usize) -> bool {
        let ok = self.game.do_pon(player);
        if ok {
            self.push_log(format!("p{} pon", player));
        }
        ok
    }

    /// `player` が明槓 (大明槓) を実行。
    pub fn kan(&mut self, player: usize) -> bool {
        let ok = self.game.do_kan(player);
        if ok {
            self.push_log(format!("p{} kan", player));
        }
        ok
    }

    /// `current_player` が暗槓を実行。
    pub fn ankan(&mut self, tile: Tile) -> bool {
        let idx = self.game.current_player;
        let ok = self.game.do_ankan(idx, tile);
        if ok {
            self.push_log(format!("p{} ankan {}", idx, tile.to_string()));
        }
        ok
    }

    /// `player` がチーを実行 (pattern 0/1/2)。
    pub fn chi(&mut self, player: usize, pattern: usize) -> bool {
        let ok = self.game.do_chi(player, pattern);
        if ok {
            self.push_log(format!("p{} chi pattern={}", player, pattern));
        }
        ok
    }
}

/// テスト記述用の牌リテラルマクロ。
///
/// 数牌: `tile!(1m)` `tile!(5p)` `tile!(9s)` (1〜9 のリテラル数字 + m/p/s)
/// 字牌: `tile!(ton) tile!(nan) tile!(shaa) tile!(pei) tile!(haku) tile!(hatsu) tile!(chun)`
///
/// `tile!` は赤ドラ・透明牌のフラグは持たない。必要なら `with_glass(true)` や
/// `Tile::new_number(_, _, true)` を直接呼ぶこと。
///
/// 例:
/// ```
/// use xmj_core::tile;
/// use xmj_core::tile::{Tile, TileType, Suit, Honor};
/// let t = tile!(5m);
/// assert!(matches!(t.tile_type, TileType::Number { suit: Suit::Man, value: 5 }));
/// let h = tile!(haku);
/// assert!(matches!(h.tile_type, TileType::Honor(Honor::Haku)));
/// ```
#[macro_export]
macro_rules! tile {
    (1m) => { $crate::tile::Tile::new_number($crate::tile::Suit::Man, 1, false) };
    (2m) => { $crate::tile::Tile::new_number($crate::tile::Suit::Man, 2, false) };
    (3m) => { $crate::tile::Tile::new_number($crate::tile::Suit::Man, 3, false) };
    (4m) => { $crate::tile::Tile::new_number($crate::tile::Suit::Man, 4, false) };
    (5m) => { $crate::tile::Tile::new_number($crate::tile::Suit::Man, 5, false) };
    (6m) => { $crate::tile::Tile::new_number($crate::tile::Suit::Man, 6, false) };
    (7m) => { $crate::tile::Tile::new_number($crate::tile::Suit::Man, 7, false) };
    (8m) => { $crate::tile::Tile::new_number($crate::tile::Suit::Man, 8, false) };
    (9m) => { $crate::tile::Tile::new_number($crate::tile::Suit::Man, 9, false) };
    (1p) => { $crate::tile::Tile::new_number($crate::tile::Suit::Pin, 1, false) };
    (2p) => { $crate::tile::Tile::new_number($crate::tile::Suit::Pin, 2, false) };
    (3p) => { $crate::tile::Tile::new_number($crate::tile::Suit::Pin, 3, false) };
    (4p) => { $crate::tile::Tile::new_number($crate::tile::Suit::Pin, 4, false) };
    (5p) => { $crate::tile::Tile::new_number($crate::tile::Suit::Pin, 5, false) };
    (6p) => { $crate::tile::Tile::new_number($crate::tile::Suit::Pin, 6, false) };
    (7p) => { $crate::tile::Tile::new_number($crate::tile::Suit::Pin, 7, false) };
    (8p) => { $crate::tile::Tile::new_number($crate::tile::Suit::Pin, 8, false) };
    (9p) => { $crate::tile::Tile::new_number($crate::tile::Suit::Pin, 9, false) };
    (1s) => { $crate::tile::Tile::new_number($crate::tile::Suit::Sou, 1, false) };
    (2s) => { $crate::tile::Tile::new_number($crate::tile::Suit::Sou, 2, false) };
    (3s) => { $crate::tile::Tile::new_number($crate::tile::Suit::Sou, 3, false) };
    (4s) => { $crate::tile::Tile::new_number($crate::tile::Suit::Sou, 4, false) };
    (5s) => { $crate::tile::Tile::new_number($crate::tile::Suit::Sou, 5, false) };
    (6s) => { $crate::tile::Tile::new_number($crate::tile::Suit::Sou, 6, false) };
    (7s) => { $crate::tile::Tile::new_number($crate::tile::Suit::Sou, 7, false) };
    (8s) => { $crate::tile::Tile::new_number($crate::tile::Suit::Sou, 8, false) };
    (9s) => { $crate::tile::Tile::new_number($crate::tile::Suit::Sou, 9, false) };
    (ton) => { $crate::tile::Tile::new_honor($crate::tile::Honor::Ton) };
    (nan) => { $crate::tile::Tile::new_honor($crate::tile::Honor::Nan) };
    (shaa) => { $crate::tile::Tile::new_honor($crate::tile::Honor::Shaa) };
    (pei) => { $crate::tile::Tile::new_honor($crate::tile::Honor::Pei) };
    (haku) => { $crate::tile::Tile::new_honor($crate::tile::Honor::Haku) };
    (hatsu) => { $crate::tile::Tile::new_honor($crate::tile::Honor::Hatsu) };
    (chun) => { $crate::tile::Tile::new_honor($crate::tile::Honor::Chun) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{Honor, Suit, TileType};

    #[test]
    fn tile_macro_produces_expected_tiles() {
        let t1 = tile!(5m);
        assert!(matches!(t1.tile_type, TileType::Number { suit: Suit::Man, value: 5 }));
        assert!(!t1.is_red);

        let t2 = tile!(9p);
        assert!(matches!(t2.tile_type, TileType::Number { suit: Suit::Pin, value: 9 }));

        let h = tile!(haku);
        assert!(matches!(h.tile_type, TileType::Honor(Honor::Haku)));
    }

    #[test]
    fn default_scenario_builds_standard_game() {
        let s = Scenario::default();
        let game = s.build();
        assert_eq!(game.players.len(), 4);
        assert_eq!(game.dealer, 0);
        assert_eq!(game.current_player, 0);
        assert_eq!(game.round, 1);
        assert_eq!(game.honba, 0);
        assert_eq!(game.mode, GameMode::Standard);
        // 通常初期化なら親 14 枚 / 子 13 枚配られる
        assert_eq!(game.players[0].hand.tile_count(), 14);
        for i in 1..4 {
            assert_eq!(game.players[i].hand.tile_count(), 13);
        }
    }

    #[test]
    fn scenario_overrides_wall_and_hands() {
        let mut s = Scenario::default();
        // 山牌を 3 枚に縮める。`Vec::pop` 末尾から取るので「次にツモる」のは tile!(9m)。
        s.wall = vec![tile!(1m), tile!(2m), tile!(9m)];
        // p0 の手牌を 13 枚に置き換える (通常初期は親 14 枚なので減らす形)。
        s.hands[0] = Some(vec![
            tile!(1p), tile!(2p), tile!(3p),
            tile!(4p), tile!(5p), tile!(6p),
            tile!(7p), tile!(8p), tile!(9p),
            tile!(2s), tile!(3s), tile!(4s),
            tile!(ton),
        ]);
        let mut game = s.build();
        assert_eq!(game.wall.len(), 3);
        assert_eq!(game.players[0].hand.tile_count(), 13);
        assert_eq!(game.draw_tile(), Some(tile!(9m)));
        assert_eq!(game.wall.len(), 2);
    }

    #[test]
    fn scenario_overrides_dora_dealer_round_honba() {
        let mut s = Scenario::default();
        s.dora_indicators = vec![tile!(haku)];
        s.dealer = 2;
        s.round = 5;
        s.honba = 3;
        let game = s.build();
        assert_eq!(game.dora_indicators, vec![tile!(haku)]);
        assert_eq!(game.dealer, 2);
        assert_eq!(game.current_player, 2);
        assert_eq!(game.round, 5);
        assert_eq!(game.honba, 3);
        assert!(game.players[2].is_dealer);
        assert!(!game.players[0].is_dealer);
    }

    #[test]
    fn runner_logs_draw_and_discard() {
        let mut s = Scenario::default();
        s.wall = vec![tile!(9m)];
        // 親が 14 枚 → 何か 1 枚捨ててから検査するのは煩雑なので、
        // 親手牌を 13 枚に置き換えて draw→discard の一往復を見る。
        s.hands[0] = Some(vec![
            tile!(1p), tile!(2p), tile!(3p),
            tile!(4p), tile!(5p), tile!(6p),
            tile!(7p), tile!(8p), tile!(9p),
            tile!(2s), tile!(3s), tile!(4s),
            tile!(ton),
        ]);
        let mut r = ScenarioRunner::from_scenario(s);
        let drawn = r.draw().expect("wall has 1 tile");
        assert_eq!(drawn, tile!(9m));
        assert_eq!(r.game.players[0].hand.tile_count(), 14);
        assert!(r.discard(tile!(9m)));
        assert_eq!(r.game.last_discard, Some(tile!(9m)));
        assert_eq!(r.game.current_player, 1);
        assert!(r
            .log()
            .iter()
            .any(|m| m.contains("draws 9m")));
        assert!(r
            .log()
            .iter()
            .any(|m| m.contains("discards 9m")));
    }
}
