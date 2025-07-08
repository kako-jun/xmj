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