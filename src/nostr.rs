use serde::{Serialize, Deserialize};
use std::collections::HashMap;

/// Nostr関連の機能（基本構造のみ、実際のNostr SDKは後で統合）
///
/// 現在は依存関係を最小限に保つため、基本的なデータ構造のみを定義

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NostrKeys {
    pub public_key: String,
    pub private_key: String,
}

impl NostrKeys {
    /// 新しい鍵ペアを生成（簡易実装）
    pub fn generate() -> Self {
        // TODO: nostr-sdkを使った実際の鍵生成
        // 現在は仮のプレースホルダー
        Self {
            public_key: "npub_placeholder".to_string(),
            private_key: "nsec_placeholder".to_string(),
        }
    }

    /// ローカルストレージから鍵を読み込む
    pub fn load() -> Option<Self> {
        // TODO: ファイルシステムからの読み込み実装
        None
    }

    /// ローカルストレージに鍵を保存
    pub fn save(&self) -> Result<(), String> {
        // TODO: ファイルシステムへの保存実装
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// Nostrクライアント（基本構造）
pub struct NostrClient {
    keys: NostrKeys,
    relay_url: String,
}

impl NostrClient {
    pub fn new(relay_url: String) -> Self {
        let keys = NostrKeys::load().unwrap_or_else(|| {
            let new_keys = NostrKeys::generate();
            let _ = new_keys.save();
            new_keys
        });

        Self { keys, relay_url }
    }

    /// 対戦募集イベントを送信
    pub async fn seek_match(&self, game_mode: &str, player_count: usize) -> Result<String, String> {
        // TODO: 実際のNostr イベント送信実装
        println!("対戦募集: mode={}, players={}", game_mode, player_count);
        Ok("event_id_placeholder".to_string())
    }

    /// 参加応答イベントを送信
    pub async fn join_match(&self, seek_event_id: &str) -> Result<String, String> {
        // TODO: 実際のNostr イベント送信実装
        println!("参加応答: event_id={}", seek_event_id);
        Ok("event_id_placeholder".to_string())
    }

    /// ゲームイベントを購読
    pub async fn subscribe_game_events(&self, game_id: &str) -> Result<(), String> {
        // TODO: 実際のNostr購読実装
        println!("ゲームイベント購読: game_id={}", game_id);
        Ok(())
    }

    /// ゲームイベントを送信
    pub async fn publish_game_event(&self, event: GameEvent) -> Result<String, String> {
        // TODO: 実際のNostr イベント送信実装
        println!("ゲームイベント送信: {:?}", event);
        Ok("event_id_placeholder".to_string())
    }

    pub fn public_key(&self) -> &str {
        &self.keys.public_key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keys_generation() {
        let keys = NostrKeys::generate();
        assert!(!keys.public_key.is_empty());
        assert!(!keys.private_key.is_empty());
    }

    #[test]
    fn test_game_event_creation() {
        let event = GameEvent::new(
            GameEventType::DrawTile,
            "game123".to_string(),
            "player1".to_string(),
        )
        .with_data("tile".to_string(), "1m".to_string());

        assert_eq!(event.game_id, "game123");
        assert!(event.data.contains_key("tile"));
    }

    #[test]
    fn test_nostr_client_creation() {
        let client = NostrClient::new("ws://localhost:7000".to_string());
        assert!(!client.public_key().is_empty());
    }
}
