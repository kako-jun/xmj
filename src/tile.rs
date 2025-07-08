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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TileType {
    Number { suit: Suit, value: u8 },
    Honor(Honor),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Tile {
    pub tile_type: TileType,
    pub is_red: bool,  // 赤ドラ用
}

impl Tile {
    pub fn new_number(suit: Suit, value: u8, is_red: bool) -> Self {
        assert!(value >= 1 && value <= 9, "Invalid tile value: {}", value);
        Self {
            tile_type: TileType::Number { suit, value },
            is_red,
        }
    }

    pub fn new_honor(honor: Honor) -> Self {
        Self {
            tile_type: TileType::Honor(honor),
            is_red: false,
        }
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
}