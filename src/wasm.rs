//! WebAssembly bindings for xmj麻雀ゲーム

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::{Game, Tile, Hand, Player};

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
