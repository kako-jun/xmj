//! WASM用Nostrクライアント（ブラウザ環境特化）

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;
#[cfg(feature = "wasm")]
use web_sys::{window, Storage};
#[cfg(feature = "wasm")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "wasm")]
use uuid::Uuid;

/// Nostr鍵ペア（ブラウザのlocalStorageに保存）
#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmNostrKeys {
    pub public_key: String,
    pub secret_key: String,
}

#[cfg(feature = "wasm")]
impl WasmNostrKeys {
    /// 新しい鍵ペアを生成（実際の暗号化は簡略化版）
    pub fn generate() -> Self {
        // 本来はsecp256k1で生成するが、WASMでは簡略化
        let secret = Uuid::new_v4().to_string();
        let public = format!("npub{}", Uuid::new_v4().to_string().replace("-", ""));

        Self {
            public_key: public,
            secret_key: secret,
        }
    }

    /// ブラウザのlocalStorageから読み込み
    pub fn load() -> Result<Self, String> {
        let storage = Self::get_storage()?;

        let public_key = storage
            .get_item("xmj_nostr_public")
            .map_err(|_| "Failed to read public key".to_string())?
            .ok_or("No public key found".to_string())?;

        let secret_key = storage
            .get_item("xmj_nostr_secret")
            .map_err(|_| "Failed to read secret key".to_string())?
            .ok_or("No secret key found".to_string())?;

        Ok(Self {
            public_key,
            secret_key,
        })
    }

    /// ブラウザのlocalStorageに保存
    pub fn save(&self) -> Result<(), String> {
        let storage = Self::get_storage()?;

        storage
            .set_item("xmj_nostr_public", &self.public_key)
            .map_err(|_| "Failed to save public key".to_string())?;

        storage
            .set_item("xmj_nostr_secret", &self.secret_key)
            .map_err(|_| "Failed to save secret key".to_string())?;

        Ok(())
    }

    /// ブラウザのlocalStorageから削除
    pub fn delete() -> Result<(), String> {
        let storage = Self::get_storage()?;

        storage
            .remove_item("xmj_nostr_public")
            .map_err(|_| "Failed to delete public key".to_string())?;

        storage
            .remove_item("xmj_nostr_secret")
            .map_err(|_| "Failed to delete secret key".to_string())?;

        Ok(())
    }

    /// localStorage取得ヘルパー
    fn get_storage() -> Result<Storage, String> {
        window()
            .ok_or("No window object".to_string())?
            .local_storage()
            .map_err(|_| "Failed to get localStorage".to_string())?
            .ok_or("localStorage not available".to_string())
    }
}

/// マッチング状態
#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmMatchState {
    pub match_id: String,
    pub host_pubkey: String,
    pub players: Vec<String>,
    pub max_players: usize,
    pub status: String,
}

#[cfg(feature = "wasm")]
impl WasmMatchState {
    pub fn new(host_pubkey: String, max_players: usize) -> Self {
        Self {
            match_id: Uuid::new_v4().to_string(),
            host_pubkey,
            players: Vec::new(),
            max_players,
            status: "waiting".to_string(),
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

/// ゲームイベントタイプ
#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WasmGameEventType {
    MatchSeek,
    MatchJoin,
    MatchStart,
    DrawTile,
    DiscardTile,
    Chi,
    Pon,
    Kan,
    Riichi,
    Ron,
    Tsumo,
    GameEnd,
}

/// ゲームイベント
#[cfg(feature = "wasm")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmGameEvent {
    pub event_type: WasmGameEventType,
    pub match_id: String,
    pub player_id: String,
    pub data: String,
    pub timestamp: u64,
}

#[cfg(feature = "wasm")]
impl WasmGameEvent {
    pub fn new(event_type: WasmGameEventType, match_id: String, player_id: String, data: String) -> Self {
        Self {
            event_type,
            match_id,
            player_id,
            data,
            timestamp: js_sys::Date::now() as u64,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

/// WASM Nostrクライアント
#[cfg(feature = "wasm")]
pub struct WasmNostrClient {
    pub keys: WasmNostrKeys,
    pub relay_url: String,
}

#[cfg(feature = "wasm")]
impl WasmNostrClient {
    pub fn new(relay_url: String) -> Result<Self, String> {
        let keys = match WasmNostrKeys::load() {
            Ok(keys) => keys,
            Err(_) => {
                let keys = WasmNostrKeys::generate();
                keys.save()?;
                keys
            }
        };

        Ok(Self { keys, relay_url })
    }

    pub fn get_public_key(&self) -> String {
        self.keys.public_key.clone()
    }

    /// マッチング募集イベントを作成
    pub fn create_match_seek_event(&self, max_players: usize) -> WasmGameEvent {
        let match_state = WasmMatchState::new(self.keys.public_key.clone(), max_players);
        WasmGameEvent::new(
            WasmGameEventType::MatchSeek,
            match_state.match_id.clone(),
            self.keys.public_key.clone(),
            match_state.to_json(),
        )
    }

    /// マッチング参加イベントを作成
    pub fn create_match_join_event(&self, match_id: String) -> WasmGameEvent {
        WasmGameEvent::new(
            WasmGameEventType::MatchJoin,
            match_id,
            self.keys.public_key.clone(),
            String::new(),
        )
    }

    /// ゲームイベントを作成
    pub fn create_game_event(
        &self,
        event_type: WasmGameEventType,
        match_id: String,
        data: String,
    ) -> WasmGameEvent {
        WasmGameEvent::new(event_type, match_id, self.keys.public_key.clone(), data)
    }
}
