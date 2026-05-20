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

    /// 13 枚手 (副露なし) の待ち牌候補を列挙する。
    ///
    /// 34 種の各候補牌を順に試し、加えると `is_winning_hand` 成立するものを返す。
    /// 副露があるとき・手牌が `13 - melds*3` 枚でないときは空 Vec を返す。
    ///
    /// Issue #34: 多面待ちの精度確認 / UI 表示拡張のためのユーティリティ。
    pub fn compute_machi_tiles(&self) -> Vec<Tile> {
        let expected_tiles = 13usize.saturating_sub(self.melds.len() * 3);
        if self.tiles.len() != expected_tiles {
            return Vec::new();
        }
        // 副露ありの待ち列挙は #34 スコープ外。`is_winning_hand` が melds を考慮するので
        // 一応動作はするが、ここでは簡明性のため副露なしに限定する。
        if !self.melds.is_empty() {
            return Vec::new();
        }
        let mut waits: Vec<Tile> = Vec::new();
        for candidate in Self::all_34_tiles() {
            if waits.contains(&candidate) {
                continue;
            }
            if self.can_win(&candidate) {
                waits.push(candidate);
            }
        }
        waits
    }

    /// 34 種の標準牌列挙 (赤ドラなし、is_glass なし)。
    fn all_34_tiles() -> Vec<Tile> {
        use crate::tile::{Honor, Suit};
        let mut out: Vec<Tile> = Vec::with_capacity(34);
        for suit in [Suit::Man, Suit::Pin, Suit::Sou] {
            for value in 1..=9u8 {
                out.push(Tile::new_number(suit, value, false));
            }
        }
        for h in Honor::ALL {
            out.push(Tile::new_honor(h));
        }
        out
    }

    pub fn can_win(&self, winning_tile: &Tile) -> bool {
        let mut test_tiles = self.tiles.clone();
        test_tiles.push(*winning_tile);
        self.is_winning_hand(&test_tiles)
    }

    /// 5 枚麻雀（FiveTile モード）の和了判定。
    ///
    /// 5 枚麻雀の和了形は「雀頭(2) + 面子(3) = 5 枚使い切り」。
    /// アガリ牌は和了形の構成牌として必ず使われている必要があるため、
    /// 本関数は以下を要求する:
    ///
    /// 1. `melds` が空（5 枚麻雀は門前運用）
    /// 2. `tiles.len() == 5`（ツモ後 / ロン後で手に取り込まれた状態）
    /// 3. 手牌 5 枚に `winning_tile` と同値の牌が少なくとも 1 枚含まれている
    /// 4. 手牌 5 枚そのものが「雀頭 + 面子」で使い切れる
    ///
    /// これにより「既に和了形を含む手牌に対して、関係ない捨て牌でロン成立」というバグを防ぐ。
    pub fn can_win_five_tile(&self, winning_tile: &Tile) -> bool {
        if !self.melds.is_empty() {
            return false;
        }
        if self.tiles.len() != 5 {
            return false;
        }
        if !self.tiles.contains(winning_tile) {
            return false;
        }
        Self::is_five_tile_complete_form(&self.tiles)
    }

    /// 5 枚麻雀のテンパイ判定（打牌後相当）。
    ///
    /// 手牌 5 枚から 1 枚捨てたあと、残り 4 枚にいずれかの 1 枚を加えて
    /// 5 枚で和了形になる組み合わせが存在するかを判定する。
    /// 「いま手にある 5 枚のうち、どれかを捨てれば次のツモ/ロンで和了できる」状態。
    pub fn is_tenpai_five_tile(&self) -> bool {
        !self.five_tile_waits().is_empty()
    }

    /// 5 枚麻雀の待ち牌候補リスト。
    ///
    /// 手牌 5 枚から 1 枚捨てた残り 4 枚に対して、34 種の各候補牌を加えて
    /// 5 枚で和了形（雀頭 + 面子）が成立するものを返す。
    /// 鳴きがあるとき・手牌が 5 枚でないときは空 Vec を返す。
    pub fn five_tile_waits(&self) -> Vec<Tile> {
        let mut waits: Vec<Tile> = Vec::new();
        if !self.melds.is_empty() {
            return waits;
        }
        if self.tiles.len() != 5 {
            return waits;
        }

        for candidate in Self::five_tile_candidate_tiles() {
            // 既に拾った待ち牌は再判定不要
            if waits.contains(&candidate) {
                continue;
            }
            // 手牌の 1 枚を捨て、candidate を加えた 5 枚で完成形になるか
            for i in 0..self.tiles.len() {
                let mut hand4: Vec<Tile> = self.tiles.clone();
                hand4.remove(i);
                hand4.push(candidate);
                if Self::is_five_tile_complete_form(&hand4) {
                    waits.push(candidate);
                    break;
                }
            }
        }
        waits
    }

    /// 5 枚麻雀の待ち牌候補列挙（34 種、赤ドラなし）。
    fn five_tile_candidate_tiles() -> Vec<Tile> {
        use crate::tile::{Honor, Suit};
        let mut out: Vec<Tile> = Vec::with_capacity(34);
        for suit in [Suit::Man, Suit::Pin, Suit::Sou] {
            for value in 1..=9u8 {
                out.push(Tile::new_number(suit, value, false));
            }
        }
        for honor in Honor::ALL {
            out.push(Tile::new_honor(honor));
        }
        out
    }

    /// 5 枚が「雀頭(2) + 面子(3)」で完全に使い切れるかを判定する。
    /// 余り牌は許容しない（5 枚すべてが和了形に組み込まれる必要がある）。
    fn is_five_tile_complete_form(tiles: &[Tile]) -> bool {
        if tiles.len() != 5 {
            return false;
        }
        Self::has_pair_and_one_meld(tiles)
    }

    /// `tiles` を「対子 2 枚 + 面子 3 枚 = 5 枚」で完全に使い切れるか判定する。
    fn has_pair_and_one_meld(tiles: &[Tile]) -> bool {
        if tiles.len() != 5 {
            return false;
        }
        let tile_map = Self::create_tile_map(tiles);
        let unique_tiles: Vec<Tile> = tile_map.keys().copied().collect();

        for pair_tile in &unique_tiles {
            if tile_map.get(pair_tile).copied().unwrap_or(0) < 2 {
                continue;
            }
            let mut remain = tile_map.clone();
            *remain.get_mut(pair_tile).unwrap() -= 2;

            if Self::has_exactly_one_meld(&remain) {
                return true;
            }
        }
        false
    }

    /// `tile_map` の総数が 3 枚で、その 3 枚が面子 1 組（順子 or 刻子）として
    /// ぴったり使い切れるかを判定する。
    fn has_exactly_one_meld(tile_map: &HashMap<Tile, usize>) -> bool {
        let total: usize = tile_map.values().sum();
        if total != 3 {
            return false;
        }
        for (tile, &count) in tile_map.iter() {
            if count == 3 {
                return true;
            }
            if let crate::tile::TileType::Number { suit, value } = tile.tile_type {
                if (1..=7).contains(&value) && count >= 1 {
                    let tile2 = crate::tile::Tile::new_number(suit, value + 1, false);
                    let tile3 = crate::tile::Tile::new_number(suit, value + 2, false);
                    if tile_map.get(&tile2).copied().unwrap_or(0) >= 1
                        && tile_map.get(&tile3).copied().unwrap_or(0) >= 1
                    {
                        return true;
                    }
                }
            }
        }
        false
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
        use crate::tile::{Honor, Suit};

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
        use crate::tile::{Honor, Suit};

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

    /// 5 枚麻雀: 手牌 5 枚が「雀頭 + 刻子」で完成形 + アガリ牌が手牌に含まれる → 和了成立
    /// 手牌 5 枚: 2m 2m 5p 5p 5p、アガリ牌: 2m（雀頭の一部）
    #[test]
    fn test_five_tile_winning_pair_triplet() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));

        // アガリ牌は和了形の構成牌でなければならない
        let winning_tile = Tile::new_number(Suit::Man, 2, false);
        assert!(hand.can_win_five_tile(&winning_tile), "雀頭 + 刻子の完成形で構成牌 2m を上がる");
    }

    /// 5 枚麻雀: 雀頭 + 順子の完成形でアガリ牌が順子の一部 → 和了成立
    /// 手牌 5 枚: 7m 7m 1s 2s 3s、アガリ牌: 2s
    #[test]
    fn test_five_tile_winning_pair_sequence() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 7, false));
        hand.add_tile(Tile::new_number(Suit::Man, 7, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 1, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 2, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 3, false));

        let winning_tile = Tile::new_number(Suit::Sou, 2, false);
        assert!(hand.can_win_five_tile(&winning_tile), "雀頭 + 順子の完成形で構成牌 2s を上がる");
    }

    /// 5 枚麻雀: 雀頭も面子も作れない場合は和了不可
    #[test]
    fn test_five_tile_not_winning() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 3, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 5, false));
        hand.add_tile(Tile::new_number(Suit::Man, 7, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 9, false));

        let winning_tile = Tile::new_honor(Honor::Ton);
        assert!(!hand.can_win_five_tile(&winning_tile), "対子も面子も作れないので和了不可");
    }

    /// M-1 リグレッション: 既存テストが踏んでいた致命バグの再発防止。
    /// 「手牌 5 枚が完成形でも、アガリ牌が手牌に含まれなければロン不成立」
    /// 旧実装は 6 枚目を余り牌として無視していたため、任意の捨て牌でロンが通っていた。
    #[test]
    fn test_five_tile_rejects_unrelated_winning_tile() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));

        // 9m は手牌に含まれない → 和了不可
        let unrelated = Tile::new_number(Suit::Man, 9, false);
        assert!(
            !hand.can_win_five_tile(&unrelated),
            "アガリ牌が手牌に含まれなければ和了不可（M-1 リグレッション）"
        );

        // 字牌（東）でも同様
        let unrelated_honor = Tile::new_honor(Honor::Ton);
        assert!(
            !hand.can_win_five_tile(&unrelated_honor),
            "関係ない字牌の捨て牌ではロン不可"
        );
    }

    /// 5 枚麻雀: 字牌の雀頭 + 字牌の刻子は可能だが、刻子は字牌でも OK
    /// 手牌: to to hk hk hk（東対子 + 白刻子）
    #[test]
    fn test_five_tile_honor_triplet_winning() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_honor(Honor::Ton));
        hand.add_tile(Tile::new_honor(Honor::Ton));
        hand.add_tile(Tile::new_honor(Honor::Haku));
        hand.add_tile(Tile::new_honor(Honor::Haku));
        hand.add_tile(Tile::new_honor(Honor::Haku));

        let winning_tile = Tile::new_honor(Honor::Haku);
        assert!(hand.can_win_five_tile(&winning_tile), "字牌の雀頭 + 字牌の刻子で和了");
    }

    /// 5 枚麻雀: 手牌 4 枚（打牌後）では和了不可（tiles.len() == 5 を要求）
    #[test]
    fn test_five_tile_requires_five_tiles() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));

        let win = Tile::new_number(Suit::Man, 2, false);
        assert!(!hand.can_win_five_tile(&win), "手牌 4 枚では和了不可");
    }

    /// 5 枚麻雀: テンパイ判定（打牌後相当）
    /// 手牌 5 枚: 7m 7m 1s 2s 3s（既に完成形）
    /// → 1 枚捨てて待ち牌で完成し直せるかを確認
    #[test]
    fn test_five_tile_tenpai_detection() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 7, false));
        hand.add_tile(Tile::new_number(Suit::Man, 7, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 1, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 2, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 3, false));

        assert!(hand.is_tenpai_five_tile(), "完成形を持つ手は当然テンパイ");

        // 完全バラバラ手はテンパイ不可
        let mut bad_hand = Hand::new();
        bad_hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        bad_hand.add_tile(Tile::new_number(Suit::Pin, 3, false));
        bad_hand.add_tile(Tile::new_number(Suit::Sou, 5, false));
        bad_hand.add_tile(Tile::new_number(Suit::Man, 7, false));
        bad_hand.add_tile(Tile::new_honor(Honor::Ton));

        assert!(!bad_hand.is_tenpai_five_tile(), "対子の元すらない手はテンパイ不可");
    }

    /// 5 枚麻雀: 単騎待ち（雀頭待ち）
    /// 手牌 5 枚: 1m 2m 3m 5p X → 5p で雀頭の単騎、捨てる牌は X
    /// 例: 1m 2m 3m 5p 9s → 9s を捨てれば「5p の単騎待ち」
    #[test]
    fn test_five_tile_waits_tanki() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Man, 3, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 9, false));

        let waits = hand.five_tile_waits();
        // 9s 捨てで 5p 単騎待ち
        assert!(
            waits.contains(&Tile::new_number(Suit::Pin, 5, false)),
            "9s 捨ての 5p 単騎待ちが含まれる: {:?}",
            waits
        );
    }

    /// 5 枚麻雀: シャンポン待ち（雀頭 2 候補）
    /// 手牌: 2m 2m 5p 5p 9s → 9s 捨て後、2m 2m 5p 5p の 4 枚は
    /// 2m か 5p の刻子 + 残対子で完成 → シャンポン待ち（2m / 5p）
    #[test]
    fn test_five_tile_waits_shanpon() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 9, false));

        let waits = hand.five_tile_waits();
        assert!(
            waits.contains(&Tile::new_number(Suit::Man, 2, false)),
            "シャンポン待ち 2m が含まれる: {:?}",
            waits
        );
        assert!(
            waits.contains(&Tile::new_number(Suit::Pin, 5, false)),
            "シャンポン待ち 5p が含まれる: {:?}",
            waits
        );
    }

    /// 5 枚麻雀: カンチャン待ち
    /// 手牌: 7m 7m 1s 3s 9p → 9p 捨て、1s 3s で 2s カンチャン待ち
    #[test]
    fn test_five_tile_waits_kanchan() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 7, false));
        hand.add_tile(Tile::new_number(Suit::Man, 7, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 1, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 3, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 9, false));

        let waits = hand.five_tile_waits();
        assert!(
            waits.contains(&Tile::new_number(Suit::Sou, 2, false)),
            "カンチャン待ち 2s が含まれる: {:?}",
            waits
        );
    }

    // ==================== Issue #33: 副露込み和了形抽出のテスト ====================

    /// 副露ヘルパー: 順子・刻子・カンの Meld を作る
    fn pon_meld(tile: Tile) -> Meld {
        Meld {
            meld_type: MeldType::Pon,
            tiles: vec![tile, tile, tile],
            is_open: true,
        }
    }
    fn chi_meld(suit: Suit, start: u8) -> Meld {
        Meld {
            meld_type: MeldType::Chi,
            tiles: vec![
                Tile::new_number(suit, start, false),
                Tile::new_number(suit, start + 1, false),
                Tile::new_number(suit, start + 2, false),
            ],
            is_open: true,
        }
    }
    fn kan_meld(tile: Tile, is_open: bool) -> Meld {
        Meld {
            meld_type: MeldType::Kan,
            tiles: vec![tile, tile, tile, tile],
            is_open,
        }
    }

    /// ポン 1 つ + 残り手牌 10 枚 + 和了牌 1 = 副露込みで 4 面子 1 雀頭
    /// 副露: 1m 1m 1m (ポン)
    /// 残り手牌: 2p 3p 4p 5p 5p 7p 8p 9p 6s 6s (10 枚)
    /// 和了牌: 5p (シャンポン待ち)
    /// 構成: [1m1m1m] + 2p3p4p + 5p5p5p + 7p8p9p + 6s6s (雀頭)
    #[test]
    fn test_can_win_with_pon_meld() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Pin, 2, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 3, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 4, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 7, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 8, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 9, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 6, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 6, false));
        // ポン
        hand.melds.push(pon_meld(Tile::new_number(Suit::Man, 1, false)));

        let win = Tile::new_number(Suit::Pin, 5, false);
        assert!(
            hand.can_win(&win),
            "ポン 1m + 残り手牌で和了形が組めるはず"
        );
    }

    /// チー 1 つ + 残り手牌 10 枚 + 和了牌 1
    /// チー: 4m5m6m
    /// 残り手牌: 1m1m1m 2p3p4p 7s8s9s 5p (10 枚)
    /// 和了牌: 5p (単騎雀頭)
    /// 構成: [4m5m6m] + 1m1m1m + 2p3p4p + 7s8s9s + 5p5p (雀頭)
    #[test]
    fn test_can_win_with_chi_meld() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 2, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 3, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 4, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 7, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 8, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 9, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        // チー
        hand.melds.push(chi_meld(Suit::Man, 4));

        let win = Tile::new_number(Suit::Pin, 5, false);
        assert!(
            hand.can_win(&win),
            "チー 4m5m6m + 残り手牌で和了形が組めるはず"
        );
    }

    /// 明槓 1 つ + 残り手牌 10 枚 + 和了牌 1
    /// 明槓: 9m9m9m9m (4 枚で 1 面子相当、tiles 4 + tile_count 計算上 +3)
    /// 注: Hand::tile_count() は `melds.len() * 3` なのでカンも 3 枚相当として扱う。
    #[test]
    fn test_can_win_with_kan_meld() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Pin, 1, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 2, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 3, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 4, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 5, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 6, false));
        hand.add_tile(Tile::new_honor(Honor::Haku));
        hand.add_tile(Tile::new_honor(Honor::Haku));
        hand.add_tile(Tile::new_honor(Honor::Haku));
        hand.add_tile(Tile::new_honor(Honor::Ton));
        // カン (9m)
        hand.melds.push(kan_meld(Tile::new_number(Suit::Man, 9, false), true));

        let win = Tile::new_honor(Honor::Ton);
        assert!(
            hand.can_win(&win),
            "カン 9m + 残り手牌 (123p, 456s, 白刻子, 東単騎) で東を引いて和了"
        );
    }

    /// 副露 2 つ (ポン + チー) + 残り手牌 7 枚 + 和了牌 1
    #[test]
    fn test_can_win_with_two_melds() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Sou, 7, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 8, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 9, false));
        hand.add_tile(Tile::new_honor(Honor::Chun));
        hand.add_tile(Tile::new_honor(Honor::Chun));
        hand.add_tile(Tile::new_honor(Honor::Chun));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        // ポン (1m) + チー (4m5m6m)
        hand.melds.push(pon_meld(Tile::new_number(Suit::Man, 1, false)));
        hand.melds.push(chi_meld(Suit::Man, 4));

        let win = Tile::new_number(Suit::Pin, 5, false);
        assert!(
            hand.can_win(&win),
            "ポン + チー + 7s8s9s + 中刻子 + 5p雀頭で和了"
        );
    }

    /// 副露 3 つ + 残り手牌 4 枚 + 和了牌 1
    #[test]
    fn test_can_win_with_three_melds() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Pin, 7, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 8, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 9, false));
        hand.add_tile(Tile::new_honor(Honor::Ton));
        // 副露 3 つ
        hand.melds.push(pon_meld(Tile::new_number(Suit::Man, 1, false)));
        hand.melds.push(chi_meld(Suit::Sou, 4));
        hand.melds.push(pon_meld(Tile::new_honor(Honor::Haku)));

        let win = Tile::new_honor(Honor::Ton);
        assert!(
            hand.can_win(&win),
            "副露 3 + 7p8p9p + 東単騎で東を引いて和了"
        );
    }

    /// 副露ありで和了形が組めないケース (バラバラの残り手牌)
    #[test]
    fn test_can_win_with_meld_no_agari() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 4, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 6, false));
        hand.add_tile(Tile::new_number(Suit::Man, 8, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 1, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 3, false));
        hand.add_tile(Tile::new_number(Suit::Man, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 7, false));
        hand.add_tile(Tile::new_number(Suit::Sou, 9, false));
        hand.add_tile(Tile::new_honor(Honor::Ton));
        hand.melds.push(pon_meld(Tile::new_honor(Honor::Haku)));

        let win = Tile::new_honor(Honor::Ton);
        assert!(
            !hand.can_win(&win),
            "副露あり、残り手牌バラバラなら和了不可"
        );
    }

    /// 副露ありの tile_count() が 14 相当 (副露N=2 → 残り手牌 7 + ツモ 1 = 14 枚)
    #[test]
    fn test_tile_count_with_melds() {
        let mut hand = Hand::new();
        for _ in 0..7 {
            hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        }
        hand.melds.push(pon_meld(Tile::new_number(Suit::Man, 1, false)));
        hand.melds.push(chi_meld(Suit::Sou, 4));
        // 残り手牌 7 + 副露 2*3 = 13
        assert_eq!(hand.tile_count(), 13);
    }

    /// 5 枚麻雀: 字牌対子 + 数牌塔子（カンチャン）
    /// 手牌: to to 3p 4p 9m → 9m 捨て、3p 4p で 2p/5p のリャンメン待ち
    #[test]
    fn test_five_tile_waits_honor_pair_plus_ryanmen() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_honor(Honor::Ton));
        hand.add_tile(Tile::new_honor(Honor::Ton));
        hand.add_tile(Tile::new_number(Suit::Pin, 3, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 4, false));
        hand.add_tile(Tile::new_number(Suit::Man, 9, false));

        let waits = hand.five_tile_waits();
        assert!(
            waits.contains(&Tile::new_number(Suit::Pin, 2, false)),
            "リャンメン待ち 2p が含まれる: {:?}",
            waits
        );
        assert!(
            waits.contains(&Tile::new_number(Suit::Pin, 5, false)),
            "リャンメン待ち 5p が含まれる: {:?}",
            waits
        );
    }
}
