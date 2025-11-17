//! WebAssembly bindings for xmj麻雀ゲーム

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::{Game, Tile, Hand, Player, AiEngine, AiLevel};

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmGame {
    game: Game,
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
        self.game.current_player != 0
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
}
