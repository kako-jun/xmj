use crate::player::Player;
use crate::tile::{Tile, TileType, Suit, Honor};
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Debug, Clone)]
pub struct Game {
    pub players: Vec<Player>,
    pub wall: Vec<Tile>,
    pub dora_indicators: Vec<Tile>,
    pub current_player: usize,
    pub round: u32,
    pub dealer: usize,
    pub last_discard: Option<Tile>,
}

impl Game {
    pub fn new(player_names: Vec<String>) -> Self {
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

        let mut game = Self {
            players,
            wall: Vec::new(),
            dora_indicators: Vec::new(),
            current_player: 0,
            round: 1,
            dealer: 0,
            last_discard: None,
        };

        game.initialize_wall();
        game.deal_initial_tiles();
        game
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
        
        // ドラ表示牌を設定
        if let Some(dora_indicator) = self.wall.pop() {
            self.dora_indicators.push(dora_indicator);
        }
    }

    fn deal_initial_tiles(&mut self) {
        // 親は14枚、子は13枚配る
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
            self.next_player();
            true
        } else {
            false
        }
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
        let mut winners = Vec::new();
        for (i, player) in self.players.iter().enumerate() {
            if i != self.current_player && player.can_win(tile) {
                winners.push(i);
            }
        }
        winners
    }

    pub fn is_game_over(&self) -> bool {
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
            
            if !player.discards.is_empty() {
                result.push_str(&format!("  河: {}\n", player.get_discards_string()));
            }
        }
        
        if let Some(tile) = self.last_discard {
            result.push_str(&format!("Last discard: {}\n", tile.to_string()));
        }
        
        result
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
}