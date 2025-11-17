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
        self.shanten() == 0
    }

    pub fn can_win(&self, winning_tile: &Tile) -> bool {
        let mut test_tiles = self.tiles.clone();
        test_tiles.push(*winning_tile);
        self.is_winning_hand(&test_tiles)
    }

    fn is_winning_hand(&self, tiles: &[Tile]) -> bool {
        // 手牌総数チェック
        let total_tiles = tiles.len() + self.melds.len() * 3;
        if total_tiles != 14 {
            return false;
        }

        // 既に副露がある場合は通常形のみ（七対子・国士無双は不可）
        if !self.melds.is_empty() {
            return self.check_normal_win(tiles);
        }

        // 七対子チェック
        if self.check_chitoi(tiles) {
            return true;
        }

        // 国士無双チェック
        if self.check_kokushi(tiles) {
            return true;
        }

        // 通常形（4面子1雀頭）チェック
        self.check_normal_win(tiles)
    }

    /// 通常形（4面子1雀頭）の判定
    fn check_normal_win(&self, tiles: &[Tile]) -> bool {
        let mut tile_map = Self::create_tile_map(tiles);
        let melds_needed = 4 - self.melds.len();

        // 雀頭候補を探す
        for tile in tiles {
            if tile_map.get(tile).copied().unwrap_or(0) >= 2 {
                // 雀頭として2枚取り除く
                *tile_map.get_mut(tile).unwrap() -= 2;

                // 残りで必要な面子が作れるかチェック
                if self.check_melds(&tile_map, melds_needed) {
                    return true;
                }

                // 戻す
                *tile_map.get_mut(tile).unwrap() += 2;
            }
        }

        false
    }

    /// 面子（順子・刻子）を作れるかチェック（再帰）
    fn check_melds(&self, tile_map: &HashMap<Tile, usize>, melds_needed: usize) -> bool {
        if melds_needed == 0 {
            // 全ての牌が使われているかチェック
            return tile_map.values().all(|&count| count == 0);
        }

        // 残り牌から最小の牌を探す
        let mut sorted_tiles: Vec<_> = tile_map
            .iter()
            .filter(|(_, &count)| count > 0)
            .map(|(tile, _)| *tile)
            .collect();
        sorted_tiles.sort_by(|a, b| {
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

        if sorted_tiles.is_empty() {
            return false;
        }

        let tile = sorted_tiles[0];
        let mut new_map = tile_map.clone();

        // 刻子を試す
        if new_map.get(&tile).copied().unwrap_or(0) >= 3 {
            *new_map.get_mut(&tile).unwrap() -= 3;
            if self.check_melds(&new_map, melds_needed - 1) {
                return true;
            }
            *new_map.get_mut(&tile).unwrap() += 3;
        }

        // 順子を試す（数牌のみ）
        if let crate::tile::TileType::Number { suit, value } = tile.tile_type {
            if value <= 7 {
                let tile2 = crate::tile::Tile::new_number(suit, value + 1, false);
                let tile3 = crate::tile::Tile::new_number(suit, value + 2, false);

                if new_map.get(&tile).copied().unwrap_or(0) >= 1
                    && new_map.get(&tile2).copied().unwrap_or(0) >= 1
                    && new_map.get(&tile3).copied().unwrap_or(0) >= 1
                {
                    *new_map.get_mut(&tile).unwrap() -= 1;
                    *new_map.entry(tile2).or_insert(0) -= 1;
                    *new_map.entry(tile3).or_insert(0) -= 1;

                    if self.check_melds(&new_map, melds_needed - 1) {
                        return true;
                    }

                    *new_map.get_mut(&tile).unwrap() += 1;
                    *new_map.entry(tile2).or_insert(0) += 1;
                    *new_map.entry(tile3).or_insert(0) += 1;
                }
            }
        }

        false
    }

    /// 七対子の判定
    fn check_chitoi(&self, tiles: &[Tile]) -> bool {
        if tiles.len() != 14 {
            return false;
        }

        let tile_map = Self::create_tile_map(tiles);

        // 7種類のペアがあるかチェック
        let pairs: Vec<_> = tile_map.iter().filter(|(_, &count)| count == 2).collect();

        pairs.len() == 7
    }

    /// 国士無双の判定
    fn check_kokushi(&self, tiles: &[Tile]) -> bool {
        use crate::tile::{TileType, Honor, Suit};

        if tiles.len() != 14 {
            return false;
        }

        let tile_map = Self::create_tile_map(tiles);

        // 13種の么九牌
        let terminals_and_honors = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 9, false),
            Tile::new_number(Suit::Pin, 1, false),
            Tile::new_number(Suit::Pin, 9, false),
            Tile::new_number(Suit::Sou, 1, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Nan),
            Tile::new_honor(Honor::Shaa),
            Tile::new_honor(Honor::Pei),
            Tile::new_honor(Honor::Haku),
            Tile::new_honor(Honor::Hatsu),
            Tile::new_honor(Honor::Chun),
        ];

        let mut has_pair = false;
        for yaochu_tile in &terminals_and_honors {
            let count = tile_map.get(yaochu_tile).copied().unwrap_or(0);
            if count == 0 {
                return false; // 必須の么九牌がない
            } else if count == 2 {
                if has_pair {
                    return false; // ペアが2組以上
                }
                has_pair = true;
            } else if count != 1 {
                return false; // 1枚または2枚以外
            }
        }

        // 他の牌がないかチェック
        for (tile, count) in tile_map.iter() {
            if !terminals_and_honors.contains(tile) && *count > 0 {
                return false;
            }
        }

        has_pair
    }

    /// シャンテン数の計算
    pub fn shanten(&self) -> i32 {
        if self.melds.is_empty() {
            // 七対子・国士無双も考慮して最小値を返す
            let normal = self.shanten_normal(&self.tiles);
            let chitoi = self.shanten_chitoi(&self.tiles);
            let kokushi = self.shanten_kokushi(&self.tiles);

            normal.min(chitoi).min(kokushi)
        } else {
            // 副露がある場合は通常形のみ
            self.shanten_normal(&self.tiles)
        }
    }

    /// 通常形のシャンテン数
    fn shanten_normal(&self, tiles: &[Tile]) -> i32 {
        let tile_map = Self::create_tile_map(tiles);
        let melds_needed = 4 - self.melds.len();

        // 簡易実装：完全なシャンテン計算は複雑なため、暫定版
        // TODO: より正確な実装
        let mut min_shanten = 8;

        for tile in tiles {
            if tile_map.get(tile).copied().unwrap_or(0) >= 2 {
                let mut test_map = tile_map.clone();
                *test_map.get_mut(tile).unwrap() -= 2;

                let melds_made = self.count_melds(&test_map);
                let shanten = melds_needed as i32 - melds_made as i32 - 1;
                min_shanten = min_shanten.min(shanten.max(0));
            }
        }

        min_shanten
    }

    /// 七対子形のシャンテン数
    fn shanten_chitoi(&self, tiles: &[Tile]) -> i32 {
        if tiles.len() != 13 {
            return 8;
        }

        let tile_map = Self::create_tile_map(tiles);
        let pairs = tile_map.iter().filter(|(_, &count)| count >= 2).count();

        6 - pairs as i32
    }

    /// 国士無双形のシャンテン数
    fn shanten_kokushi(&self, tiles: &[Tile]) -> i32 {
        use crate::tile::{TileType, Honor, Suit};

        let tile_map = Self::create_tile_map(tiles);

        let terminals_and_honors = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 9, false),
            Tile::new_number(Suit::Pin, 1, false),
            Tile::new_number(Suit::Pin, 9, false),
            Tile::new_number(Suit::Sou, 1, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Nan),
            Tile::new_honor(Honor::Shaa),
            Tile::new_honor(Honor::Pei),
            Tile::new_honor(Honor::Haku),
            Tile::new_honor(Honor::Hatsu),
            Tile::new_honor(Honor::Chun),
        ];

        let mut kinds = 0;
        let mut has_pair = false;

        for yaochu in &terminals_and_honors {
            if let Some(&count) = tile_map.get(yaochu) {
                if count > 0 {
                    kinds += 1;
                    if count >= 2 {
                        has_pair = true;
                    }
                }
            }
        }

        let mut shanten = 13 - kinds;
        if !has_pair {
            shanten -= 1;
        }

        shanten
    }

    /// 面子の数を数える（簡易版）
    fn count_melds(&self, tile_map: &HashMap<Tile, usize>) -> usize {
        let mut count = 0;
        let mut map = tile_map.clone();

        // 刻子を優先的に数える
        for (tile, &tile_count) in tile_map.iter() {
            if tile_count >= 3 {
                let sets = tile_count / 3;
                count += sets;
                *map.get_mut(tile).unwrap() -= sets * 3;
            }
        }

        // 順子を数える（簡易版）
        for (tile, &tile_count) in map.iter() {
            if tile_count > 0 {
                if let crate::tile::TileType::Number { suit, value } = tile.tile_type {
                    if value <= 7 {
                        let tile2 = crate::tile::Tile::new_number(suit, value + 1, false);
                        let tile3 = crate::tile::Tile::new_number(suit, value + 2, false);

                        let min = tile_count
                            .min(map.get(&tile2).copied().unwrap_or(0))
                            .min(map.get(&tile3).copied().unwrap_or(0));

                        if min > 0 {
                            count += min;
                        }
                    }
                }
            }
        }

        count
    }

    /// 牌のカウントマップを作成
    fn create_tile_map(tiles: &[Tile]) -> HashMap<Tile, usize> {
        let mut map = HashMap::new();
        for tile in tiles {
            // 赤ドラは通常牌として扱う
            let normalized = if tile.is_red {
                match tile.tile_type {
                    crate::tile::TileType::Number { suit, value } => {
                        crate::tile::Tile::new_number(suit, value, false)
                    }
                    _ => *tile,
                }
            } else {
                *tile
            };
            *map.entry(normalized).or_insert(0) += 1;
        }
        map
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