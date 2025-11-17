use crate::tile::Tile;
use crate::hand::Hand;
use rand::seq::SliceRandom;
use rand::thread_rng;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AiLevel {
    Random,      // レベル1: ランダム打牌
    Simple,      // レベル2: 孤立牌優先
    Intermediate, // レベル3: シャンテン数ベース
    Advanced,    // レベル4: 期待値計算（未実装）
}

pub struct AiEngine {
    level: AiLevel,
}

impl AiEngine {
    pub fn new(level: AiLevel) -> Self {
        Self { level }
    }

    /// AIが打牌する牌を選択
    pub fn select_discard(&self, hand: &Hand) -> Option<Tile> {
        let tiles = hand.get_tiles();
        if tiles.is_empty() {
            return None;
        }

        match self.level {
            AiLevel::Random => self.select_random(tiles),
            AiLevel::Simple => self.select_simple(tiles),
            AiLevel::Intermediate => self.select_intermediate(hand),
            AiLevel::Advanced => self.select_advanced(hand),
        }
    }

    /// レベル1: ランダムに打牌を選択
    fn select_random(&self, tiles: &[Tile]) -> Option<Tile> {
        let mut rng = thread_rng();
        tiles.choose(&mut rng).copied()
    }

    /// レベル2: 孤立牌を優先的に打牌
    fn select_simple(&self, tiles: &[Tile]) -> Option<Tile> {
        use crate::tile::TileType;
        use std::collections::HashMap;

        // 牌のカウント
        let mut tile_counts = HashMap::new();
        for tile in tiles {
            *tile_counts.entry(*tile).or_insert(0) += 1;
        }

        // 1. 字牌で孤立しているものを優先
        for tile in tiles {
            if let TileType::Honor(_) = tile.tile_type {
                if tile_counts.get(tile).copied().unwrap_or(0) == 1 {
                    return Some(*tile);
                }
            }
        }

        // 2. 数牌で孤立しているものを選択
        for tile in tiles {
            if let TileType::Number { suit, value } = tile.tile_type {
                // 前後の牌がないか確認
                let has_prev = if value > 1 {
                    let prev = Tile::new_number(suit, value - 1, false);
                    tile_counts.contains_key(&prev)
                } else {
                    false
                };

                let has_next = if value < 9 {
                    let next = Tile::new_number(suit, value + 1, false);
                    tile_counts.contains_key(&next)
                } else {
                    false
                };

                // 孤立牌（前後に連続する牌がない）
                if !has_prev && !has_next && tile_counts.get(tile).copied().unwrap_or(0) == 1 {
                    return Some(*tile);
                }
            }
        }

        // 3. 端牌（1, 9）を優先
        for tile in tiles {
            if let TileType::Number { value, .. } = tile.tile_type {
                if value == 1 || value == 9 {
                    return Some(*tile);
                }
            }
        }

        // 4. どれも該当しない場合は最初の牌
        tiles.first().copied()
    }

    /// レベル3: シャンテン数を考慮した打牌選択
    fn select_intermediate(&self, hand: &Hand) -> Option<Tile> {
        let tiles = hand.get_tiles();
        if tiles.is_empty() {
            return None;
        }

        let current_shanten = hand.shanten();
        let mut best_tile = tiles[0];
        let mut best_shanten = 100;

        // 各牌を打牌した場合のシャンテン数を計算
        for &tile in tiles {
            let mut test_hand = hand.clone();
            if test_hand.remove_tile(&tile) {
                let shanten = test_hand.shanten();

                // より良いシャンテン数になる牌を選択
                if shanten < best_shanten {
                    best_shanten = shanten;
                    best_tile = tile;
                } else if shanten == best_shanten {
                    // 同じシャンテン数なら、端牌や字牌を優先
                    if Self::is_less_useful(&tile, &best_tile) {
                        best_tile = tile;
                    }
                }
            }
        }

        Some(best_tile)
    }

    /// レベル4: 期待値計算ベース（未実装）
    fn select_advanced(&self, hand: &Hand) -> Option<Tile> {
        // TODO: より高度な期待値計算を実装
        // 暫定的にレベル3と同じ
        self.select_intermediate(hand)
    }

    /// 牌の有用性を比較（より有用でない方がtrue）
    fn is_less_useful(tile1: &Tile, tile2: &Tile) -> bool {
        use crate::tile::TileType;

        match (&tile1.tile_type, &tile2.tile_type) {
            // 字牌 vs 数牌 -> 字牌の方が有用でない
            (TileType::Honor(_), TileType::Number { .. }) => true,
            (TileType::Number { .. }, TileType::Honor(_)) => false,

            // 端牌（1,9）vs 中張牌 -> 端牌の方が有用でない
            (TileType::Number { value: v1, .. }, TileType::Number { value: v2, .. }) => {
                let is_terminal1 = *v1 == 1 || *v1 == 9;
                let is_terminal2 = *v2 == 1 || *v2 == 9;

                match (is_terminal1, is_terminal2) {
                    (true, false) => true,
                    (false, true) => false,
                    _ => false, // どちらも端牌またはどちらも中張牌
                }
            }

            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{Tile, Suit, Honor};

    #[test]
    fn test_ai_random() {
        let ai = AiEngine::new(AiLevel::Random);
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Man, 3, false));

        let discard = ai.select_discard(&hand);
        assert!(discard.is_some());
    }

    #[test]
    fn test_ai_simple() {
        let ai = AiEngine::new(AiLevel::Simple);
        let mut hand = Hand::new();

        // 孤立字牌を追加
        hand.add_tile(Tile::new_honor(Honor::Ton));
        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Man, 3, false));

        let discard = ai.select_discard(&hand);
        // 孤立字牌が選ばれるべき
        assert_eq!(discard, Some(Tile::new_honor(Honor::Ton)));
    }

    #[test]
    fn test_ai_intermediate() {
        let ai = AiEngine::new(AiLevel::Intermediate);
        let mut hand = Hand::new();

        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Man, 3, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 9, false));

        let discard = ai.select_discard(&hand);
        assert!(discard.is_some());
    }
}
