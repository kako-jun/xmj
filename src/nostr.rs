use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

/// Nostr鍵ペアの管理
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrKeys {
    pub public_key: String,
    pub private_key: String,
}

impl NostrKeys {
    /// 新しい鍵ペアを生成
    pub fn generate() -> Self {
        // 簡易的な鍵生成（本番環境ではnostr-sdkを使用）
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let private_key = format!("nsec_{:x}", timestamp);
        let public_key = format!("npub_{:x}", timestamp);

        Self {
            public_key,
            private_key,
        }
    }

    /// 鍵の保存先パスを取得
    fn get_keys_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".xmj");
        path.push("nostr_keys.json");
        path
    }

    /// ローカルストレージから鍵を読み込む
    pub fn load() -> Option<Self> {
        let path = Self::get_keys_path();
        if !path.exists() {
            return None;
        }

        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// ローカルストレージに鍵を保存
    pub fn save(&self) -> Result<(), String> {
        let path = Self::get_keys_path();

        // ディレクトリを作成
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        fs::write(path, json).map_err(|e| e.to_string())?;

        Ok(())
    }

    /// 鍵を削除
    pub fn delete() -> Result<(), String> {
        let path = Self::get_keys_path();
        if path.exists() {
            fs::remove_file(path).map_err(|e| e.to_string())?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GameEventType {
    SeekingMatch,      // 対戦募集
    JoinRequest,       // 参加応答
    MatchEstablished,  // マッチング成立
    GameStart,         // ゲーム開始
    DrawTile,          // ツモ
    DiscardTile,       // 打牌
    Chi,               // チー
    Pon,               // ポン
    Kan,               // カン
    Riichi,            // リーチ
    Ron,               // ロン
    Tsumo,             // ツモ和了
    GameEnd,           // ゲーム終了
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameEvent {
    pub event_type: GameEventType,
    pub game_id: String,
    pub player_id: String,
    pub data: HashMap<String, String>,
    pub timestamp: u64,
}

impl GameEvent {
    pub fn new(event_type: GameEventType, game_id: String, player_id: String) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            event_type,
            game_id,
            player_id,
            data: HashMap::new(),
            timestamp,
        }
    }

    pub fn with_data(mut self, key: String, value: String) -> Self {
        self.data.insert(key, value);
        self
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string(self).map_err(|e| e.to_string())
    }

    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json).map_err(|e| e.to_string())
    }
}

/// マッチング状態
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchState {
    pub match_id: String,
    pub game_mode: String,
    pub player_count: usize,
    pub joined_players: Vec<String>,
    pub is_ready: bool,
}

impl MatchState {
    pub fn new(match_id: String, game_mode: String, player_count: usize) -> Self {
        Self {
            match_id,
            game_mode,
            player_count,
            joined_players: Vec::new(),
            is_ready: false,
        }
    }

    pub fn add_player(&mut self, player_id: String) -> bool {
        if self.joined_players.contains(&player_id) {
            return false;
        }

        self.joined_players.push(player_id);

        if self.joined_players.len() >= self.player_count {
            self.is_ready = true;
        }

        true
    }

    pub fn is_full(&self) -> bool {
        self.joined_players.len() >= self.player_count
    }
}

/// Nostrクライアント
pub struct NostrClient {
    keys: NostrKeys,
    relay_url: String,
    active_matches: HashMap<String, MatchState>,
}

impl NostrClient {
    pub fn new(relay_url: String) -> Self {
        let keys = NostrKeys::load().unwrap_or_else(|| {
            let new_keys = NostrKeys::generate();
            let _ = new_keys.save();
            new_keys
        });

        Self {
            keys,
            relay_url,
            active_matches: HashMap::new(),
        }
    }

    pub fn with_keys(keys: NostrKeys, relay_url: String) -> Self {
        Self {
            keys,
            relay_url,
            active_matches: HashMap::new(),
        }
    }

    /// 対戦募集イベントを送信
    pub fn seek_match(&mut self, game_mode: &str, player_count: usize) -> Result<String, String> {
        let match_id = format!("match_{}", uuid::Uuid::new_v4());

        let match_state = MatchState::new(
            match_id.clone(),
            game_mode.to_string(),
            player_count,
        );

        self.active_matches.insert(match_id.clone(), match_state);

        println!("対戦募集開始: mode={}, players={}, match_id={}",
            game_mode, player_count, match_id);

        Ok(match_id)
    }

    /// 参加応答イベントを送信
    pub fn join_match(&mut self, match_id: &str) -> Result<(), String> {
        let player_id = self.keys.public_key.clone();

        if let Some(match_state) = self.active_matches.get_mut(match_id) {
            if match_state.add_player(player_id.clone()) {
                println!("マッチ参加: match_id={}, player={}", match_id, player_id);

                if match_state.is_ready {
                    println!("マッチング成立！ゲーム開始");
                }

                Ok(())
            } else {
                Err("既に参加しています".to_string())
            }
        } else {
            Err("マッチが見つかりません".to_string())
        }
    }

    /// アクティブなマッチング一覧を取得
    pub fn get_active_matches(&self) -> Vec<&MatchState> {
        self.active_matches
            .values()
            .filter(|m| !m.is_ready)
            .collect()
    }

    /// ゲームイベントを送信
    pub fn publish_game_event(&self, event: GameEvent) -> Result<String, String> {
        let json = event.to_json()?;
        println!("ゲームイベント送信: {:?}", event.event_type);
        println!("  Data: {}", json);
        Ok("event_id_placeholder".to_string())
    }

    pub fn public_key(&self) -> &str {
        &self.keys.public_key
    }

    pub fn relay_url(&self) -> &str {
        &self.relay_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_keys_generation() {
        let keys = NostrKeys::generate();
        assert!(!keys.public_key.is_empty());
        assert!(!keys.private_key.is_empty());
        assert!(keys.public_key.starts_with("npub_"));
        assert!(keys.private_key.starts_with("nsec_"));
    }

    #[test]
    fn test_keys_save_and_load() {
        // テスト用の鍵を生成
        let keys = NostrKeys::generate();

        // 保存
        assert!(keys.save().is_ok());

        // 読み込み
        let loaded = NostrKeys::load();
        assert!(loaded.is_some());

        let loaded_keys = loaded.unwrap();
        assert_eq!(keys.public_key, loaded_keys.public_key);
        assert_eq!(keys.private_key, loaded_keys.private_key);

        // クリーンアップ
        let _ = NostrKeys::delete();
    }

    #[test]
    fn test_game_event_serialization() {
        let event = GameEvent::new(
            GameEventType::DrawTile,
            "game123".to_string(),
            "player1".to_string(),
        )
        .with_data("tile".to_string(), "1m".to_string());

        let json = event.to_json().unwrap();
        assert!(json.contains("DrawTile"));

        let parsed = GameEvent::from_json(&json).unwrap();
        assert_eq!(parsed.event_type, GameEventType::DrawTile);
        assert_eq!(parsed.game_id, "game123");
    }

    #[test]
    fn test_match_state() {
        let mut match_state = MatchState::new(
            "match1".to_string(),
            "normal".to_string(),
            4,
        );

        assert!(!match_state.is_ready);
        assert!(match_state.add_player("player1".to_string()));
        assert!(match_state.add_player("player2".to_string()));
        assert!(match_state.add_player("player3".to_string()));
        assert!(!match_state.is_ready);

        assert!(match_state.add_player("player4".to_string()));
        assert!(match_state.is_ready);
        assert!(match_state.is_full());
    }

    #[test]
    fn test_nostr_client_matching() {
        let mut client = NostrClient::new("ws://localhost:7000".to_string());

        let match_id = client.seek_match("normal", 4).unwrap();
        assert!(!match_id.is_empty());

        let matches = client.get_active_matches();
        assert_eq!(matches.len(), 1);

        assert!(client.join_match(&match_id).is_ok());
    }
}
