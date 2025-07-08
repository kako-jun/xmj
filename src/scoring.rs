use crate::tile::{Tile, TileType, Honor};
use crate::hand::{Hand, MeldType};

#[derive(Debug, Clone)]
pub enum Yaku {
    Riichi,
    Tanyao,
    Pinfu,
    Yakuhai(Honor),
    // 他の役も必要に応じて追加
}

#[derive(Debug, Clone)]
pub struct ScoringResult {
    pub han: u32,
    pub fu: u32,
    pub yaku: Vec<Yaku>,
    pub base_points: u32,
    pub total_points: u32,
}

pub struct ScoringEngine;

impl ScoringEngine {
    pub fn calculate_score(hand: &Hand, winning_tile: &Tile, is_tsumo: bool, is_dealer: bool) -> Option<ScoringResult> {
        let mut yaku = Vec::new();
        let mut han = 0;
        
        // 基本的な役判定
        if Self::check_tanyao(hand, winning_tile) {
            yaku.push(Yaku::Tanyao);
            han += 1;
        }
        
        if Self::check_pinfu(hand, winning_tile) {
            yaku.push(Yaku::Pinfu);
            han += 1;
        }
        
        // 役牌チェック
        for honor in [Honor::Haku, Honor::Hatsu, Honor::Chun] {
            if Self::check_yakuhai(hand, honor) {
                yaku.push(Yaku::Yakuhai(honor));
                han += 1;
            }
        }
        
        if han == 0 {
            return None; // 役なし
        }
        
        let fu = Self::calculate_fu(hand, winning_tile, is_tsumo);
        let base_points = Self::calculate_base_points(han, fu);
        let total_points = Self::calculate_total_points(base_points, is_dealer, is_tsumo);
        
        Some(ScoringResult {
            han,
            fu,
            yaku,
            base_points,
            total_points,
        })
    }
    
    fn check_tanyao(hand: &Hand, winning_tile: &Tile) -> bool {
        // タンヤオ: 么九牌が含まれていない
        let all_tiles: Vec<Tile> = hand.get_tiles().iter()
            .chain(hand.get_melds().iter().flat_map(|m| &m.tiles))
            .chain(std::iter::once(winning_tile))
            .cloned()
            .collect();
            
        for tile in all_tiles {
            match tile.tile_type {
                TileType::Number { value, .. } if value == 1 || value == 9 => return false,
                TileType::Honor(_) => return false,
                _ => {}
            }
        }
        true
    }
    
    fn check_pinfu(hand: &Hand, _winning_tile: &Tile) -> bool {
        // 簡単なピンフ判定（副露なし、全て順子）
        hand.get_melds().is_empty()
    }
    
    fn check_yakuhai(hand: &Hand, honor: Honor) -> bool {
        // 役牌判定: 指定された字牌の刻子があるか
        for meld in hand.get_melds() {
            if let MeldType::Pon | MeldType::Kan = meld.meld_type {
                if meld.tiles.len() >= 3 {
                    if let TileType::Honor(h) = meld.tiles[0].tile_type {
                        if h == honor {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
    
    fn calculate_fu(_hand: &Hand, _winning_tile: &Tile, is_tsumo: bool) -> u32 {
        // 簡単な符計算
        let mut fu = 20; // 基本符
        
        if is_tsumo {
            fu += 2; // ツモ符
        }
        
        // 待ちや面子による符は省略（簡単な実装）
        
        // 10の位を切り上げ
        ((fu + 9) / 10) * 10
    }
    
    fn calculate_base_points(han: u32, fu: u32) -> u32 {
        match han {
            1..=4 => fu * (1 << (han + 2)),
            5 => 2000,  // 満貫
            6..=7 => 3000, // 跳満
            8..=10 => 4000, // 倍満
            11..=12 => 6000, // 三倍満
            _ => 8000, // 役満
        }
    }
    
    fn calculate_total_points(base_points: u32, is_dealer: bool, is_tsumo: bool) -> u32 {
        if is_dealer {
            if is_tsumo {
                base_points * 6 // 親ツモ: 子全員からbase_points * 2
            } else {
                base_points * 6 // 親ロン: 放銃者からbase_points * 6
            }
        } else {
            if is_tsumo {
                base_points * 4 // 子ツモ: 親からbase_points * 2、子からbase_points * 1ずつ
            } else {
                base_points * 4 // 子ロン: 放銃者からbase_points * 4
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{Tile, Suit};

    #[test]
    fn test_tanyao_check() {
        let hand = Hand::new();
        let winning_tile = crate::tile::Tile::new_number(crate::tile::Suit::Man, 5, false);
        
        // タンヤオの基本テスト（実際の手牌構成は省略）
        assert!(ScoringEngine::check_tanyao(&hand, &winning_tile));
    }
    
    #[test]
    fn test_score_calculation() {
        let hand = Hand::new();
        let winning_tile = crate::tile::Tile::new_number(crate::tile::Suit::Man, 5, false);
        
        // 役なしの場合
        let result = ScoringEngine::calculate_score(&hand, &winning_tile, false, false);
        assert!(result.is_none());
    }
}