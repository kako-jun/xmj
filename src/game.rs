use crate::player::Player;
use crate::scoring::Yaku;
use crate::tile::{Tile, TileType, Suit, Honor};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::{HashMap, HashSet};

/// ゲームモード
///
/// - `Standard`: 通常ルール
/// - `Seikyo`: 誠京麻雀（『天』『アカギ』の裏ルール）。場代・二度ヅモ・役満祝儀
/// - `Washizu`: 鷲巣麻雀（『アカギ』）。全牌の 3/4 が透明で他家からも見える
/// - `FiveTile`: 5枚麻雀（クライマックスだけ麻雀）。手牌 5 枚（親 6 枚）スタート、
///   雀頭+面子1組で和了、タンヤオのみ判定
/// - `EastWest`: 東西戦（クリア麻雀、『天』のチーム戦ルール）。東家+西家＝東チーム、
///   南家+北家＝西チーム。指定二翻役5種を先にチームとして全て揃えた方の勝利。
/// - `Yamima`: 闇麻。プレイヤーは点棒 1000 点を支払って打牌を**裏向き（闇牌）**で
///   河に置ける。他家からは「闇牌」（種類非公開）として見え、鳴き・ロンの対象に
///   できない。ターンプレイヤーは点棒 500 点を支払って「照射」を宣言することで
///   他家の闇牌を公開させられる。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Standard,
    Seikyo,
    Washizu,
    FiveTile,
    EastWest,
    Yamima,
}

/// 東西戦（クリア麻雀）のチーム。
///
/// 麻雀の座席名（東家 = ton, 南家 = nan, 西家 = shaa, 北家 = pei）と
/// チーム名（East / West）は別概念であることに注意。
/// 座席 0 (東家) + 座席 2 (西家) → East チーム、
/// 座席 1 (南家) + 座席 3 (北家) → West チーム。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Team {
    East,
    West,
}

/// 座席 index からチームを返すヘルパー。
///
/// - 座席 0 (東家) と 2 (西家) → `Team::East`
/// - 座席 1 (南家) と 3 (北家) → `Team::West`
pub fn team_of(seat_idx: usize) -> Team {
    match seat_idx {
        0 | 2 => Team::East,
        _ => Team::West,
    }
}

/// 東西戦のクリア対象役5種（指定二翻役）。
///
/// 三色同順 / 一気通貫 / 対々和 / 全帯么 / 混老頭
pub fn east_west_target_yaku() -> [Yaku; 5] {
    [
        Yaku::SanshokuDoujun,
        Yaku::Ittsu,
        Yaku::Toitoi,
        Yaku::Chanta,
        Yaku::Honroutou,
    ]
}

/// 誠京麻雀の固定額
pub const SEIKYO_SEAT_FEE: i32 = 1000;
pub const SEIKYO_YAKUMAN_TIP: i32 = 8000;

/// 闇麻の固定額
pub const YAMIMA_HIDDEN_COST: i32 = 1000;
pub const YAMIMA_LIGHT_UP_COST: i32 = 500;

#[derive(Debug, Clone)]
pub struct Game {
    pub players: Vec<Player>,
    pub wall: Vec<Tile>,
    pub dora_indicators: Vec<Tile>,
    pub current_player: usize,
    pub round: u32,
    pub dealer: usize,
    pub last_discard: Option<Tile>,
    /// 直近の打牌が闇牌（Yamima ルール）かどうか。
    ///
    /// `discard_hidden_tile` で打牌したときに true、通常 `discard_tile` で false に
    /// 戻る。`can_someone_win` / `can_pon` / `can_chi` / `can_kan` はこのフラグが
    /// true のとき**全てロン・鳴き不可**として扱う（闇牌は鳴けない仕様）。
    /// 照射（`light_up`）で実際に公開された後でも、`last_discard_hidden` は
    /// 「最後の打牌時点での裏/表」を表すフィールドであり、過去ターンの照射成功で
    /// 鳴きが復活する設計ではない（鳴きは原則直後 1 ターン以内）。
    pub last_discard_hidden: bool,
    /// ゲームモード（Standard / Seikyo / Washizu / FiveTile / EastWest / Yamima）
    pub mode: GameMode,
    /// 供託（誠京麻雀の場代合計）。和了者が回収・流局で持ち越し
    pub pot: i32,
    /// 前局で親が和了したか（= 連荘フラグ。二度ヅモ判定に使う）。
    ///
    /// **注意**: 現状の xmj には「局終了→次局」のループ実装が無いため、
    /// このフラグを更新する本番コードはまだ存在しない。
    /// 外部から win-resolve 時に手動で更新するフラグとして API のみ提供している。
    /// 完全な連荘配線は follow-up Issue で対応予定。
    pub dealer_won_last: bool,
    /// 東西戦（クリア麻雀）のチーム別役クリア進捗。
    ///
    /// `GameMode::EastWest` のときのみ実質的に使用する。
    /// `record_team_yaku` で和了者のチームに役を追加していき、
    /// `east_west_target_yaku()` 全 5 種が揃ったチームが勝利。
    /// EastWest 以外のモードでは初期化はされるが書き込まれない（no-op）。
    pub team_progress: HashMap<Team, HashSet<Yaku>>,
}

impl Game {
    pub fn new(player_names: Vec<String>) -> Self {
        Self::new_with_mode(player_names, GameMode::Standard)
    }

    /// モードを指定してゲームを構築
    pub fn new_with_mode(player_names: Vec<String>, mode: GameMode) -> Self {
        assert!(player_names.len() == 4, "Mahjong requires exactly 4 players");

        let players: Vec<Player> = player_names
            .into_iter()
            .enumerate()
            .map(|(i, name)| {
                let mut player = Player::new(i, name);
                if i == 0 {
                    player.is_dealer = true;
                }
                player
            })
            .collect();

        let mut team_progress = HashMap::new();
        team_progress.insert(Team::East, HashSet::new());
        team_progress.insert(Team::West, HashSet::new());

        let mut game = Self {
            players,
            wall: Vec::new(),
            dora_indicators: Vec::new(),
            current_player: 0,
            round: 1,
            dealer: 0,
            last_discard: None,
            last_discard_hidden: false,
            mode,
            pot: 0,
            dealer_won_last: false,
            team_progress,
        };

        game.initialize_wall();
        game.deal_initial_tiles();
        game
    }

    /// 誠京麻雀の場代を全員から徴収して供託に積む。
    /// Standard モードでは no-op。
    ///
    /// # Arguments
    /// - `amount`: 1 人あたりの場代額。標準は [`SEIKYO_SEAT_FEE`]（1000 点）。
    ///   テストや特殊バリアントで上書きできるよう引数化している。
    ///
    /// # 仕様メモ
    /// 本来は「**各局開始時**」に呼ぶべきだが、現状の xmj には局ループが無く、
    /// `main.rs` ではゲーム起動時に 1 回だけ呼ぶ simplified version になっている。
    /// 局ごとの再徴収配線は follow-up。複数回呼べば素直に pot が累積する設計。
    pub fn collect_seat_fee(&mut self, amount: i32) {
        if self.mode != GameMode::Seikyo {
            return;
        }
        for player in self.players.iter_mut() {
            player.subtract_score(amount);
        }
        self.pot += amount * self.players.len() as i32;
    }

    /// 供託（pot）を winner に渡してリセット。移動した点数を返す。
    pub fn winner_takes_pot(&mut self, winner_idx: usize) -> i32 {
        if winner_idx >= self.players.len() {
            return 0;
        }
        let moved = self.pot;
        if moved > 0 {
            self.players[winner_idx].add_score(moved);
            self.pot = 0;
        }
        moved
    }

    /// 親の二度ヅモ。**2 枚ツモするだけ。打牌は呼び出し側の責務**。
    ///
    /// 誠京麻雀かつ前局親和了（連荘）かつ現在のプレイヤーが親のときのみ、
    /// 山牌から 2 枚連続でツモして親の手牌に追加する。
    /// 戻り値は (1 枚目, 2 枚目) のタプル。山牌が 2 枚未満なら None を返す。
    ///
    /// **注意**: この関数を呼んだ直後、親の手牌は 15 枚（通常 13 + 2 ツモ、
    /// または親初期 14 + 1 ツモから 2 枚追加）になる。和了判定 `tile_count() == 14`
    /// を維持するには、呼び出し側で 1 枚を即捨てる UX を実装する必要がある。
    /// `main.rs` の `handle_player_turn` がそのリファレンス実装。
    pub fn dealer_double_draw(&mut self) -> Option<(Tile, Tile)> {
        if self.mode != GameMode::Seikyo {
            return None;
        }
        if !self.dealer_won_last {
            return None;
        }
        if self.current_player != self.dealer {
            return None;
        }
        if self.wall.len() < 2 {
            return None;
        }
        let first = self.wall.pop()?;
        let second = self.wall.pop()?;
        self.players[self.current_player].draw_tile(first);
        self.players[self.current_player].draw_tile(second);
        Some((first, second))
    }

    fn initialize_wall(&mut self) {
        self.wall.clear();
        
        // 数牌（各4枚）
        for suit in [Suit::Man, Suit::Pin, Suit::Sou] {
            for value in 1..=9 {
                for _ in 0..4 {
                    let is_red = value == 5 && self.wall.iter().filter(|t| 
                        matches!(t.tile_type, TileType::Number { suit: s, value: 5 } if s == suit) && t.is_red
                    ).count() == 0; // 各色5の1枚目のみ赤ドラ
                    
                    self.wall.push(Tile::new_number(suit, value, is_red));
                }
            }
        }

        // 字牌（各4枚）
        for honor in [Honor::Ton, Honor::Nan, Honor::Shaa, Honor::Pei, Honor::Haku, Honor::Hatsu, Honor::Chun] {
            for _ in 0..4 {
                self.wall.push(Tile::new_honor(honor));
            }
        }

        // シャッフル
        self.wall.shuffle(&mut thread_rng());

        // 鷲巣麻雀: 全 136 牌のうちランダムに 3/4 を glass にする
        // 山牌のシャッフル後に 0..(len * 3 / 4) を glass マーク（既にシャッフル済みなので
        // インデックス先頭から塗ってもランダム分布になる）
        if self.mode == GameMode::Washizu {
            let total = self.wall.len();
            let glass_count = total * 3 / 4;
            for i in 0..glass_count {
                self.wall[i].is_glass = true;
            }
        }

        // ドラ表示牌を設定
        if let Some(dora_indicator) = self.wall.pop() {
            self.dora_indicators.push(dora_indicator);
        }
    }

    /// 観測者 (`observer_idx`) から見た対象 (`target_idx`) の手牌のうち、
    /// 視認可能な牌のリストを返す。
    ///
    /// - 自分自身 (`observer_idx == target_idx`) なら手牌すべて
    /// - 他家なら鷲巣ルール: `is_glass == true` の牌のみ
    /// - 非鷲巣モードでも、他家の手牌は通常見えないため空ベクタを返す
    ///   （UI 側は別経路で「自分の手牌のみ表示」しているのでこの API は鷲巣専用想定）
    ///
    /// 副槓子（meld）は鳴いた時点で公開されているので別 API で参照する想定。
    /// ここでは concealed hand の見え方だけ扱う。
    pub fn get_visible_tiles_of_opponent(
        &self,
        observer_idx: usize,
        target_idx: usize,
    ) -> Vec<Tile> {
        if observer_idx >= self.players.len() || target_idx >= self.players.len() {
            return Vec::new();
        }
        let target_hand = self.players[target_idx].hand.get_tiles();
        if observer_idx == target_idx {
            return target_hand.clone();
        }
        match self.mode {
            GameMode::Washizu => target_hand
                .iter()
                .filter(|t| t.is_glass)
                .cloned()
                .collect(),
            // 他モードでは他家の手牌は不可視（空）
            _ => Vec::new(),
        }
    }

    fn deal_initial_tiles(&mut self) {
        // FiveTile モード: 子 5 枚、親 6 枚（「ツモ番が回った」状態でスタート）
        if self.mode == GameMode::FiveTile {
            for player_idx in 0..4 {
                for _ in 0..5 {
                    if let Some(tile) = self.wall.pop() {
                        self.players[player_idx].draw_tile(tile);
                    }
                }
            }
            // 親に追加の 1 枚
            if let Some(tile) = self.wall.pop() {
                self.players[self.dealer].draw_tile(tile);
            }
            return;
        }

        // Standard / Seikyo / Washizu: 親は14枚、子は13枚配る
        for _round in 0..3 {
            for player_idx in 0..4 {
                for _ in 0..4 {
                    if let Some(tile) = self.wall.pop() {
                        self.players[player_idx].draw_tile(tile);
                    }
                }
            }
        }

        // 最後の1枚ずつ
        for player_idx in 0..4 {
            if let Some(tile) = self.wall.pop() {
                self.players[player_idx].draw_tile(tile);
            }
        }

        // 親に追加の1枚
        if let Some(tile) = self.wall.pop() {
            self.players[self.dealer].draw_tile(tile);
        }
    }

    pub fn draw_tile(&mut self) -> Option<Tile> {
        self.wall.pop()
    }

    pub fn current_player_draw(&mut self) -> bool {
        if let Some(tile) = self.draw_tile() {
            self.players[self.current_player].draw_tile(tile);
            true
        } else {
            false
        }
    }

    pub fn discard_tile(&mut self, tile: Tile) -> bool {
        if self.players[self.current_player].discard_tile(tile) {
            self.last_discard = Some(tile);
            self.last_discard_hidden = false;
            self.next_player();
            true
        } else {
            false
        }
    }

    /// 闇牌打牌（Yamima ルール）。`Player::discard_hidden` を呼んで河に裏向きで置く。
    ///
    /// - `GameMode::Yamima` でないと no-op で false
    /// - 現プレイヤーの点数が 1000 未満なら false（`Player::discard_hidden` 内でも検査）
    /// - 手牌に `tile` が無ければ false
    /// - 成功時: 1000 点減 + 河に闇牌追加 + `last_discard_hidden=true` + 次の手番へ進む
    ///
    /// `last_discard` には実体牌をセットするが、`last_discard_hidden` が true の間は
    /// `can_someone_win` / `can_pon` / `can_chi` / `can_kan` が全て false を返す（鳴き不可）。
    /// 照射で公開してもこのフラグは戻らない（鳴きの再開ではなく河の可視化のみ）。
    pub fn discard_hidden_tile(&mut self, tile: Tile) -> bool {
        if self.mode != GameMode::Yamima {
            return false;
        }
        if !self.players[self.current_player].discard_hidden(tile) {
            return false;
        }
        self.last_discard = Some(tile);
        self.last_discard_hidden = true;
        self.next_player();
        true
    }

    /// 照射（Yamima ルール）。観測者が 500 点支払って対象の闇牌を公開させる。
    ///
    /// - `GameMode::Yamima` でないと None
    /// - `observer_idx` / `target_idx` の範囲外なら None
    /// - 観測者の点数が 500 未満なら None
    /// - 対象 discard が既に公開済（is_hidden==false）なら None
    /// - 成功時: 観測者から 500 点減 + 対象の `is_hidden=false` に書き換え + 実体牌を返す
    ///
    /// 仕様メモ: 「照射するだけで必ず公開される」運用。空振り（外れ）はなく、
    /// 失敗するのは「点数不足」「対象 index 不正」「既に公開済」のみ。
    pub fn light_up(
        &mut self,
        observer_idx: usize,
        target_idx: usize,
        discard_idx: usize,
    ) -> Option<Tile> {
        if self.mode != GameMode::Yamima {
            return None;
        }
        if observer_idx >= self.players.len() || target_idx >= self.players.len() {
            return None;
        }
        // 自分で自分の闇牌を照射するのは無効（点棒だけ焼く操作になる）
        if observer_idx == target_idx {
            return None;
        }
        if self.players[observer_idx].score < YAMIMA_LIGHT_UP_COST {
            return None;
        }
        // 先に公開を試みる（失敗したら点数は引かない）
        let revealed = self.players[target_idx].reveal_discard(discard_idx)?;
        self.players[observer_idx].subtract_score(YAMIMA_LIGHT_UP_COST);
        Some(revealed)
    }

    pub fn next_player(&mut self) {
        self.current_player = (self.current_player + 1) % 4;
    }

    pub fn get_current_player(&self) -> &Player {
        &self.players[self.current_player]
    }

    pub fn get_current_player_mut(&mut self) -> &mut Player {
        &mut self.players[self.current_player]
    }

    pub fn can_someone_win(&self, tile: &Tile) -> Vec<usize> {
        // Yamima: `last_discard` が闇牌（裏向き）のときはロン不可。先に照射で公開してから
        // 改めて判定する仕様。引数 `tile` は呼び出し側が `last_discard` と同じ値を渡す前提で
        // 利用するため、`last_discard_hidden` のみで遮断してよい。
        if self.last_discard_hidden {
            return Vec::new();
        }
        let mut winners = Vec::new();
        for (i, player) in self.players.iter().enumerate() {
            if i == self.current_player {
                continue;
            }
            let can_win = match self.mode {
                GameMode::FiveTile => player.can_win_with_mode(tile, GameMode::FiveTile),
                _ => player.can_win(tile),
            };
            if can_win {
                winners.push(i);
            }
        }
        winners
    }

    /// チー可能かチェック（下家のみ）
    pub fn can_chi(&self, player_idx: usize) -> bool {
        if self.last_discard.is_none() {
            return false;
        }
        // Yamima: 闇牌はチー不可
        if self.last_discard_hidden {
            return false;
        }

        // チーは下家（前のプレイヤー）のみ可能
        let prev_player = (self.current_player + 3) % 4;
        if player_idx != prev_player {
            return false;
        }

        let tile = self.last_discard.unwrap();

        // 数牌のみチー可能
        if let TileType::Number { suit, value } = tile.tile_type {
            let hand = &self.players[player_idx].hand;
            let tiles = hand.get_tiles();

            // パターン1: n-2, n-1, n（鳴き牌がn）
            if value >= 3 {
                let t1 = Tile::new_number(suit, value - 2, false);
                let t2 = Tile::new_number(suit, value - 1, false);
                if tiles.contains(&t1) && tiles.contains(&t2) {
                    return true;
                }
            }

            // パターン2: n-1, n, n+1（鳴き牌がn）
            if value >= 2 && value <= 8 {
                let t1 = Tile::new_number(suit, value - 1, false);
                let t2 = Tile::new_number(suit, value + 1, false);
                if tiles.contains(&t1) && tiles.contains(&t2) {
                    return true;
                }
            }

            // パターン3: n, n+1, n+2（鳴き牌がn）
            if value <= 7 {
                let t1 = Tile::new_number(suit, value + 1, false);
                let t2 = Tile::new_number(suit, value + 2, false);
                if tiles.contains(&t1) && tiles.contains(&t2) {
                    return true;
                }
            }
        }

        false
    }

    /// ポン可能かチェック
    pub fn can_pon(&self, player_idx: usize) -> bool {
        if self.last_discard.is_none() || player_idx == self.current_player {
            return false;
        }
        // Yamima: 闇牌はポン不可
        if self.last_discard_hidden {
            return false;
        }

        let tile = self.last_discard.unwrap();
        let hand = &self.players[player_idx].hand;
        let tiles = hand.get_tiles();

        // 同じ牌が2枚以上あればポン可能
        tiles.iter().filter(|&&t| t == tile).count() >= 2
    }

    /// カン可能かチェック（明槓）
    pub fn can_kan(&self, player_idx: usize) -> bool {
        if self.last_discard.is_none() || player_idx == self.current_player {
            return false;
        }
        // Yamima: 闇牌は明槓不可
        if self.last_discard_hidden {
            return false;
        }

        let tile = self.last_discard.unwrap();
        let hand = &self.players[player_idx].hand;
        let tiles = hand.get_tiles();

        // 同じ牌が3枚あれば明槓可能
        tiles.iter().filter(|&&t| t == tile).count() >= 3
    }

    /// 暗槓可能な牌のリストを取得
    pub fn can_ankan(&self, player_idx: usize) -> Vec<Tile> {
        let hand = &self.players[player_idx].hand;
        let tiles = hand.get_tiles();
        let mut ankan_tiles = Vec::new();

        use std::collections::HashMap;
        let mut tile_counts = HashMap::new();
        for tile in tiles {
            *tile_counts.entry(*tile).or_insert(0) += 1;
        }

        for (tile, count) in tile_counts {
            if count >= 4 {
                ankan_tiles.push(tile);
            }
        }

        ankan_tiles
    }

    /// チーを実行
    pub fn do_chi(&mut self, player_idx: usize, pattern: usize) -> bool {
        if !self.can_chi(player_idx) {
            return false;
        }

        let tile = self.last_discard.unwrap();

        if let TileType::Number { suit, value } = tile.tile_type {
            let (t1, t2) = match pattern {
                0 => {
                    // n-2, n-1, n
                    if value < 3 {
                        return false;
                    }
                    (
                        Tile::new_number(suit, value - 2, false),
                        Tile::new_number(suit, value - 1, false),
                    )
                }
                1 => {
                    // n-1, n, n+1
                    if value < 2 || value > 8 {
                        return false;
                    }
                    (
                        Tile::new_number(suit, value - 1, false),
                        Tile::new_number(suit, value + 1, false),
                    )
                }
                2 => {
                    // n, n+1, n+2
                    if value > 7 {
                        return false;
                    }
                    (
                        Tile::new_number(suit, value + 1, false),
                        Tile::new_number(suit, value + 2, false),
                    )
                }
                _ => return false,
            };

            let player = &mut self.players[player_idx];
            if !player.hand.remove_tile(&t1) || !player.hand.remove_tile(&t2) {
                return false;
            }

            let meld = crate::hand::Meld {
                meld_type: crate::hand::MeldType::Chi,
                tiles: vec![t1, tile, t2],
                is_open: true,
            };

            player.hand.add_meld(meld);
            self.last_discard = None;
            self.current_player = player_idx;
            true
        } else {
            false
        }
    }

    /// ポンを実行
    pub fn do_pon(&mut self, player_idx: usize) -> bool {
        if !self.can_pon(player_idx) {
            return false;
        }

        let tile = self.last_discard.unwrap();
        let player = &mut self.players[player_idx];

        // 同じ牌を2枚削除
        if !player.hand.remove_tile(&tile) || !player.hand.remove_tile(&tile) {
            return false;
        }

        let meld = crate::hand::Meld {
            meld_type: crate::hand::MeldType::Pon,
            tiles: vec![tile, tile, tile],
            is_open: true,
        };

        player.hand.add_meld(meld);
        self.last_discard = None;
        self.current_player = player_idx;
        true
    }

    /// 明槓を実行
    pub fn do_kan(&mut self, player_idx: usize) -> bool {
        if !self.can_kan(player_idx) {
            return false;
        }

        let tile = self.last_discard.unwrap();
        let player = &mut self.players[player_idx];

        // 同じ牌を3枚削除
        for _ in 0..3 {
            if !player.hand.remove_tile(&tile) {
                return false;
            }
        }

        let meld = crate::hand::Meld {
            meld_type: crate::hand::MeldType::Kan,
            tiles: vec![tile, tile, tile, tile],
            is_open: true,
        };

        player.hand.add_meld(meld);
        self.last_discard = None;

        // 槓ドラ追加
        if let Some(dora_indicator) = self.wall.pop() {
            self.dora_indicators.push(dora_indicator);
        }

        // 嶺上牌をツモ
        if let Some(rinshan_tile) = self.wall.pop() {
            self.players[player_idx].draw_tile(rinshan_tile);
        }

        self.current_player = player_idx;
        true
    }

    /// 暗槓を実行
    pub fn do_ankan(&mut self, player_idx: usize, tile: Tile) -> bool {
        let player = &mut self.players[player_idx];

        // 同じ牌を4枚削除
        for _ in 0..4 {
            if !player.hand.remove_tile(&tile) {
                return false;
            }
        }

        let meld = crate::hand::Meld {
            meld_type: crate::hand::MeldType::Kan,
            tiles: vec![tile, tile, tile, tile],
            is_open: false,
        };

        player.hand.add_meld(meld);

        // 槓ドラ追加
        if let Some(dora_indicator) = self.wall.pop() {
            self.dora_indicators.push(dora_indicator);
        }

        // 嶺上牌をツモ
        if let Some(rinshan_tile) = self.wall.pop() {
            self.players[player_idx].draw_tile(rinshan_tile);
        }

        true
    }

    /// 東西戦: 和了者のチームに役を 1 件登録する。
    ///
    /// - `GameMode::EastWest` 以外のモードでは安全に no-op（`team_progress` は変更されない）。
    /// - 既に同じ役を登録済みなら no-op（`HashSet` の性質）。
    /// - `winner_seat` の範囲チェックはしない（`team_of` が常に East/West を返すため安全）。
    pub fn record_team_yaku(&mut self, winner_seat: usize, yaku: Yaku) {
        if self.mode != GameMode::EastWest {
            return;
        }
        let team = team_of(winner_seat);
        self.team_progress.entry(team).or_insert_with(HashSet::new).insert(yaku);
    }

    /// 東西戦: 指定チームのクリア進捗を返す。
    ///
    /// `east_west_target_yaku()` の並び順でソートされた `Vec<Yaku>` を返す。
    /// （クリア対象 5 種のうち登録済みのものだけ含める。順序が安定なので CLI 表示と
    /// テスト assert で揺れない。）
    pub fn team_clear_progress(&self, team: Team) -> Vec<Yaku> {
        let empty = HashSet::new();
        let set = self.team_progress.get(&team).unwrap_or(&empty);
        east_west_target_yaku()
            .iter()
            .filter(|y| set.contains(y))
            .cloned()
            .collect()
    }

    /// 東西戦: 指定チームがクリア対象 5 役を全て揃えたか。
    pub fn is_team_cleared(&self, team: Team) -> bool {
        let empty = HashSet::new();
        let set = self.team_progress.get(&team).unwrap_or(&empty);
        east_west_target_yaku().iter().all(|y| set.contains(y))
    }

    /// 東西戦の勝者チーム。両方 cleared の同時成立はゲーム性質上ほぼ起きないが、
    /// 起きた場合は East を優先して返す（決定論的）。
    pub fn east_west_winner(&self) -> Option<Team> {
        if self.is_team_cleared(Team::East) {
            Some(Team::East)
        } else if self.is_team_cleared(Team::West) {
            Some(Team::West)
        } else {
            None
        }
    }

    pub fn is_game_over(&self) -> bool {
        if self.mode == GameMode::EastWest && self.east_west_winner().is_some() {
            return true;
        }
        self.wall.is_empty() || self.players.iter().any(|p| p.score <= 0)
    }

    pub fn get_wall_count(&self) -> usize {
        self.wall.len()
    }

    pub fn get_dora_indicators(&self) -> &Vec<Tile> {
        &self.dora_indicators
    }

    pub fn get_game_state_string(&self) -> String {
        let mut result = String::new();
        
        result.push_str(&format!("Round: {} | Wall: {} tiles\n", self.round, self.wall.len()));
        result.push_str(&format!("Dora indicators: {}\n", 
            self.dora_indicators.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" ")));
        
        for (i, player) in self.players.iter().enumerate() {
            let marker = if i == self.current_player { ">" } else { " " };
            let dealer_mark = if player.is_dealer { "親" } else { " " };
            result.push_str(&format!("{}{} {} ({}点): {}\n",
                marker, dealer_mark, player.name, player.score, player.get_hand_string()));

            // 鷲巣麻雀: 自分以外（player id 0 = 「あなた」視点）の glass 牌を追加表示
            // CLI クライアントは player id 0 を観測者として想定。
            // 自分自身 (i == 0) はすでに手牌全表示済みなのでスキップ
            if self.mode == GameMode::Washizu && i != 0 {
                let glass_tiles = self.get_visible_tiles_of_opponent(0, i);
                let glass_str = if glass_tiles.is_empty() {
                    "（透明牌なし）".to_string()
                } else {
                    glass_tiles
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                result.push_str(&format!("  [{} の透明牌: {}]\n", player.name, glass_str));
            }

            if !player.discards.is_empty() {
                result.push_str(&format!("  河: {}\n", player.get_discards_string()));
            }
        }
        
        if let Some(tile) = self.last_discard {
            result.push_str(&format!("Last discard: {}\n", tile.to_string()));
        }

        // 東西戦のクリア進捗表示
        if self.mode == GameMode::EastWest {
            result.push_str(&format!(
                "東チーム: {}\n",
                format_team_progress_line(self, Team::East)
            ));
            result.push_str(&format!(
                "西チーム: {}\n",
                format_team_progress_line(self, Team::West)
            ));
            if let Some(winner) = self.east_west_winner() {
                let label = match winner {
                    Team::East => "東",
                    Team::West => "西",
                };
                result.push_str(&format!("{}チーム勝利！\n", label));
            }
        }

        result
    }
}

/// 東西戦進捗の 1 行表示。
/// 例: `[✓三色同順, _一気通貫, _対々和, _全帯么, _混老頭]`
fn format_team_progress_line(game: &Game, team: Team) -> String {
    let set: HashSet<Yaku> = game
        .team_clear_progress(team)
        .into_iter()
        .collect();
    let parts: Vec<String> = east_west_target_yaku()
        .iter()
        .map(|y| {
            let mark = if set.contains(y) { "✓" } else { "_" };
            format!("{}{}", mark, yaku_label_ja(y))
        })
        .collect();
    format!("[{}]", parts.join(", "))
}

/// 東西戦クリア対象役の日本語ラベル。
fn yaku_label_ja(y: &Yaku) -> &'static str {
    match y {
        Yaku::SanshokuDoujun => "三色同順",
        Yaku::Ittsu => "一気通貫",
        Yaku::Toitoi => "対々和",
        Yaku::Chanta => "全帯么",
        Yaku::Honroutou => "混老頭",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_creation() {
        let names = vec!["Player1".to_string(), "Player2".to_string(), "Player3".to_string(), "Player4".to_string()];
        let game = Game::new(names);
        
        assert_eq!(game.players.len(), 4);
        assert!(game.players[0].is_dealer);
        assert_eq!(game.players[0].tile_count(), 14); // 親は14枚
        assert_eq!(game.players[1].tile_count(), 13); // 子は13枚
        assert_eq!(game.dora_indicators.len(), 1);
    }

    #[test]
    fn test_tile_draw_and_discard() {
        let names = vec!["P1".to_string(), "P2".to_string(), "P3".to_string(), "P4".to_string()];
        let mut game = Game::new(names);

        let initial_wall_count = game.get_wall_count();
        assert!(game.current_player_draw());
        assert_eq!(game.get_wall_count(), initial_wall_count - 1);

        let _player_tiles = game.get_current_player().get_hand_string();
        let first_tile = game.get_current_player().hand.get_tiles()[0];

        assert!(game.discard_tile(first_tile));
        assert_eq!(game.current_player, 1); // Next player
    }

    fn seikyo_names() -> Vec<String> {
        vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ]
    }

    #[test]
    fn test_standard_mode_pot_is_noop() {
        let mut game = Game::new(seikyo_names());
        assert_eq!(game.mode, GameMode::Standard);
        assert_eq!(game.pot, 0);

        let initial_scores: Vec<i32> = game.players.iter().map(|p| p.score).collect();
        game.collect_seat_fee(SEIKYO_SEAT_FEE);
        let after_scores: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        assert_eq!(game.pot, 0, "Standard モードでは pot が増えない");
        assert_eq!(initial_scores, after_scores, "Standard モードでは点数も動かない");
    }

    #[test]
    fn test_seikyo_collect_seat_fee() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        let initial_scores: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        game.collect_seat_fee(SEIKYO_SEAT_FEE);

        assert_eq!(game.pot, SEIKYO_SEAT_FEE * 4);
        for (i, p) in game.players.iter().enumerate() {
            assert_eq!(p.score, initial_scores[i] - SEIKYO_SEAT_FEE);
        }
    }

    #[test]
    fn test_seikyo_winner_takes_pot() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        game.collect_seat_fee(SEIKYO_SEAT_FEE);
        let pot_before = game.pot;
        let winner_score_before = game.players[1].score;

        let moved = game.winner_takes_pot(1);

        assert_eq!(moved, pot_before);
        assert_eq!(game.pot, 0);
        assert_eq!(game.players[1].score, winner_score_before + pot_before);
    }

    #[test]
    fn test_seikyo_dealer_double_draw() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        // 連荘していなければ二度ヅモは発動しない
        assert!(game.dealer_double_draw().is_none());

        game.dealer_won_last = true;
        game.current_player = game.dealer;
        let hand_before = game.players[game.dealer].tile_count();
        let wall_before = game.wall.len();

        let result = game.dealer_double_draw();
        assert!(result.is_some(), "Seikyo + 連荘 + 親手番なら二度ヅモが成立する");
        let hand_after = game.players[game.dealer].tile_count();
        let wall_after = game.wall.len();
        assert_eq!(hand_after, hand_before + 2, "親の手牌が 2 枚増える");
        assert_eq!(wall_after, wall_before - 2, "山牌が 2 枚減る");
    }

    #[test]
    fn test_seikyo_dealer_double_draw_not_dealer() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        game.dealer_won_last = true;
        game.current_player = (game.dealer + 1) % 4; // 子の手番

        assert!(game.dealer_double_draw().is_none(), "親の手番でなければ発動しない");
    }

    /// pot が 0 のときに `winner_takes_pot` を呼んでも winner の点数は不変
    #[test]
    fn test_winner_takes_empty_pot() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        // collect_seat_fee は呼ばない → pot は 0
        assert_eq!(game.pot, 0);

        let winner_score_before = game.players[2].score;
        let moved = game.winner_takes_pot(2);

        assert_eq!(moved, 0, "pot が空なら 0 が返る");
        assert_eq!(game.players[2].score, winner_score_before, "winner の点数は不変");
        assert_eq!(game.pot, 0);
    }

    /// `collect_seat_fee` を複数回呼ぶと pot が累積する（流局持ち越しのシミュレーション）
    #[test]
    fn test_pot_carries_over_multiple_collections() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);

        for _ in 0..3 {
            game.collect_seat_fee(SEIKYO_SEAT_FEE);
        }

        // 1000 × 4 人 × 3 回 = 12000
        assert_eq!(game.pot, SEIKYO_SEAT_FEE * 4 * 3);
        assert_eq!(game.pot, 12000);

        // 各プレイヤーは初期 25000 - 3000 = 22000
        for p in &game.players {
            assert_eq!(p.score, 25000 - SEIKYO_SEAT_FEE * 3);
        }
    }

    fn washizu_names() -> Vec<String> {
        vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ]
    }

    /// Washizu モードでは wall + 配牌 + dora indicator の合計のうち
    /// **3/4 が glass** になっていること。
    /// シャッフル後に先頭 75% を塗る決定的な整数演算実装なので、毎回ぴったり 102 枚 glass。
    /// exact 比較で固定。将来確率的実装（サンプリング等）に変えたら許容幅を緩める。
    #[test]
    fn test_washizu_wall_has_3_4_glass_tiles() {
        let game = Game::new_with_mode(washizu_names(), GameMode::Washizu);

        let mut total = 0usize;
        let mut glass = 0usize;

        // 山牌
        for t in &game.wall {
            total += 1;
            if t.is_glass {
                glass += 1;
            }
        }
        // 各プレイヤー手牌
        for p in &game.players {
            for t in p.hand.get_tiles() {
                total += 1;
                if t.is_glass {
                    glass += 1;
                }
            }
        }
        // ドラ表示牌
        for t in &game.dora_indicators {
            total += 1;
            if t.is_glass {
                glass += 1;
            }
        }

        assert_eq!(total, 136, "麻雀牌は全 136 枚");
        let expected = 136 * 3 / 4; // 102
        let diff = (glass as i32 - expected as i32).abs();
        assert!(
            diff <= 1,
            "glass 牌は 102 (3/4 of 136) 枚であるべき。実測 {} / 期待 {} (exact 比較)",
            glass,
            expected
        );
    }

    #[test]
    fn test_standard_wall_no_glass_tiles() {
        let game = Game::new(washizu_names());

        for t in &game.wall {
            assert!(!t.is_glass, "Standard モードでは glass が立たない");
        }
        for p in &game.players {
            for t in p.hand.get_tiles() {
                assert!(!t.is_glass, "Standard モードでは手牌にも glass が無い");
            }
        }
        for t in &game.dora_indicators {
            assert!(!t.is_glass, "Standard モードではドラ表示牌にも glass が無い");
        }
    }

    #[test]
    fn test_visible_tiles_self_returns_all() {
        let game = Game::new_with_mode(washizu_names(), GameMode::Washizu);

        for idx in 0..4 {
            let visible = game.get_visible_tiles_of_opponent(idx, idx);
            let own = game.players[idx].hand.get_tiles();
            assert_eq!(
                visible.len(),
                own.len(),
                "自分の手牌は glass/opaque 問わず全部見える (player {})",
                idx
            );
        }
    }

    #[test]
    fn test_visible_tiles_opponent_returns_glass_only() {
        let game = Game::new_with_mode(washizu_names(), GameMode::Washizu);

        // 観測者 0、対象 1
        let visible = game.get_visible_tiles_of_opponent(0, 1);
        let target_all = game.players[1].hand.get_tiles();
        let target_glass_count = target_all.iter().filter(|t| t.is_glass).count();

        assert_eq!(
            visible.len(),
            target_glass_count,
            "他家の glass 牌のみ見える"
        );
        for t in &visible {
            assert!(t.is_glass, "他家からの見えは必ず is_glass=true");
        }
    }

    /// Standard モードでは `get_visible_tiles_of_opponent` は他家に対して空を返す。
    #[test]
    fn test_visible_tiles_standard_mode_opponent_empty() {
        let game = Game::new(washizu_names());
        let visible = game.get_visible_tiles_of_opponent(0, 1);
        assert!(visible.is_empty(), "Standard では他家手牌は不可視");

        // ただし自分自身なら全表示
        let own = game.get_visible_tiles_of_opponent(0, 0);
        assert_eq!(own.len(), game.players[0].hand.get_tiles().len());
    }

    /// 5 枚麻雀の配牌: 子 5 枚 / 親 6 枚
    #[test]
    fn test_five_tile_initial_deal() {
        let names = vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ];
        let game = Game::new_with_mode(names, GameMode::FiveTile);

        assert_eq!(game.players[0].tile_count(), 6, "親（P1）は 6 枚");
        for i in 1..4 {
            assert_eq!(game.players[i].tile_count(), 5, "子（P{}）は 5 枚", i + 1);
        }
        // ドラ表示牌 1 枚は配られている
        assert_eq!(game.dora_indicators.len(), 1);
    }

    /// 二度ヅモ直後は親手牌が 15 枚になり、(1枚目, 2枚目) を返す（打牌は呼び出し側責務）。
    ///
    /// 実機の局ループでは「親が前局打牌した直後 = 手牌 13 枚」状態で次局が始まる想定。
    /// 本テストは Game::new 直後（親 14 枚）から 1 枚捨てて 13 枚にしてから二度ヅモする。
    #[test]
    fn test_dealer_double_draw_returns_two_tiles_and_hand_size_15() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        game.current_player = game.dealer;

        // 親手牌を 14 → 13 にする（1 枚捨てる）
        let first_tile = game.players[game.dealer].hand.get_tiles()[0];
        assert!(game.players[game.dealer].discard_tile(first_tile));
        // discard_tile は current_player を回さないので手動でリセット
        game.current_player = game.dealer;
        assert_eq!(game.players[game.dealer].tile_count(), 13, "親手牌 13 枚に揃える");

        game.dealer_won_last = true;
        let result = game.dealer_double_draw();
        assert!(result.is_some(), "二度ヅモ成立");

        let (_t1, _t2) = result.unwrap();
        let hand_after = game.players[game.dealer].tile_count();
        assert_eq!(hand_after, 15, "親 13 + 2 ツモ = 15 枚（呼び出し側で 1 枚捨てて 14 枚に戻す）");
    }

    // ========================================
    // 東西戦（クリア麻雀）テスト
    // ========================================

    fn east_west_names() -> Vec<String> {
        vec![
            "東家".to_string(),
            "南家".to_string(),
            "西家".to_string(),
            "北家".to_string(),
        ]
    }

    /// 座席 0/2 → East、座席 1/3 → West の対応
    #[test]
    fn test_team_of_seats() {
        assert_eq!(team_of(0), Team::East, "東家(座席0)は East チーム");
        assert_eq!(team_of(2), Team::East, "西家(座席2)は East チーム");
        assert_eq!(team_of(1), Team::West, "南家(座席1)は West チーム");
        assert_eq!(team_of(3), Team::West, "北家(座席3)は West チーム");
    }

    /// 起動直後は両チームとも進捗 0 件
    #[test]
    fn test_east_west_no_yaku_recorded_initially() {
        let game = Game::new_with_mode(east_west_names(), GameMode::EastWest);
        assert!(game.team_clear_progress(Team::East).is_empty());
        assert!(game.team_clear_progress(Team::West).is_empty());
        assert!(!game.is_team_cleared(Team::East));
        assert!(!game.is_team_cleared(Team::West));
        assert_eq!(game.east_west_winner(), None);
    }

    /// 和了者の所属チームに役が登録される
    #[test]
    fn test_record_team_yaku_for_winner() {
        let mut game = Game::new_with_mode(east_west_names(), GameMode::EastWest);

        // 座席 0 (東家) が三色同順で和了 → East チームに追加
        game.record_team_yaku(0, Yaku::SanshokuDoujun);
        assert_eq!(game.team_clear_progress(Team::East), vec![Yaku::SanshokuDoujun]);
        assert!(game.team_clear_progress(Team::West).is_empty());

        // 座席 3 (北家) が一気通貫で和了 → West チームに追加
        game.record_team_yaku(3, Yaku::Ittsu);
        assert_eq!(game.team_clear_progress(Team::West), vec![Yaku::Ittsu]);
    }

    /// 同じ役を 2 回登録しても重複しない（HashSet）
    #[test]
    fn test_team_clear_progress_unique() {
        let mut game = Game::new_with_mode(east_west_names(), GameMode::EastWest);
        game.record_team_yaku(0, Yaku::Toitoi);
        game.record_team_yaku(0, Yaku::Toitoi);
        // 別の East 座席（2 = 西家）で再登録しても同じ
        game.record_team_yaku(2, Yaku::Toitoi);
        assert_eq!(game.team_clear_progress(Team::East), vec![Yaku::Toitoi]);
    }

    /// 5 役全て登録すると is_team_cleared が true
    #[test]
    fn test_is_team_cleared_after_five() {
        let mut game = Game::new_with_mode(east_west_names(), GameMode::EastWest);
        for y in east_west_target_yaku().iter() {
            game.record_team_yaku(0, y.clone());
        }
        assert!(game.is_team_cleared(Team::East));
        assert!(!game.is_team_cleared(Team::West));
    }

    /// どちらか cleared なら east_west_winner = Some
    #[test]
    fn test_east_west_winner_returns_first_cleared() {
        let mut game = Game::new_with_mode(east_west_names(), GameMode::EastWest);
        assert_eq!(game.east_west_winner(), None);

        // West チーム（座席 1 = 南家）が先に揃える
        for y in east_west_target_yaku().iter() {
            game.record_team_yaku(1, y.clone());
        }
        assert_eq!(game.east_west_winner(), Some(Team::West));
        // ゲーム終了判定もトリガーされる
        assert!(game.is_game_over());
    }

    /// EastWest 以外のモードでは record_team_yaku は no-op
    #[test]
    fn test_record_yaku_no_op_in_other_modes() {
        let mut game = Game::new(east_west_names());
        assert_eq!(game.mode, GameMode::Standard);

        for y in east_west_target_yaku().iter() {
            game.record_team_yaku(0, y.clone());
        }
        // Standard モードでは team_progress に書き込まれない
        assert!(game.team_clear_progress(Team::East).is_empty());
        assert!(!game.is_team_cleared(Team::East));
        assert_eq!(game.east_west_winner(), None);
    }

    /// 東西戦のターゲット役は 5 種ちょうど
    #[test]
    fn test_east_west_target_yaku_count() {
        let targets = east_west_target_yaku();
        assert_eq!(targets.len(), 5);
        assert!(targets.contains(&Yaku::SanshokuDoujun));
        assert!(targets.contains(&Yaku::Ittsu));
        assert!(targets.contains(&Yaku::Toitoi));
        assert!(targets.contains(&Yaku::Chanta));
        assert!(targets.contains(&Yaku::Honroutou));
    }

    // ========================================
    // Yamima（闇麻）テスト
    // ========================================

    fn yamima_names() -> Vec<String> {
        vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ]
    }

    /// Yamima モードで闇牌打牌すると last_discard_hidden=true、点数が 1000 減る。
    #[test]
    fn test_discard_hidden_tile_in_yamima() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];
        let score_before = game.players[current].score;

        assert!(game.discard_hidden_tile(tile));
        assert_eq!(game.players[current].score, score_before - YAMIMA_HIDDEN_COST);
        assert!(game.last_discard_hidden, "闇牌打牌後は last_discard_hidden=true");
        assert_eq!(game.last_discard, Some(tile));
        // 次プレイヤーへ進んでいる
        assert_eq!(game.current_player, (current + 1) % 4);
    }

    /// Yamima 以外のモードでは discard_hidden_tile は no-op で false。
    #[test]
    fn test_discard_hidden_tile_no_op_in_other_modes() {
        let mut game = Game::new(yamima_names());
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];

        assert!(!game.discard_hidden_tile(tile), "Standard では闇牌打牌不可");
        assert!(!game.last_discard_hidden);
    }

    /// 通常 discard_tile を呼ぶと last_discard_hidden が false に戻る。
    #[test]
    fn test_discard_tile_clears_hidden_flag() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        // 直接フラグを true にしてから通常打牌
        game.last_discard_hidden = true;
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];

        assert!(game.discard_tile(tile));
        assert!(!game.last_discard_hidden, "通常打牌で false に戻る");
    }

    /// 闇牌に対する can_pon は false（鳴き不可）。
    #[test]
    fn test_can_pon_returns_false_on_hidden_discard() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        // 強制的に「他家が手番、闇牌打牌直後、相手にポンできる牌がある」状態を作る
        let tile = Tile::new_number(Suit::Man, 5, false);
        // player 1 の手牌に同じ牌 2 枚を入れる
        game.players[1].hand = crate::hand::Hand::new();
        game.players[1].hand.add_tile(tile);
        game.players[1].hand.add_tile(tile);
        game.last_discard = Some(tile);
        game.last_discard_hidden = true;
        game.current_player = 0;

        assert!(!game.can_pon(1), "闇牌はポン不可");

        // フラグを下ろせばポン可能になる（リグレッション防止）
        game.last_discard_hidden = false;
        assert!(game.can_pon(1), "通常打牌ならポン可能");
    }

    /// 闇牌に対する can_kan / can_chi も false。
    #[test]
    fn test_can_kan_and_chi_return_false_on_hidden_discard() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let tile = Tile::new_number(Suit::Pin, 5, false);
        game.players[1].hand = crate::hand::Hand::new();
        for _ in 0..3 {
            game.players[1].hand.add_tile(tile);
        }
        // チー対象も仕込む（下家パターン: 4p, 6p を持つ）
        game.players[3].hand = crate::hand::Hand::new();
        game.players[3].hand.add_tile(Tile::new_number(Suit::Pin, 4, false));
        game.players[3].hand.add_tile(Tile::new_number(Suit::Pin, 6, false));

        game.last_discard = Some(tile);
        game.last_discard_hidden = true;
        game.current_player = 0;

        assert!(!game.can_kan(1), "闇牌は明槓不可");
        // current_player=0 の下家 = 3
        assert!(!game.can_chi(3), "闇牌はチー不可");
    }

    /// 闇牌に対する can_someone_win は常に空ベクタ（ロン不可）。
    #[test]
    fn test_can_someone_win_returns_empty_on_hidden_discard() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let tile = Tile::new_number(Suit::Sou, 1, false);
        game.last_discard = Some(tile);
        game.last_discard_hidden = true;
        game.current_player = 0;

        assert!(
            game.can_someone_win(&tile).is_empty(),
            "闇牌に対してロン宣言は不可（先に照射が必要）"
        );
    }

    /// 照射で 500 点支払い、対象の闇牌が公開されて tile が返る。
    #[test]
    fn test_light_up_reveals_and_costs_500() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];
        assert!(game.discard_hidden_tile(tile)); // current が闇牌打牌
        // current は元の current_player、闇牌は players[current].discards[0]
        let observer = (current + 1) % 4; // 次プレイヤー（current_player になっている）
        let score_before = game.players[observer].score;

        let revealed = game.light_up(observer, current, 0);
        assert_eq!(revealed, Some(tile));
        assert_eq!(
            game.players[observer].score,
            score_before - YAMIMA_LIGHT_UP_COST,
            "観測者から 500 点徴収"
        );
        assert!(
            !game.players[current].discards[0].is_hidden,
            "対象の河が公開される"
        );
    }

    /// 既に公開済の河を照射しても None（無効）。点数も引かない。
    #[test]
    fn test_light_up_fails_on_already_revealed() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];
        assert!(game.discard_tile(tile)); // 通常打牌（公開状態）
        let observer = (current + 1) % 4;
        let score_before = game.players[observer].score;

        assert_eq!(
            game.light_up(observer, current, 0),
            None,
            "公開済は照射対象外"
        );
        assert_eq!(
            game.players[observer].score, score_before,
            "失敗時は点数を引かない"
        );
    }

    /// 観測者の点数が 500 未満なら照射不可（None）。
    #[test]
    fn test_light_up_fails_when_observer_score_below_500() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];
        assert!(game.discard_hidden_tile(tile));
        let observer = (current + 1) % 4;
        game.players[observer].score = 499;

        assert_eq!(game.light_up(observer, current, 0), None);
        assert_eq!(game.players[observer].score, 499, "点数据え置き");
        assert!(
            game.players[current].discards[0].is_hidden,
            "対象は闇牌のまま"
        );
    }

    /// Yamima 以外では light_up は no-op で None。
    #[test]
    fn test_light_up_no_op_in_other_modes() {
        let mut game = Game::new(yamima_names());
        // 強引に Discard を仕込む（is_hidden=true のものを置く）
        use crate::player::Discard;
        let tile = Tile::new_number(Suit::Man, 1, false);
        game.players[0].discards.push(Discard { tile, is_hidden: true });

        assert_eq!(game.light_up(1, 0, 0), None, "Standard では照射不可");
    }
}
