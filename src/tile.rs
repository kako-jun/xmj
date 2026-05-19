#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Man,    // 萬子 (m)
    Pin,    // 筒子 (p)
    Sou,    // 索子 (s)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Honor {
    Ton,    // 東 (to)
    Nan,    // 南 (na)
    Shaa,   // 西 (sa)
    Pei,    // 北 (pe)
    Haku,   // 白 (hk)
    Hatsu,  // 発 (ht)
    Chun,   // 中 (cn)
}

impl Honor {
    /// 字牌 7 種の全列挙。テンパイ判定や待ち牌候補の走査などで使う。
    pub const ALL: [Honor; 7] = [
        Honor::Ton,
        Honor::Nan,
        Honor::Shaa,
        Honor::Pei,
        Honor::Haku,
        Honor::Hatsu,
        Honor::Chun,
    ];
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileType {
    Number { suit: Suit, value: u8 },
    Honor(Honor),
}

/// 麻雀牌。
///
/// # 同値比較に関する重要な注意
///
/// `is_glass` は「他家から見えるか」という**表示属性**であり、
/// 牌の同値性（和了判定・鳴き判定・赤ドラ判定）には関与しない。
/// そのため `PartialEq` / `Eq` / `Hash` は **`is_glass` を除外して手動実装** している。
/// `derive` を使うと `is_glass` の異なる 2 枚の同種牌が `==` で false になり、
/// `Hand::can_win` や `can_pon` などの既存ロジックが壊れる。
#[derive(Debug, Clone, Copy)]
pub struct Tile {
    pub tile_type: TileType,
    pub is_red: bool,  // 赤ドラ用
    /// 鷲巣麻雀の透明牌フラグ。他家からも種類が見える。
    /// `GameMode::Washizu` で wall 初期化時に 3/4 が true になる。
    /// **同値比較・ハッシュには含めない**（上記コメント参照）。
    pub is_glass: bool,
}

impl PartialEq for Tile {
    fn eq(&self, other: &Self) -> bool {
        // is_glass は表示属性なので比較に含めない
        self.tile_type == other.tile_type && self.is_red == other.is_red
    }
}

impl Eq for Tile {}

impl std::hash::Hash for Tile {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        // is_glass はハッシュにも含めない（Eq と整合性をとる）
        self.tile_type.hash(state);
        self.is_red.hash(state);
    }
}

impl Tile {
    pub fn new_number(suit: Suit, value: u8, is_red: bool) -> Self {
        assert!(value >= 1 && value <= 9, "Invalid tile value: {}", value);
        Self {
            tile_type: TileType::Number { suit, value },
            is_red,
            is_glass: false,
        }
    }

    pub fn new_honor(honor: Honor) -> Self {
        Self {
            tile_type: TileType::Honor(honor),
            is_red: false,
            is_glass: false,
        }
    }

    /// 透明牌フラグを立てたコピーを返す（builder スタイル）。
    /// 鷲巣麻雀の wall 初期化で使う。
    ///
    /// builder 用途では `true` を渡す。`false` は試験用
    /// （`with_glass(false)` で元に戻す経路をテストするためにシグネチャを残している）。
    /// 互換性のため引数 `is_glass: bool` は維持する。
    pub fn with_glass(mut self, is_glass: bool) -> Self {
        self.is_glass = is_glass;
        self
    }

    pub fn to_string(&self) -> String {
        match self.tile_type {
            TileType::Number { suit, value } => {
                let suit_char = match suit {
                    Suit::Man => "m",
                    Suit::Pin => "p", 
                    Suit::Sou => "s",
                };
                if self.is_red {
                    format!("{}{}r", value, suit_char)
                } else {
                    format!("{}{}", value, suit_char)
                }
            }
            TileType::Honor(honor) => {
                match honor {
                    Honor::Ton => "to".to_string(),
                    Honor::Nan => "na".to_string(),
                    Honor::Shaa => "sa".to_string(),
                    Honor::Pei => "pe".to_string(),
                    Honor::Haku => "hk".to_string(),
                    Honor::Hatsu => "ht".to_string(),
                    Honor::Chun => "cn".to_string(),
                }
            }
        }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        if s.len() < 2 {
            return None;
        }

        // 字牌の場合をまず試す
        if let Some(honor) = match s {
            "to" => Some(Honor::Ton),
            "na" => Some(Honor::Nan),
            "sa" => Some(Honor::Shaa),
            "pe" => Some(Honor::Pei),
            "hk" => Some(Honor::Haku),
            "ht" => Some(Honor::Hatsu),
            "cn" => Some(Honor::Chun),
            _ => None,
        } {
            return Some(Self::new_honor(honor));
        }

        // 数牌の場合
        let chars: Vec<char> = s.chars().collect();
        if chars.len() < 2 {
            return None;
        }

        let value = chars[0].to_digit(10)? as u8;
        if value < 1 || value > 9 {
            return None;
        }

        let suit_char = chars[1];
        let is_red = chars.len() == 3 && chars[2] == 'r';

        let suit = match suit_char {
            'm' => Suit::Man,
            'p' => Suit::Pin,
            's' => Suit::Sou,
            _ => return None,
        };

        Some(Self::new_number(suit, value, is_red))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile_creation() {
        let tile = Tile::new_number(Suit::Man, 5, false);
        assert_eq!(tile.to_string(), "5m");

        let red_tile = Tile::new_number(Suit::Pin, 5, true);
        assert_eq!(red_tile.to_string(), "5pr");

        let honor_tile = Tile::new_honor(Honor::Ton);
        assert_eq!(honor_tile.to_string(), "to");
    }

    #[test]
    fn test_tile_from_string() {
        assert_eq!(Tile::from_string("5m").unwrap().to_string(), "5m");
        assert_eq!(Tile::from_string("5mr").unwrap().to_string(), "5mr");
        assert_eq!(Tile::from_string("to").unwrap().to_string(), "to");
        assert!(Tile::from_string("invalid").is_none());
    }

    /// `is_glass` フラグは同値比較に影響しないことを保証する。
    /// 既存の和了判定・鳴き判定・赤ドラ判定の互換性に直結する重要テスト。
    #[test]
    fn test_tile_equality_ignores_glass() {
        let opaque = Tile::new_number(Suit::Man, 5, false);
        let glass = Tile::new_number(Suit::Man, 5, false).with_glass(true);

        assert_eq!(opaque, glass, "is_glass の違いは同値比較に影響しない");

        // ハッシュも一致することを確認（HashMap キーとして使うため）
        use std::collections::HashMap;
        let mut counts = HashMap::new();
        *counts.entry(opaque).or_insert(0) += 1;
        *counts.entry(glass).or_insert(0) += 1;
        assert_eq!(counts.len(), 1, "同種牌は is_glass が違っても同じハッシュキー");
        assert_eq!(counts[&opaque], 2);

        // 字牌でも同様
        let h_opaque = Tile::new_honor(Honor::Ton);
        let h_glass = Tile::new_honor(Honor::Ton).with_glass(true);
        assert_eq!(h_opaque, h_glass);

        // 種類が違えば当然 != （sanity check）
        let other = Tile::new_number(Suit::Pin, 5, false);
        assert_ne!(opaque, other);

        // 赤ドラ違いは != （is_red は同値比較に含める）
        let red = Tile::new_number(Suit::Man, 5, true);
        assert_ne!(opaque, red, "is_red は同値比較に含める（赤ドラは別牌）");
    }

    #[test]
    fn test_with_glass_setter() {
        let t = Tile::new_number(Suit::Sou, 3, false);
        assert!(!t.is_glass);

        let g = t.with_glass(true);
        assert!(g.is_glass);

        // 元の値以外は変わらない
        assert_eq!(g.tile_type, t.tile_type);
        assert_eq!(g.is_red, t.is_red);

        // false に戻せる（試験用途）
        let back = g.with_glass(false);
        assert!(!back.is_glass);
    }
}
