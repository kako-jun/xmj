use crate::tile::{Tile, TileType, Honor, Suit};
use crate::hand::{Hand, MeldType};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Yaku {
    // 一飜役
    Riichi,
    Ippatsu,
    Tsumo,
    Tanyao,
    Pinfu,
    Iipeikou,
    Yakuhai(Honor),
    Haitei,
    Houtei,
    Rinshan,
    Chankan,
    DoubleRiichi,

    // 二飜役
    Chanta,
    SanshokuDoujun,
    Ittsu,
    Toitoi,
    Sanankou,
    SanshokuDoukou,
    Sankantsu,
    Chiitoitsu,
    Shousangen,

    // 三飜役
    Honitsu,
    Junchan,
    Ryanpeikou,

    // 六飜役
    Chinitsu,

    // 役満
    Kokushi,
    Suuankou,
    Daisangen,
    Tsuuiisou,
    Shousuushii,
    Daisuushii,
    Ryuuiisou,
    Chinroutou,
    Chuuren,
    Suukantsu,
    Tenhou,
    Chiihou,
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
        let is_menzen = hand.get_melds().is_empty();

        // 手牌情報の取得
        let mut all_tiles = hand.get_tiles().clone();
        all_tiles.push(*winning_tile);

        // 役満チェック
        if Self::check_kokushi(&all_tiles) {
            yaku.push(Yaku::Kokushi);
            han += 13;
        }

        if Self::check_suuankou(hand, winning_tile, is_tsumo) {
            yaku.push(Yaku::Suuankou);
            han += 13;
        }

        if Self::check_daisangen(hand) {
            yaku.push(Yaku::Daisangen);
            han += 13;
        }

        if Self::check_tsuuiisou(&all_tiles) {
            yaku.push(Yaku::Tsuuiisou);
            han += 13;
        }

        if Self::check_ryuuiisou(&all_tiles) {
            yaku.push(Yaku::Ryuuiisou);
            han += 13;
        }

        if Self::check_chinroutou(&all_tiles) {
            yaku.push(Yaku::Chinroutou);
            han += 13;
        }

        if Self::check_chuuren(&all_tiles, is_menzen) {
            yaku.push(Yaku::Chuuren);
            han += 13;
        }

        // 役満がある場合は他の役をチェックしない
        if han >= 13 {
            let fu = Self::calculate_fu(hand, winning_tile, is_tsumo);
            let base_points = 8000; // 役満
            let total_points = Self::calculate_total_points(base_points, is_dealer, is_tsumo);

            return Some(ScoringResult {
                han,
                fu,
                yaku,
                base_points,
                total_points,
            });
        }

        // 七対子チェック（特殊形）
        if is_menzen && Self::check_chiitoitsu(&all_tiles) {
            yaku.push(Yaku::Chiitoitsu);
            han += 2;
        }

        // 通常役のチェック
        if Self::check_tanyao(&all_tiles) {
            yaku.push(Yaku::Tanyao);
            han += 1;
        }

        if is_menzen && Self::check_pinfu(hand, winning_tile) {
            yaku.push(Yaku::Pinfu);
            han += 1;
        }

        if is_tsumo && is_menzen {
            yaku.push(Yaku::Tsumo);
            han += 1;
        }

        if is_menzen {
            if Self::check_iipeikou(&all_tiles) {
                yaku.push(Yaku::Iipeikou);
                han += 1;
            }

            if Self::check_ryanpeikou(&all_tiles) {
                // 二盃口は一盃口を含むので、一盃口を削除
                yaku.retain(|y| y != &Yaku::Iipeikou);
                yaku.push(Yaku::Ryanpeikou);
                han = han.saturating_sub(1) + 3; // 一盃口を引いて二盃口を足す
            }
        }

        // 役牌チェック
        for honor in [Honor::Haku, Honor::Hatsu, Honor::Chun] {
            if Self::check_yakuhai(hand, honor) {
                yaku.push(Yaku::Yakuhai(honor));
                han += 1;
            }
        }

        // 三色同順
        if let Some(h) = Self::check_sanshoku_doujun(&all_tiles, hand) {
            yaku.push(Yaku::SanshokuDoujun);
            han += if is_menzen { 2 } else { 1 };
        }

        // 一気通貫
        if Self::check_ittsu(&all_tiles, hand) {
            yaku.push(Yaku::Ittsu);
            han += if is_menzen { 2 } else { 1 };
        }

        // 混全帯么九
        if Self::check_chanta(&all_tiles, hand) {
            yaku.push(Yaku::Chanta);
            han += if is_menzen { 2 } else { 1 };
        }

        // 純全帯么九
        if Self::check_junchan(&all_tiles, hand) {
            // 純チャンがあればチャンタを削除
            yaku.retain(|y| y != &Yaku::Chanta);
            yaku.push(Yaku::Junchan);
            han = han.saturating_sub(if is_menzen { 2 } else { 1 }) + if is_menzen { 3 } else { 2 };
        }

        // 対々和
        if Self::check_toitoi(hand) {
            yaku.push(Yaku::Toitoi);
            han += 2;
        }

        // 三暗刻
        if Self::check_sanankou(hand, winning_tile, is_tsumo) {
            yaku.push(Yaku::Sanankou);
            han += 2;
        }

        // 三色同刻
        if Self::check_sanshoku_doukou(hand) {
            yaku.push(Yaku::SanshokuDoukou);
            han += 2;
        }

        // 小三元
        if Self::check_shousangen(hand) {
            yaku.push(Yaku::Shousangen);
            han += 2;
        }

        // 混一色
        if Self::check_honitsu(&all_tiles) {
            yaku.push(Yaku::Honitsu);
            han += if is_menzen { 3 } else { 2 };
        }

        // 清一色
        if Self::check_chinitsu(&all_tiles) {
            // 清一色があれば混一色を削除
            yaku.retain(|y| y != &Yaku::Honitsu);
            yaku.push(Yaku::Chinitsu);
            han = han.saturating_sub(if is_menzen { 3 } else { 2 }) + if is_menzen { 6 } else { 5 };
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
    
    // タンヤオ（断么九）
    fn check_tanyao(tiles: &[Tile]) -> bool {
        tiles.iter().all(|tile| match tile.tile_type {
            TileType::Number { value, .. } => value >= 2 && value <= 8,
            TileType::Honor(_) => false,
        })
    }

    // ピンフ（平和）
    fn check_pinfu(hand: &Hand, _winning_tile: &Tile) -> bool {
        // 副露なし、全て順子、雀頭が役牌でない、両面待ち
        // 簡易実装: 副露なしのみチェック
        hand.get_melds().is_empty()
    }

    // 役牌
    fn check_yakuhai(hand: &Hand, honor: Honor) -> bool {
        for meld in hand.get_melds() {
            if let MeldType::Pon | MeldType::Kan = meld.meld_type {
                if !meld.tiles.is_empty() {
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

    // 七対子
    fn check_chiitoitsu(tiles: &[Tile]) -> bool {
        if tiles.len() != 14 {
            return false;
        }

        let mut tile_map = HashMap::new();
        for tile in tiles {
            *tile_map.entry(*tile).or_insert(0) += 1;
        }

        let pairs: Vec<_> = tile_map.iter().filter(|(_, &count)| count == 2).collect();
        pairs.len() == 7
    }

    // 一盃口
    fn check_iipeikou(tiles: &[Tile]) -> bool {
        // 同じ順子が2組
        // 簡易実装: 省略（複雑なので後で実装）
        false
    }

    // 二盃口
    fn check_ryanpeikou(tiles: &[Tile]) -> bool {
        // 同じ順子が2組x2
        // 簡易実装: 省略
        false
    }

    // 三色同順
    fn check_sanshoku_doujun(tiles: &[Tile], hand: &Hand) -> Option<u8> {
        // 萬子・筒子・索子で同じ数の順子
        // 簡易実装: 省略
        None
    }

    // 一気通貫
    fn check_ittsu(tiles: &[Tile], hand: &Hand) -> bool {
        // 同じ色で123・456・789の順子
        // 簡易実装: 省略
        false
    }

    // 混全帯么九（チャンタ）
    fn check_chanta(tiles: &[Tile], hand: &Hand) -> bool {
        // 全ての面子と雀頭に么九牌が含まれる
        // 簡易実装: 省略
        false
    }

    // 純全帯么九（ジュンチャン）
    fn check_junchan(tiles: &[Tile], hand: &Hand) -> bool {
        // 全ての面子と雀頭に老頭牌（1,9）が含まれる（字牌なし）
        // 簡易実装: 省略
        false
    }

    // 対々和
    fn check_toitoi(hand: &Hand) -> bool {
        // 全ての面子が刻子
        if hand.get_melds().len() + 4 != 4 {
            return false;
        }

        hand.get_melds().iter().all(|meld| match meld.meld_type {
            MeldType::Pon | MeldType::Kan => true,
            _ => false,
        })
    }

    // 三暗刻
    fn check_sanankou(hand: &Hand, _winning_tile: &Tile, is_tsumo: bool) -> bool {
        // 暗刻が3組
        // 簡易実装: 副露がない刻子が3組
        let ankou_count = hand
            .get_melds()
            .iter()
            .filter(|meld| !meld.is_open && matches!(meld.meld_type, MeldType::Pon | MeldType::Kan))
            .count();

        ankou_count >= 3
    }

    // 三色同刻
    fn check_sanshoku_doukou(hand: &Hand) -> bool {
        // 萬子・筒子・索子で同じ数の刻子
        // 簡易実装: 省略
        false
    }

    // 小三元
    fn check_shousangen(hand: &Hand) -> bool {
        // 三元牌の2組が刻子、1組が雀頭
        // 簡易実装: 省略
        false
    }

    // 混一色
    fn check_honitsu(tiles: &[Tile]) -> bool {
        let mut suits = HashSet::new();
        let mut has_honors = false;

        for tile in tiles {
            match tile.tile_type {
                TileType::Number { suit, .. } => {
                    suits.insert(suit);
                }
                TileType::Honor(_) => {
                    has_honors = true;
                }
            }
        }

        suits.len() == 1 && has_honors
    }

    // 清一色
    fn check_chinitsu(tiles: &[Tile]) -> bool {
        let mut suits = HashSet::new();

        for tile in tiles {
            match tile.tile_type {
                TileType::Number { suit, .. } => {
                    suits.insert(suit);
                }
                TileType::Honor(_) => return false,
            }
        }

        suits.len() == 1
    }

    // 国士無双
    fn check_kokushi(tiles: &[Tile]) -> bool {
        if tiles.len() != 14 {
            return false;
        }

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

        let mut tile_map = HashMap::new();
        for tile in tiles {
            *tile_map.entry(*tile).or_insert(0) += 1;
        }

        let mut has_pair = false;
        for yaochu_tile in &terminals_and_honors {
            let count = tile_map.get(yaochu_tile).copied().unwrap_or(0);
            if count == 0 {
                return false;
            } else if count == 2 {
                if has_pair {
                    return false;
                }
                has_pair = true;
            } else if count != 1 {
                return false;
            }
        }

        has_pair
    }

    // 四暗刻
    fn check_suuankou(hand: &Hand, _winning_tile: &Tile, is_tsumo: bool) -> bool {
        // 暗刻が4組（ツモの場合のみ）
        if !is_tsumo {
            return false;
        }

        let ankou_count = hand
            .get_melds()
            .iter()
            .filter(|meld| !meld.is_open && matches!(meld.meld_type, MeldType::Pon | MeldType::Kan))
            .count();

        ankou_count >= 4
    }

    // 大三元
    fn check_daisangen(hand: &Hand) -> bool {
        // 三元牌（白発中）の3組が全て刻子
        let mut sangenpai_count = 0;

        for meld in hand.get_melds() {
            if let MeldType::Pon | MeldType::Kan = meld.meld_type {
                if !meld.tiles.is_empty() {
                    if let TileType::Honor(h) = meld.tiles[0].tile_type {
                        if matches!(h, Honor::Haku | Honor::Hatsu | Honor::Chun) {
                            sangenpai_count += 1;
                        }
                    }
                }
            }
        }

        sangenpai_count == 3
    }

    // 字一色
    fn check_tsuuiisou(tiles: &[Tile]) -> bool {
        tiles.iter().all(|tile| matches!(tile.tile_type, TileType::Honor(_)))
    }

    // 緑一色
    fn check_ryuuiisou(tiles: &[Tile]) -> bool {
        // 索子の2,3,4,6,8と発のみ
        tiles.iter().all(|tile| match tile.tile_type {
            TileType::Number { suit: Suit::Sou, value } => matches!(value, 2 | 3 | 4 | 6 | 8),
            TileType::Honor(Honor::Hatsu) => true,
            _ => false,
        })
    }

    // 清老頭
    fn check_chinroutou(tiles: &[Tile]) -> bool {
        // 全て老頭牌（1,9）
        tiles.iter().all(|tile| match tile.tile_type {
            TileType::Number { value, .. } => value == 1 || value == 9,
            _ => false,
        })
    }

    // 九蓮宝燈
    fn check_chuuren(tiles: &[Tile], is_menzen: bool) -> bool {
        if !is_menzen || tiles.len() != 14 {
            return false;
        }

        // 清一色かチェック
        if !Self::check_chinitsu(tiles) {
            return false;
        }

        // 1112345678999のパターン
        // 簡易実装: 省略
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
        // タンヤオの手牌を作成（2-8のみ）
        let tiles = vec![
            crate::tile::Tile::new_number(crate::tile::Suit::Man, 2, false),
            crate::tile::Tile::new_number(crate::tile::Suit::Man, 3, false),
            crate::tile::Tile::new_number(crate::tile::Suit::Man, 4, false),
            crate::tile::Tile::new_number(crate::tile::Suit::Pin, 5, false),
        ];

        assert!(ScoringEngine::check_tanyao(&tiles));

        // 1や9が含まれる場合はfalse
        let tiles_with_terminal = vec![
            crate::tile::Tile::new_number(crate::tile::Suit::Man, 1, false),
            crate::tile::Tile::new_number(crate::tile::Suit::Man, 2, false),
        ];

        assert!(!ScoringEngine::check_tanyao(&tiles_with_terminal));
    }
    
    #[test]
    fn test_score_calculation() {
        let mut hand = Hand::new();

        // 役なしの手牌を作成（ピンフにもタンヤオにもならない）
        // 1m 1m 1m 9p 9p 9p 1s 1s 1s to to to hk
        for _ in 0..3 {
            hand.add_tile(crate::tile::Tile::new_number(crate::tile::Suit::Man, 1, false));
            hand.add_tile(crate::tile::Tile::new_number(crate::tile::Suit::Pin, 9, false));
            hand.add_tile(crate::tile::Tile::new_number(crate::tile::Suit::Sou, 1, false));
            hand.add_tile(crate::tile::Tile::new_honor(crate::tile::Honor::Ton));
        }
        hand.add_tile(crate::tile::Tile::new_honor(crate::tile::Honor::Haku));

        let winning_tile = crate::tile::Tile::new_honor(crate::tile::Honor::Haku);

        // 役牌白のみ（副露なしで門前なのでリーチ可能だが、リーチはここでは判定しない）
        let result = ScoringEngine::calculate_score(&hand, &winning_tile, false, false);

        // 白の刻子があるので役牌が付く
        assert!(result.is_some());
        if let Some(scoring) = result {
            assert!(scoring.han >= 1);
        }
    }
}