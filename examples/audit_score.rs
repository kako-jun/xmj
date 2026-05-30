//! #147 OSS対比監査: 参照 (MahjongRepository/mahjong) と同一手牌を xmj で採点する。
//! `cargo run --example audit_score` で id|han|fu|yaku を出力し audit/ref_score.py と diff する。

use xmj_core::hand::Hand;
use xmj_core::scoring::{ScoringContext, ScoringEngine, Yaku};
use xmj_core::tile::{Honor, Suit, Tile};

fn honor_of(d: u8) -> Honor {
    match d {
        1 => Honor::Ton,
        2 => Honor::Nan,
        3 => Honor::Shaa,
        4 => Honor::Pei,
        5 => Honor::Haku,
        6 => Honor::Hatsu,
        _ => Honor::Chun,
    }
}

fn push_suit(v: &mut Vec<Tile>, s: Suit, digits: &str) {
    for c in digits.chars() {
        v.push(Tile::new_number(s, c.to_digit(10).unwrap() as u8, false));
    }
}

fn build_tiles(m: &str, p: &str, s: &str, h: &str) -> Vec<Tile> {
    let mut v = Vec::new();
    push_suit(&mut v, Suit::Man, m);
    push_suit(&mut v, Suit::Pin, p);
    push_suit(&mut v, Suit::Sou, s);
    for c in h.chars() {
        v.push(Tile::new_honor(honor_of(c.to_digit(10).unwrap() as u8)));
    }
    v
}

fn win_tile(ws: char, wv: u8) -> Tile {
    match ws {
        'm' => Tile::new_number(Suit::Man, wv, false),
        'p' => Tile::new_number(Suit::Pin, wv, false),
        's' => Tile::new_number(Suit::Sou, wv, false),
        _ => Tile::new_honor(honor_of(wv)),
    }
}

struct H {
    id: &'static str,
    m: &'static str,
    p: &'static str,
    s: &'static str,
    h: &'static str,
    win: (char, u8),
    tsumo: bool,
    riichi: bool,
    round: char,
    seat: char,
}

fn wind(c: char) -> Honor {
    match c {
        'E' => Honor::Ton,
        'S' => Honor::Nan,
        'W' => Honor::Shaa,
        _ => Honor::Pei,
    }
}

fn main() {
    let hands = vec![
        H { id: "pinfu_tsumo", m: "234", p: "567", s: "23456799", h: "", win: ('s', 7), tsumo: true, riichi: false, round: 'E', seat: 'S' },
        H { id: "pinfu_ron", m: "234", p: "567", s: "23456799", h: "", win: ('s', 7), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "tanyao_ron", m: "234345", p: "678", s: "23455", h: "", win: ('s', 4), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "sanshoku_ron", m: "234", p: "23467899", s: "234", h: "", win: ('p', 8), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "ittsu_ron", m: "123456789", p: "23499", s: "", h: "", win: ('p', 4), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "iipeikou_ron", m: "223344", p: "567", s: "23499", h: "", win: ('s', 4), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "chiitoi_ron", m: "1199", p: "2288", s: "5566", h: "77", win: ('z', 7), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "yakuhai_haku_ron", m: "234", p: "234", s: "234", h: "55511", win: ('p', 2), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "kanchan_riichi_ron", m: "12399", p: "234567", s: "234", h: "", win: ('m', 2), tsumo: false, riichi: true, round: 'E', seat: 'S' },
        H { id: "tanki_riichi_ron", m: "234", p: "234567", s: "234", h: "44", win: ('z', 4), tsumo: false, riichi: true, round: 'E', seat: 'S' },
        H { id: "ankou_term_tsumo", m: "111", p: "234567", s: "23499", h: "", win: ('s', 4), tsumo: true, riichi: false, round: 'E', seat: 'S' },
        H { id: "toitoi_sanankou_ron", m: "11199", p: "333", s: "555", h: "777", win: ('z', 7), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "chinitsu_ron", m: "23423456767899", p: "", s: "", h: "", win: ('m', 9), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "honitsu_ittsu_ron", m: "12345678999", p: "", s: "", h: "111", win: ('m', 9), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "kokushi_ron", m: "19", p: "19", s: "19", h: "12345677", win: ('z', 7), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "suuankou_tsumo", m: "11199", p: "333", s: "555", h: "777", win: ('z', 7), tsumo: true, riichi: false, round: 'E', seat: 'S' },
        H { id: "daisangen_ron", m: "234", p: "99", s: "", h: "555666777", win: ('m', 2), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "chinroutou_tsumo", m: "111999", p: "111999", s: "11", h: "", win: ('s', 1), tsumo: true, riichi: false, round: 'E', seat: 'S' },
        H { id: "tsuuiisou_daisan_ron", m: "", p: "", s: "", h: "11155566677722", win: ('z', 2), tsumo: false, riichi: false, round: 'E', seat: 'S' },
        H { id: "dealer_haneman_tsumo", m: "234", p: "234", s: "23423499", h: "", win: ('s', 4), tsumo: true, riichi: true, round: 'E', seat: 'E' },
    ];

    for hh in &hands {
        let all = build_tiles(hh.m, hh.p, hh.s, hh.h);
        let wt = win_tile(hh.win.0, hh.win.1);
        // 14 枚から win を 1 枚抜いて 13 枚手牌にする
        let mut hand = Hand::new();
        let mut removed = false;
        for t in &all {
            if !removed && *t == wt {
                removed = true;
                continue;
            }
            hand.add_tile(*t);
        }
        let ctx = ScoringContext {
            is_tsumo: hh.tsumo,
            is_riichi: hh.riichi,
            round_wind: wind(hh.round),
            seat_wind: wind(hh.seat),
            allow_open_tanyao: true,
            allow_local_yakuman: false,
            ..ScoringContext::default()
        };
        match ScoringEngine::calculate_score_with_context(&hand, &wt, &ctx) {
            Some(r) => {
                let mut yaku: Vec<String> = r.yaku.iter().map(fmt_yaku).collect();
                yaku.sort();
                println!(
                    "{}|han={}|fu={}|ymax={}|yaku={}",
                    hh.id,
                    r.han,
                    r.fu,
                    r.yakuman_count,
                    yaku.join(",")
                );
            }
            None => println!("{}|NONE", hh.id),
        }
    }
}

fn fmt_yaku(y: &Yaku) -> String {
    format!("{:?}", y)
}
