use crate::hand::Hand;
use crate::tile::Tile;

#[derive(Debug, Clone)]
pub struct Player {
    pub id: usize,
    pub name: String,
    pub hand: Hand,
    pub score: i32,
    pub is_dealer: bool,
    pub discards: Vec<Tile>,
    pub is_riichi: bool,
    pub riichi_turn: Option<usize>, // リーチ宣言したターン
    pub ippatsu: bool,               // 一発フラグ
    pub double_riichi: bool,         // ダブル立直
}

impl Player {
    pub fn new(id: usize, name: String) -> Self {
        Self {
            id,
            name,
            hand: Hand::new(),
            score: 25000, // 初期点数
            is_dealer: false,
            discards: Vec::new(),
            is_riichi: false,
            riichi_turn: None,
            ippatsu: false,
            double_riichi: false,
        }
    }

    pub fn draw_tile(&mut self, tile: Tile) {
        self.hand.add_tile(tile);
    }

    pub fn discard_tile(&mut self, tile: Tile) -> bool {
        if self.hand.remove_tile(&tile) {
            self.discards.push(tile);
            true
        } else {
            false
        }
    }

    pub fn can_win(&self, tile: &Tile) -> bool {
        self.hand.can_win(tile)
    }

    pub fn get_hand_string(&self) -> String {
        self.hand.to_string()
    }

    pub fn get_discards_string(&self) -> String {
        self.discards
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn tile_count(&self) -> usize {
        self.hand.tile_count()
    }

    pub fn is_tenpai(&self) -> bool {
        self.hand.is_tenpai()
    }

    pub fn add_score(&mut self, points: i32) {
        self.score += points;
    }

    pub fn subtract_score(&mut self, points: i32) {
        self.score -= points;
        if self.score < 0 {
            self.score = 0;
        }
    }

    /// リーチ可能かチェック
    pub fn can_riichi(&self) -> bool {
        // 門前（副露なし）
        if !self.hand.get_melds().is_empty() {
            return false;
        }

        // テンパイ
        if !self.is_tenpai() {
            return false;
        }

        // 1000点以上
        if self.score < 1000 {
            return false;
        }

        // 既にリーチしていない
        !self.is_riichi
    }

    /// リーチを宣言
    pub fn declare_riichi(&mut self, turn: usize) -> bool {
        if !self.can_riichi() {
            return false;
        }

        self.is_riichi = true;
        self.riichi_turn = Some(turn);
        self.ippatsu = true;

        // 供託1000点を支払う
        self.subtract_score(1000);

        true
    }

    /// 一発フラグを消す（鳴きがあった場合など）
    pub fn clear_ippatsu(&mut self) {
        self.ippatsu = false;
    }

    /// リーチ後の打牌チェック（ツモ切りのみ）
    pub fn can_discard_after_riichi(&self, tile: &Tile) -> bool {
        if !self.is_riichi {
            return true; // リーチしていない場合は制限なし
        }

        // リーチ後は最後にツモった牌のみ打牌可能
        // 簡易実装: 手牌の最後の牌のみ打牌可能とする
        let tiles = self.hand.get_tiles();
        if let Some(last_tile) = tiles.last() {
            last_tile == tile
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{Tile, Suit};

    #[test]
    fn test_player_creation() {
        let player = Player::new(0, "Test Player".to_string());
        assert_eq!(player.id, 0);
        assert_eq!(player.name, "Test Player");
        assert_eq!(player.score, 25000);
        assert!(!player.is_dealer);
        assert_eq!(player.tile_count(), 0);
    }

    #[test]
    fn test_draw_and_discard() {
        let mut player = Player::new(0, "Test".to_string());
        let tile = Tile::new_number(Suit::Man, 1, false);
        
        player.draw_tile(tile);
        assert_eq!(player.tile_count(), 1);
        
        assert!(player.discard_tile(tile));
        assert_eq!(player.tile_count(), 0);
        assert_eq!(player.discards.len(), 1);
    }
}