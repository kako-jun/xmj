use crate::tile::Tile;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Hand {
    tiles: Vec<Tile>,
    melds: Vec<Meld>,
}

#[derive(Debug, Clone)]
pub enum MeldType {
    Chi,    // 順子 (チー)
    Pon,    // 刻子 (ポン)
    Kan,    // 槓子 (カン)
}

#[derive(Debug, Clone)]
pub struct Meld {
    pub meld_type: MeldType,
    pub tiles: Vec<Tile>,
    pub is_open: bool,
}

impl Hand {
    pub fn new() -> Self {
        Self {
            tiles: Vec::new(),
            melds: Vec::new(),
        }
    }

    pub fn add_tile(&mut self, tile: Tile) {
        self.tiles.push(tile);
        self.sort();
    }

    pub fn remove_tile(&mut self, tile: &Tile) -> bool {
        if let Some(pos) = self.tiles.iter().position(|t| t == tile) {
            self.tiles.remove(pos);
            true
        } else {
            false
        }
    }

    pub fn get_tiles(&self) -> &Vec<Tile> {
        &self.tiles
    }

    pub fn get_melds(&self) -> &Vec<Meld> {
        &self.melds
    }

    pub fn add_meld(&mut self, meld: Meld) {
        for tile in &meld.tiles {
            self.remove_tile(tile);
        }
        self.melds.push(meld);
    }

    pub fn tile_count(&self) -> usize {
        self.tiles.len() + self.melds.len() * 3
    }

    pub fn is_tenpai(&self) -> bool {
        // 簡単なテンパイ判定（完全な実装は後で行う）
        self.tile_count() == 13
    }

    pub fn can_win(&self, winning_tile: &Tile) -> bool {
        let mut test_tiles = self.tiles.clone();
        test_tiles.push(*winning_tile);
        self.is_winning_hand(&test_tiles)
    }

    fn is_winning_hand(&self, tiles: &[Tile]) -> bool {
        // 基本的な和了形チェック（4面子1雀頭）
        if tiles.len() + self.melds.len() * 3 != 14 {
            return false;
        }

        // 簡単な実装: 同じ牌2枚以上があるかチェック（雀頭候補）
        let mut counts = HashMap::new();
        for tile in tiles {
            *counts.entry(*tile).or_insert(0) += 1;
        }

        // 2枚以上の牌があるかチェック
        counts.values().any(|&count| count >= 2)
    }

    fn sort(&mut self) {
        self.tiles.sort_by(|a, b| {
            use crate::tile::{TileType, Suit, Honor};
            
            match (&a.tile_type, &b.tile_type) {
                (TileType::Number { suit: s1, value: v1 }, TileType::Number { suit: s2, value: v2 }) => {
                    let suit_order = |s: &Suit| match s {
                        Suit::Man => 0,
                        Suit::Pin => 1,
                        Suit::Sou => 2,
                    };
                    suit_order(s1).cmp(&suit_order(s2)).then(v1.cmp(v2))
                }
                (TileType::Honor(h1), TileType::Honor(h2)) => {
                    let honor_order = |h: &Honor| match h {
                        Honor::Ton => 0,
                        Honor::Nan => 1,
                        Honor::Shaa => 2,
                        Honor::Pei => 3,
                        Honor::Haku => 4,
                        Honor::Hatsu => 5,
                        Honor::Chun => 6,
                    };
                    honor_order(h1).cmp(&honor_order(h2))
                }
                (TileType::Number { .. }, TileType::Honor(_)) => std::cmp::Ordering::Less,
                (TileType::Honor(_), TileType::Number { .. }) => std::cmp::Ordering::Greater,
            }
        });
    }

    pub fn to_string(&self) -> String {
        let mut result = String::new();
        
        // 手牌
        for tile in &self.tiles {
            result.push_str(&tile.to_string());
            result.push(' ');
        }

        // 副露
        if !self.melds.is_empty() {
            result.push_str("| ");
            for meld in &self.melds {
                for tile in &meld.tiles {
                    result.push_str(&tile.to_string());
                }
                result.push(' ');
            }
        }

        result.trim().to_string()
    }
}

impl Default for Hand {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{Tile, Suit, Honor};

    #[test]
    fn test_hand_operations() {
        let mut hand = Hand::new();
        let tile = Tile::new_number(Suit::Man, 1, false);
        
        hand.add_tile(tile);
        assert_eq!(hand.tile_count(), 1);
        
        assert!(hand.remove_tile(&tile));
        assert_eq!(hand.tile_count(), 0);
        assert!(!hand.remove_tile(&tile));
    }

    #[test]
    fn test_hand_sorting() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_honor(Honor::Ton));
        hand.add_tile(Tile::new_number(Suit::Man, 9, false));
        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        
        let tiles = hand.get_tiles();
        assert_eq!(tiles[0].to_string(), "1m");
        assert_eq!(tiles[1].to_string(), "9m");
        assert_eq!(tiles[2].to_string(), "5p");
        assert_eq!(tiles[3].to_string(), "to");
    }
}