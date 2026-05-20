//! WebAssembly bindings for xmj麻雀ゲーム

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::{Game, Tile, Hand, Player, AiEngine, AiLevel};
#[cfg(feature = "wasm")]
use crate::game::{RoundOutcome, WinKind};
#[cfg(feature = "wasm")]
use crate::scoring::{ScoringEngine, ScoringResult};

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmGame {
    game: Game,
    human_player_index: Option<usize>, // ハイブリッドモード用：人間プレイヤーの位置（0-3）
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new(player_names: Vec<String>) -> Self {
        // パニック時にコンソールにログを出力
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        Self {
            game: Game::new(player_names),
            human_player_index: None, // 通常モードは全員人間
        }
    }

    /// ハイブリッドゲームを作成（1人間 + 3CPU）
    #[wasm_bindgen(js_name = newHybrid)]
    pub fn new_hybrid(human_name: String, human_position: usize) -> Self {
        // パニック時にコンソールにログを出力
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        let mut names = vec![
            "CPU 東".to_string(),
            "CPU 南".to_string(),
            "CPU 西".to_string(),
            "CPU 北".to_string(),
        ];

        // 人間プレイヤーの位置を設定（0=東, 1=南, 2=西, 3=北）
        let position = human_position % 4;
        names[position] = human_name;

        let game = Game::new(names);

        Self {
            game,
            human_player_index: Some(position),
        }
    }

    /// 現在のプレイヤーが人間かどうか
    #[wasm_bindgen(js_name = isCurrentPlayerHuman)]
    pub fn is_current_player_human(&self) -> bool {
        match self.human_player_index {
            Some(human_idx) => self.game.current_player == human_idx,
            None => true, // ハイブリッドモードでない場合は全員人間扱い
        }
    }

    /// 現在のプレイヤーが牌をツモする
    #[wasm_bindgen(js_name = drawTile)]
    pub fn draw_tile(&mut self) -> bool {
        self.game.current_player_draw()
    }

    /// 牌を打牌する
    #[wasm_bindgen(js_name = discardTile)]
    pub fn discard_tile(&mut self, tile_str: &str) -> bool {
        if let Some(tile) = Tile::from_string(tile_str) {
            self.game.discard_tile(tile)
        } else {
            false
        }
    }

    /// ゲーム状態を取得（JSON文字列）
    #[wasm_bindgen(js_name = getGameState)]
    pub fn get_game_state(&self) -> String {
        self.game.get_game_state_string()
    }

    /// 現在のプレイヤーの手牌を取得
    #[wasm_bindgen(js_name = getCurrentHand)]
    pub fn get_current_hand(&self) -> String {
        self.game.get_current_player().get_hand_string()
    }

    /// 現在のプレイヤーのシャンテン数を取得
    #[wasm_bindgen(js_name = getShanten)]
    pub fn get_shanten(&self) -> i32 {
        self.game.get_current_player().hand.shanten()
    }

    /// 山牌の残り枚数を取得
    #[wasm_bindgen(js_name = getWallCount)]
    pub fn get_wall_count(&self) -> usize {
        self.game.get_wall_count()
    }

    /// ゲームが終了したかチェック
    #[wasm_bindgen(js_name = isGameOver)]
    pub fn is_game_over(&self) -> bool {
        self.game.is_game_over()
    }

    /// 現在のプレイヤーIDを取得
    #[wasm_bindgen(js_name = getCurrentPlayerId)]
    pub fn get_current_player_id(&self) -> usize {
        self.game.current_player
    }

    /// チー可能かチェック
    #[wasm_bindgen(js_name = canChi)]
    pub fn can_chi(&self, player_idx: usize) -> bool {
        self.game.can_chi(player_idx)
    }

    /// ポン可能かチェック
    #[wasm_bindgen(js_name = canPon)]
    pub fn can_pon(&self, player_idx: usize) -> bool {
        self.game.can_pon(player_idx)
    }

    /// カン可能かチェック
    #[wasm_bindgen(js_name = canKan)]
    pub fn can_kan(&self, player_idx: usize) -> bool {
        self.game.can_kan(player_idx)
    }

    /// チーを実行
    #[wasm_bindgen(js_name = doChi)]
    pub fn do_chi(&mut self, player_idx: usize, pattern: usize) -> bool {
        self.game.do_chi(player_idx, pattern)
    }

    /// ポンを実行
    #[wasm_bindgen(js_name = doPon)]
    pub fn do_pon(&mut self, player_idx: usize) -> bool {
        self.game.do_pon(player_idx)
    }

    /// カンを実行
    #[wasm_bindgen(js_name = doKan)]
    pub fn do_kan(&mut self, player_idx: usize) -> bool {
        self.game.do_kan(player_idx)
    }

    /// CPU（AI）のターンを実行
    #[wasm_bindgen(js_name = executeCpuTurn)]
    pub fn execute_cpu_turn(&mut self) -> String {
        // ツモ
        if !self.game.current_player_draw() {
            return "山牌がありません".to_string();
        }

        // AIで打牌選択
        let ai = AiEngine::new(AiLevel::Intermediate);
        let hand = &self.game.get_current_player().hand;

        if let Some(discard_tile) = ai.select_discard(hand) {
            let tile_str = discard_tile.to_string();
            self.game.discard_tile(discard_tile);
            tile_str
        } else {
            "打牌できません".to_string()
        }
    }

    /// 現在のプレイヤーがCPUかどうか
    #[wasm_bindgen(js_name = isCurrentPlayerCpu)]
    pub fn is_current_player_cpu(&self) -> bool {
        match self.human_player_index {
            Some(human_idx) => self.game.current_player != human_idx,
            None => self.game.current_player != 0, // 通常モード（後方互換性）
        }
    }

    /// プレイヤーの点数を取得
    #[wasm_bindgen(js_name = getPlayerScore)]
    pub fn get_player_score(&self, player_idx: usize) -> i32 {
        if player_idx < self.game.players.len() {
            self.game.players[player_idx].score
        } else {
            0
        }
    }

    /// プレイヤー名を取得
    #[wasm_bindgen(js_name = getPlayerName)]
    pub fn get_player_name(&self, player_idx: usize) -> String {
        if player_idx < self.game.players.len() {
            self.game.players[player_idx].name.clone()
        } else {
            "Unknown".to_string()
        }
    }

    /// ドラ表示牌を取得
    #[wasm_bindgen(js_name = getDoraIndicators)]
    pub fn get_dora_indicators(&self) -> String {
        self.game
            .get_dora_indicators()
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// プレイヤーの河（捨て牌）を取得
    #[wasm_bindgen(js_name = getPlayerDiscards)]
    pub fn get_player_discards(&self, player_idx: usize) -> String {
        if player_idx < self.game.players.len() {
            self.game.players[player_idx].get_discards_string()
        } else {
            String::new()
        }
    }

    /// リーチ可能かチェック
    #[wasm_bindgen(js_name = canRiichi)]
    pub fn can_riichi(&self) -> bool {
        self.game.get_current_player().can_riichi()
    }

    /// リーチを宣言
    #[wasm_bindgen(js_name = declareRiichi)]
    pub fn declare_riichi(&mut self) -> bool {
        let current_idx = self.game.current_player;
        self.game.players[current_idx].declare_riichi(self.game.round as usize)
    }

    /// プレイヤーがリーチしているかチェック
    #[wasm_bindgen(js_name = isPlayerRiichi)]
    pub fn is_player_riichi(&self, player_idx: usize) -> bool {
        if player_idx < self.game.players.len() {
            self.game.players[player_idx].is_riichi
        } else {
            false
        }
    }

    // ==================== Round loop (Issue #27) ====================
    //
    // 局結着 (`resolve_win` / `resolve_draw`) 〜 次局遷移 (`next_round`) を
    // Web UI から駆動するための bridge。`get_last_outcome_json` は UI が
    // 直前局の結果画面（和了 / 流局）を描画するための JSON を返す。
    //
    // TODO(#28): 役満ご祝儀 (Seikyo モード) の配線は本 Issue 範囲外。
    //   ScoringEngine 側で yakuman を検出したあと `pay_yakuman_tip` を呼ぶ
    //   経路はここでは張らない。
    // TODO(#29): 東西戦 (EastWest) の team_yaku 進捗更新も本 Issue 範囲外。

    /// 山牌 0 / 全員ノーテンの簡易流局。
    /// 呼び出し側がテンパイ者の座席 index を渡す。
    /// テンパイ者の自動算出は `compute_tenpai_players` を併用する。
    #[wasm_bindgen(js_name = resolveDraw)]
    pub fn resolve_draw(&mut self, tenpai_player_indices: Vec<usize>) {
        self.game.resolve_draw(tenpai_player_indices);
    }

    /// 流局時のテンパイ者の座席 index を全プレイヤーから抽出する。
    /// `Player::is_tenpai()` で判定し、テンパイしているプレイヤーの index 配列を返す。
    /// `resolve_draw` に渡してノーテン罰符を正しく徴収するための補助 API。
    #[wasm_bindgen(js_name = computeTenpaiPlayers)]
    pub fn compute_tenpai_players(&self) -> Vec<usize> {
        self.game
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_tenpai())
            .map(|(i, _)| i)
            .collect()
    }

    /// ツモ和了を確定する。`ScoringResult` は本関数内で `ScoringEngine` に計算させる。
    ///
    /// winning_tile の決定:
    ///   `Hand` は `add_tile` 時に自動ソートされるため「最後に引いた牌」を
    ///   末尾位置から復元できない。代わりに「手牌から 1 枚抜いて
    ///   `can_win(その牌)` が true になる牌」を winning_tile とみなす。
    ///   候補が複数あっても点数計算には大きく影響しないため最初の候補を採用。
    ///
    /// 戻り値: 計算できた `ScoringResult` のサマリ JSON
    ///   `{ "han": n, "fu": n, "totalPoints": n, "yaku": [...] }`
    ///   和了形でなければ "" を返す（呼び出し側の安全網）。
    #[wasm_bindgen(js_name = resolveWinTsumo)]
    pub fn resolve_win_tsumo(&mut self, winner_idx: usize) -> String {
        if winner_idx >= self.game.players.len() {
            return String::new();
        }
        let is_dealer = winner_idx == self.game.dealer;
        let hand_clone = self.game.players[winner_idx].hand.clone();
        let Some((sub_hand, winning_tile)) = extract_agari(&hand_clone) else {
            return String::new();
        };
        let Some(result) = ScoringEngine::calculate_score(&sub_hand, &winning_tile, true, is_dealer) else {
            return String::new();
        };
        let summary = scoring_summary_json(&result);
        self.game
            .resolve_win(winner_idx, WinKind::Tsumo, result);
        summary
    }

    /// ロン和了を確定する。打牌者は `from_idx` で指定。
    /// winning_tile は `game.last_discard` を使用する。
    #[wasm_bindgen(js_name = resolveWinRon)]
    pub fn resolve_win_ron(&mut self, winner_idx: usize, from_idx: usize) -> String {
        if winner_idx >= self.game.players.len() || from_idx >= self.game.players.len() {
            return String::new();
        }
        let Some(winning_tile) = self.game.last_discard else {
            return String::new();
        };
        let is_dealer = winner_idx == self.game.dealer;
        let hand = &self.game.players[winner_idx].hand;
        if !hand.can_win(&winning_tile) {
            return String::new();
        }
        let Some(result) = ScoringEngine::calculate_score(hand, &winning_tile, false, is_dealer) else {
            return String::new();
        };
        let summary = scoring_summary_json(&result);
        self.game
            .resolve_win(winner_idx, WinKind::Ron { from: from_idx }, result);
        summary
    }

    /// 次の局へ。戻り値: true = 続行 / false = 対局終了。
    #[wasm_bindgen(js_name = nextRound)]
    pub fn next_round(&mut self) -> bool {
        self.game.next_round()
    }

    #[wasm_bindgen(js_name = getRound)]
    pub fn get_round(&self) -> u32 {
        self.game.round
    }

    #[wasm_bindgen(js_name = getHonba)]
    pub fn get_honba(&self) -> u32 {
        self.game.honba
    }

    #[wasm_bindgen(js_name = getDealer)]
    pub fn get_dealer(&self) -> usize {
        self.game.dealer
    }

    #[wasm_bindgen(js_name = getRiichiSticks)]
    pub fn get_riichi_sticks(&self) -> u32 {
        self.game.riichi_sticks
    }

    /// 直前局の結果。
    /// - 和了: `{"kind":"win","winner":idx,"winType":"tsumo"|"ron","from":idx?,"han":n,"fu":n,"totalPoints":n,"yaku":[...]}`
    /// - 流局: `{"kind":"draw","tenpaiPlayers":[...]}`
    /// - 未確定 (`last_outcome.is_none()`): "" を返す。
    #[wasm_bindgen(js_name = getLastOutcomeJson)]
    pub fn get_last_outcome_json(&self) -> String {
        match &self.game.last_outcome {
            None => String::new(),
            Some(RoundOutcome::Win { winner, kind, result }) => {
                let (win_type, from) = match kind {
                    WinKind::Tsumo => ("tsumo", None),
                    WinKind::Ron { from } => ("ron", Some(*from)),
                };
                let mut obj = serde_json::Map::new();
                obj.insert("kind".into(), serde_json::Value::String("win".into()));
                obj.insert("winner".into(), serde_json::Value::Number((*winner).into()));
                obj.insert("winType".into(), serde_json::Value::String(win_type.into()));
                if let Some(f) = from {
                    obj.insert("from".into(), serde_json::Value::Number(f.into()));
                }
                obj.insert("han".into(), serde_json::Value::Number(result.han.into()));
                obj.insert("fu".into(), serde_json::Value::Number(result.fu.into()));
                obj.insert(
                    "totalPoints".into(),
                    serde_json::Value::Number(result.total_points.into()),
                );
                obj.insert(
                    "yaku".into(),
                    serde_json::Value::Array(
                        result
                            .yaku
                            .iter()
                            .map(|y| serde_json::Value::String(format!("{:?}", y)))
                            .collect(),
                    ),
                );
                serde_json::Value::Object(obj).to_string()
            }
            Some(RoundOutcome::Draw { tenpai_players }) => {
                let mut obj = serde_json::Map::new();
                obj.insert("kind".into(), serde_json::Value::String("draw".into()));
                obj.insert(
                    "tenpaiPlayers".into(),
                    serde_json::Value::Array(
                        tenpai_players
                            .iter()
                            .map(|i| serde_json::Value::Number((*i).into()))
                            .collect(),
                    ),
                );
                serde_json::Value::Object(obj).to_string()
            }
        }
    }
}

#[cfg(feature = "wasm")]
fn scoring_summary_json(result: &ScoringResult) -> String {
    let mut obj = serde_json::Map::new();
    obj.insert("han".into(), serde_json::Value::Number(result.han.into()));
    obj.insert("fu".into(), serde_json::Value::Number(result.fu.into()));
    obj.insert(
        "totalPoints".into(),
        serde_json::Value::Number(result.total_points.into()),
    );
    obj.insert(
        "yaku".into(),
        serde_json::Value::Array(
            result
                .yaku
                .iter()
                .map(|y| serde_json::Value::String(format!("{:?}", y)))
                .collect(),
        ),
    );
    serde_json::Value::Object(obj).to_string()
}

/// 14 枚手牌から「抜くと残りが winning 形になる 1 枚」を探す。
///
/// ツモ和了直後は `Hand` がソート済で「最後に引いた牌」を末尾から復元できない。
/// 代わりに各ユニークな牌を winning_tile 候補として試し、
/// 残り 13 枚の手牌に対して `can_win(候補)` が成立するものを返す。
///
/// 返り値: `(13 枚に縮めた Hand, winning_tile)`
///
/// # 既知の制限
/// - 単騎 / 嵌張 / 両面など待ち形により符が変わる場合があるが、本実装は
///   最初に発見した winning_tile 候補を採用する近似である (Issue #34)。
///   ScoringEngine が待ち形を正しく評価できるようになるまでの暫定対応。
///
/// # 副露 (チー / ポン / カン) を含む手 (Issue #33)
/// `Hand::tile_count()` は副露込みで 14 枚相当をカウントするため、副露ありでも
/// 残り手牌 (11/8/5/2 枚) から 1 枚抜いて `can_win` を呼べば「副露面子 + 残り手牌」で
/// 4 面子 1 雀頭が構成されるかが正しく判定される (`Hand::check_normal_win` が
/// `melds_needed = 4 - melds.len()` で副露考慮済み)。
#[cfg(feature = "wasm")]
pub(crate) fn extract_agari(hand: &Hand) -> Option<(Hand, Tile)> {
    let tiles = hand.get_tiles().clone();
    // メルド込みで 14 枚相当か確認
    if hand.tile_count() != 14 {
        return None;
    }
    // ユニークな牌で 1 枚ずつ試す
    let mut seen: Vec<Tile> = Vec::new();
    for tile in tiles.iter() {
        if seen.iter().any(|t| t == tile) {
            continue;
        }
        seen.push(*tile);
        let mut sub = hand.clone();
        if sub.remove_tile(tile) && sub.can_win(tile) {
            return Some((sub, *tile));
        }
    }
    None
}

// ==================== Nostr P2P機能 ====================

#[cfg(feature = "wasm")]
use crate::wasm_nostr::{WasmNostrKeys, WasmNostrClient, WasmMatchState, WasmGameEvent, WasmGameEventType};

/// Nostr鍵管理
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmNostrKeyManager {}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmNostrKeyManager {
    /// 新しい鍵を生成して保存
    #[wasm_bindgen(js_name = generateAndSave)]
    pub fn generate_and_save() -> Result<String, String> {
        let keys = WasmNostrKeys::generate();
        let pubkey = keys.public_key.clone();
        keys.save()?;
        Ok(pubkey)
    }

    /// 保存された鍵を読み込み
    #[wasm_bindgen(js_name = loadKeys)]
    pub fn load_keys() -> Result<String, String> {
        let keys = WasmNostrKeys::load()?;
        Ok(keys.public_key)
    }

    /// 鍵を削除
    #[wasm_bindgen(js_name = deleteKeys)]
    pub fn delete_keys() -> Result<(), String> {
        WasmNostrKeys::delete()
    }

    /// 鍵が保存されているかチェック
    #[wasm_bindgen(js_name = hasKeys)]
    pub fn has_keys() -> bool {
        WasmNostrKeys::load().is_ok()
    }
}

/// Nostrクライアント
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmNostrP2PClient {
    client: WasmNostrClient,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmNostrP2PClient {
    /// 新しいクライアントを作成
    #[wasm_bindgen(constructor)]
    pub fn new(relay_url: String) -> Result<WasmNostrP2PClient, String> {
        let client = WasmNostrClient::new(relay_url)?;
        Ok(Self { client })
    }

    /// 公開鍵を取得
    #[wasm_bindgen(js_name = getPublicKey)]
    pub fn get_public_key(&self) -> String {
        self.client.get_public_key()
    }

    /// マッチング募集イベントを作成
    #[wasm_bindgen(js_name = createMatchSeekEvent)]
    pub fn create_match_seek_event(&self, max_players: usize) -> String {
        let event = self.client.create_match_seek_event(max_players);
        event.to_json()
    }

    /// マッチング参加イベントを作成
    #[wasm_bindgen(js_name = createMatchJoinEvent)]
    pub fn create_match_join_event(&self, match_id: String) -> String {
        let event = self.client.create_match_join_event(match_id);
        event.to_json()
    }

    /// 打牌イベントを作成
    #[wasm_bindgen(js_name = createDiscardEvent)]
    pub fn create_discard_event(&self, match_id: String, tile: String) -> String {
        let event = self.client.create_game_event(
            WasmGameEventType::DiscardTile,
            match_id,
            tile,
        );
        event.to_json()
    }

    /// リーチイベントを作成
    #[wasm_bindgen(js_name = createRiichiEvent)]
    pub fn create_riichi_event(&self, match_id: String) -> String {
        let event = self.client.create_game_event(
            WasmGameEventType::Riichi,
            match_id,
            String::new(),
        );
        event.to_json()
    }

    /// ロンイベントを作成
    #[wasm_bindgen(js_name = createRonEvent)]
    pub fn create_ron_event(&self, match_id: String) -> String {
        let event = self.client.create_game_event(
            WasmGameEventType::Ron,
            match_id,
            String::new(),
        );
        event.to_json()
    }

    /// ツモイベントを作成
    #[wasm_bindgen(js_name = createTsumoEvent)]
    pub fn create_tsumo_event(&self, match_id: String) -> String {
        let event = self.client.create_game_event(
            WasmGameEventType::Tsumo,
            match_id,
            String::new(),
        );
        event.to_json()
    }
}

// ==================== WebRTC P2P通信 ====================

#[cfg(feature = "wasm")]
use crate::wasm_webrtc::{WasmWebRtcManager, SignalingData};
#[cfg(feature = "wasm")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "wasm")]
use std::collections::HashMap;

/// WebRTCマネージャーのラッパー
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmWebRtcP2PManager {
    manager: Arc<Mutex<WasmWebRtcManager>>,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmWebRtcP2PManager {
    /// 新しいマネージャーを作成
    #[wasm_bindgen(constructor)]
    pub fn new(local_id: String) -> Self {
        let manager = WasmWebRtcManager::new(local_id);
        Self {
            manager: Arc::new(Mutex::new(manager)),
        }
    }

    /// ピア接続を作成
    #[wasm_bindgen(js_name = createPeerConnection)]
    pub fn create_peer_connection(&self, peer_id: String) -> Result<(), String> {
        let mut manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.create_peer_connection(peer_id)
    }

    /// データチャネルを作成
    #[wasm_bindgen(js_name = createDataChannel)]
    pub fn create_data_channel(&self, peer_id: String, label: String) -> Result<(), String> {
        let mut manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.create_data_channel(peer_id, &label)
    }

    /// オファーを作成（非同期）
    /// JavaScript側でawaitして使用する
    #[wasm_bindgen(js_name = createOffer)]
    pub async fn create_offer(&self, peer_id: String) -> Result<String, String> {
        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        let sdp = manager.create_offer(&peer_id).await?;
        let offer_data = SignalingData::Offer { sdp };
        Ok(offer_data.to_json())
    }

    /// アンサーを作成（非同期）
    #[wasm_bindgen(js_name = createAnswer)]
    pub async fn create_answer(&self, peer_id: String, offer_json: String) -> Result<String, String> {
        let offer_data: SignalingData = SignalingData::from_json(&offer_json)?;

        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;

        if let SignalingData::Offer { sdp } = offer_data {
            let answer_sdp = manager.create_answer(&peer_id, &sdp).await?;
            let answer_data = SignalingData::Answer { sdp: answer_sdp };
            Ok(answer_data.to_json())
        } else {
            Err("Invalid offer data".to_string())
        }
    }

    /// アンサーを設定（非同期）
    #[wasm_bindgen(js_name = setAnswer)]
    pub async fn set_answer(&self, peer_id: String, answer_json: String) -> Result<(), String> {
        let answer_data: SignalingData = SignalingData::from_json(&answer_json)?;

        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;

        if let SignalingData::Answer { sdp } = answer_data {
            manager.set_answer(&peer_id, &sdp).await
        } else {
            Err("Invalid answer data".to_string())
        }
    }

    /// ICE候補を追加（非同期）
    #[wasm_bindgen(js_name = addIceCandidate)]
    pub async fn add_ice_candidate(&self, peer_id: String, candidate_json: String) -> Result<(), String> {
        let candidate_data: SignalingData = SignalingData::from_json(&candidate_json)?;

        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;

        if let SignalingData::IceCandidate { candidate, sdp_mid, sdp_m_line_index } = candidate_data {
            manager.add_ice_candidate(&peer_id, &candidate, &sdp_mid, sdp_m_line_index).await
        } else {
            Err("Invalid ICE candidate data".to_string())
        }
    }

    /// データを送信
    #[wasm_bindgen(js_name = sendData)]
    pub fn send_data(&self, peer_id: String, data: String) -> Result<(), String> {
        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.send_data(&peer_id, &data)
    }

    /// 全ピアに送信
    #[wasm_bindgen(js_name = broadcast)]
    pub fn broadcast(&self, data: String) -> Result<(), String> {
        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.broadcast(&data)
    }

    /// 接続を閉じる
    #[wasm_bindgen(js_name = closeConnection)]
    pub fn close_connection(&self, peer_id: String) -> Result<(), String> {
        let mut manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.close_connection(&peer_id)
    }

    /// 全接続を閉じる
    #[wasm_bindgen(js_name = closeAll)]
    pub fn close_all(&self) -> Result<(), String> {
        let mut manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.close_all();
        Ok(())
    }
}

/// バージョン情報を返す
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// ゲーム名を返す
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = gameName)]
pub fn game_name() -> String {
    "邪雀 Xtreme Mahjong".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "wasm")]
    fn test_wasm_game() {
        let player_names = vec![
            "Player1".to_string(),
            "Player2".to_string(),
            "Player3".to_string(),
            "Player4".to_string(),
        ];

        let mut wasm_game = WasmGame::new(player_names);

        assert!(!wasm_game.is_game_over());
        assert_eq!(wasm_game.get_current_player_id(), 0);

        let state = wasm_game.get_game_state();
        assert!(!state.is_empty());
    }

    // ==================== Round loop bridge tests (Issue #27) ====================

    fn make_game() -> WasmGame {
        WasmGame::new(vec![
            "P1".into(),
            "P2".into(),
            "P3".into(),
            "P4".into(),
        ])
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn round_getters_initial_values() {
        let g = make_game();
        assert_eq!(g.get_round(), 1);
        assert_eq!(g.get_honba(), 0);
        assert_eq!(g.get_dealer(), 0);
        assert_eq!(g.get_riichi_sticks(), 0);
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn last_outcome_empty_initially() {
        let g = make_game();
        assert_eq!(g.get_last_outcome_json(), "");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_draw_writes_outcome_json() {
        let mut g = make_game();
        g.resolve_draw(vec![0, 2]);
        let json = g.get_last_outcome_json();
        assert!(json.contains("\"kind\":\"draw\""), "got {}", json);
        assert!(json.contains("\"tenpaiPlayers\":[0,2]"), "got {}", json);
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_tsumo_non_winning_hand_returns_empty() {
        // 配牌直後の親 (13 枚) は 14 枚ではないので extract_agari が失敗する
        let mut g = make_game();
        let s = g.resolve_win_tsumo(0);
        assert_eq!(s, "");
        // last_outcome も書かれていない
        assert_eq!(g.get_last_outcome_json(), "");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn compute_tenpai_players_classifies_hands() {
        // 配牌直後は player.hand が 13 枚ランダムに埋まる。
        // テンパイ判定の本物の挙動はゲーム進行で発生する。ここでは
        // 「player 0 だけ強制的にテンパイ手 (1-9m + 1-3p + 1z 雀頭候補) を持たせる」
        // 形で挙動を直接確認する。
        use crate::tile::{Tile, Suit, Honor};
        let mut g = make_game();
        // player 0 を七対子テンパイ (6 対子 + 1 単騎) にする
        let tenpai_tiles = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_honor(Honor::Ton),
        ];
        g.game.players[0].hand = crate::Hand::new();
        for t in tenpai_tiles {
            g.game.players[0].hand.add_tile(t);
        }
        // player 1 を完全ノーテン (バラバラ)
        let noten_tiles = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Sou, 5, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_honor(Honor::Ton),
        ];
        g.game.players[1].hand = crate::Hand::new();
        for t in noten_tiles {
            g.game.players[1].hand.add_tile(t);
        }
        let tenpai = g.compute_tenpai_players();
        assert!(tenpai.contains(&0), "player0 should be tenpai: got {:?}", tenpai);
        assert!(!tenpai.contains(&1), "player1 should be noten: got {:?}", tenpai);
    }

    // ==================== Issue #33: extract_agari 副露込みテスト ====================

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_returns_none_for_incomplete_hand() {
        // 13 枚（聴牌前）では和了確定できない
        use crate::tile::{Tile, Suit};
        let mut hand = crate::Hand::new();
        for _ in 0..13 {
            hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        }
        assert!(extract_agari(&hand).is_none());
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_succeeds_with_pon_meld() {
        // ポン 1m1m1m + 残り手牌 11 枚 (= 14 枚相当) で和了形
        use crate::tile::{Tile, Suit};
        use crate::hand::{Meld, MeldType};
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Pin, 8, false),
            Tile::new_number(Suit::Pin, 9, false),
            Tile::new_number(Suit::Sou, 6, false),
            Tile::new_number(Suit::Sou, 6, false),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Pon,
            tiles: vec![
                Tile::new_number(Suit::Man, 1, false),
                Tile::new_number(Suit::Man, 1, false),
                Tile::new_number(Suit::Man, 1, false),
            ],
            is_open: true,
        });
        let result = extract_agari(&hand);
        assert!(result.is_some(), "ポン込み和了形は extract_agari が成立");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_succeeds_with_chi_meld() {
        // チー 4m5m6m + 残り手牌 11 枚
        use crate::tile::{Tile, Suit};
        use crate::hand::{Meld, MeldType};
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 5, false),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Chi,
            tiles: vec![
                Tile::new_number(Suit::Man, 4, false),
                Tile::new_number(Suit::Man, 5, false),
                Tile::new_number(Suit::Man, 6, false),
            ],
            is_open: true,
        });
        let result = extract_agari(&hand);
        assert!(result.is_some(), "チー込み和了形は extract_agari が成立");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_succeeds_with_kan_meld() {
        // 明槓 9m + 残り手牌 11 枚 (Hand::tile_count では槓も +3 扱い)
        use crate::tile::{Tile, Suit, Honor};
        use crate::hand::{Meld, MeldType};
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Pin, 1, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Sou, 4, false),
            Tile::new_number(Suit::Sou, 5, false),
            Tile::new_number(Suit::Sou, 6, false),
            Tile::new_honor(Honor::Haku),
            Tile::new_honor(Honor::Haku),
            Tile::new_honor(Honor::Haku),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Ton),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Kan,
            tiles: vec![
                Tile::new_number(Suit::Man, 9, false),
                Tile::new_number(Suit::Man, 9, false),
                Tile::new_number(Suit::Man, 9, false),
                Tile::new_number(Suit::Man, 9, false),
            ],
            is_open: true,
        });
        let result = extract_agari(&hand);
        assert!(result.is_some(), "明槓込み和了形は extract_agari が成立");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_fails_when_remaining_not_winning() {
        // 副露あり、残り手牌バラバラ → 和了不成立
        use crate::tile::{Tile, Suit, Honor};
        use crate::hand::{Meld, MeldType};
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 6, false),
            Tile::new_number(Suit::Man, 8, false),
            Tile::new_number(Suit::Pin, 1, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Man, 5, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Nan),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Pon,
            tiles: vec![
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
            ],
            is_open: true,
        });
        assert!(extract_agari(&hand).is_none(), "副露あり・バラバラ手は不成立");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_tsumo_succeeds_with_meld() {
        // 副露あり手で resolve_win_tsumo が成功する (空文字でない結果) ことの確認。
        // 役牌白ポン + 1m1m1m暗刻 + 2p3p4p順子 + 7s8s9s順子 + 雀頭東 を作る。
        use crate::tile::{Tile, Suit, Honor};
        use crate::hand::{Meld, MeldType};
        let mut g = make_game();
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Ton),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Pon,
            tiles: vec![
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
            ],
            is_open: true,
        });
        g.game.players[0].hand = hand;
        let s = g.resolve_win_tsumo(0);
        assert!(!s.is_empty(), "副露あり和了で resolve_win_tsumo が成功するはず: got {:?}", s);
        // 役牌 (白) が含まれる
        assert!(s.contains("Yakuhai") || s.contains("\"han\""), "yaku/han 情報が含まれる: {}", s);
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn next_round_progresses_or_ends() {
        let mut g = make_game();
        // 子和了相当: dealer_won_last=false にして親流れさせる
        g.resolve_draw(vec![]); // 全員ノーテン → dealer_won_last=false
        let cont = g.next_round();
        // 東風戦の途中なので continue するはず
        assert!(cont);
        assert_eq!(g.get_round(), 2);
        assert_eq!(g.get_dealer(), 1);
    }
}
