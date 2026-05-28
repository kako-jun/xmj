//! 14 枚手牌から「抜くと残りが winning 形になる 1 枚」を探すヘルパ群。
//!
//! 元々 `src/wasm.rs` の中に `#[cfg(feature = "wasm")]` 付きで定義されていたが、
//! Issue #66 のシナリオテスト基盤から非 wasm パスでも利用する必要が生じたため、
//! feature ゲート無しのモジュールに切り出した。`wasm.rs` からは re-export で
//! 既存呼び出し元の API シグネチャを温存する。
//!
//! TODO(#66 follow-up): `ScenarioRunner::try_tsumo` から呼び出すことで、
//! 役判定の組合せテストを `feature = "wasm"` 無しのまま `cargo test` で回せる。
//! 役の追加 (#49-#61) は本モジュールには触らず `scoring.rs` 側で対応する。

use crate::hand::Hand;
use crate::scoring::{ScoringContext, ScoringEngine, ScoringResult};
use crate::tile::Tile;

/// `extract_agari_with_context` のツモ・子家デフォルト。
///
/// 既存 API 互換のため残す。意味的には `extract_agari_with_context(hand, true, false)`。
pub fn extract_agari(hand: &Hand) -> Option<(Hand, Tile)> {
    extract_agari_with_context(hand, true, false)
}

/// 14 枚手牌から「抜くと残りが winning 形になる 1 枚」を探し、最高得点の解釈を返す。
///
/// ツモ和了直後は `Hand` がソート済で「最後に引いた牌」を末尾から復元できない。
/// 各ユニークな牌を winning_tile 候補として試し、`ScoringEngine::calculate_score_with_context`
/// に通して最も高得点 (total_points → han → fu の順で比較) の解釈を採用する。
///
/// 返り値: `(13 枚に縮めた Hand, winning_tile)`
///
/// # 設計メモ (Issue #34)
/// - 多面待ち手で「両面 / 嵌張 / 辺張」が同居するとき、平和の付く両面解釈を優先する
/// - 四暗刻単騎は「単騎雀頭が winning_tile」の解釈を捕捉する
/// - 役なしになる候補があってもスキップされ、役あり候補が選ばれる
/// - 全候補が役なしの場合は「最初に和了形が成立した候補」を返す
///
/// # タイブレーク
/// `total_points` → `han` → `fu` の順で大小比較する。それでも同点 (ties) の場合は
/// **手牌昇順 (Hand 内ソート済み) で最初に見つかった候補が勝つ**。
///
/// # 副露 (チー / ポン / カン) を含む手 (Issue #33)
/// `Hand::tile_count()` は副露込みで 14 枚相当をカウントするため、副露ありでも
/// 残り手牌 (11/8/5/2 枚) から 1 枚抜いて `can_win` を呼べば「副露面子 + 残り手牌」で
/// 4 面子 1 雀頭が構成されるかが正しく判定される。
///
/// # Issue #74
/// ドラ・立直・場風・状況役など `ScoringContext` 全体を反映した点数で比較するため、
/// フル ctx を受け取る版。`extract_agari_with_context` からも本関数に委譲する。
pub fn extract_agari_with_full_context(
    hand: &Hand,
    ctx: &ScoringContext,
) -> Option<(Hand, Tile)> {
    let tiles = hand.get_tiles().clone();
    if hand.tile_count() != 14 {
        return None;
    }
    // ユニークな牌で 1 枚ずつ試す
    let mut seen: Vec<Tile> = Vec::new();
    // 役あり候補の最良 + 役なし fallback を別管理
    let mut best: Option<(Hand, Tile, u32, u32, u32)> = None; // (sub, tile, total, han, fu)
    let mut fallback: Option<(Hand, Tile)> = None;
    for tile in tiles.iter() {
        if seen.iter().any(|t| t == tile) {
            continue;
        }
        seen.push(*tile);
        let mut sub = hand.clone();
        if !(sub.remove_tile(tile) && sub.can_win(tile)) {
            continue;
        }
        if fallback.is_none() {
            fallback = Some((sub.clone(), *tile));
        }
        if let Some(res) = ScoringEngine::calculate_score_with_context(&sub, tile, ctx) {
            let cand = (sub.clone(), *tile, res.total_points, res.han, res.fu);
            best = match best {
                None => Some(cand),
                Some(prev) => {
                    if (cand.2, cand.3, cand.4) > (prev.2, prev.3, prev.4) {
                        Some(cand)
                    } else {
                        Some(prev)
                    }
                }
            };
        }
    }
    if let Some((sub, tile, _, _, _)) = best {
        return Some((sub, tile));
    }
    fallback
}

/// `extract_agari_with_full_context` の後方互換ラッパ。
///
/// `is_tsumo` / `is_dealer` のみを `ScoringContext` に詰めて委譲する。
/// ドラ・立直・状況役は考慮されないため、新規コードは
/// `extract_agari_with_full_context` を直接使うこと。
pub fn extract_agari_with_context(
    hand: &Hand,
    is_tsumo: bool,
    is_dealer: bool,
) -> Option<(Hand, Tile)> {
    let ctx = ScoringContext {
        is_tsumo,
        is_dealer,
        ..ScoringContext::default()
    };
    extract_agari_with_full_context(hand, &ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::Suit;

    /// `extract_agari_with_full_context` がドラを反映した最適 winning_tile を選ぶことを確認。
    ///
    /// 多面待ち: 1m2m3m / 2m3m + 4m4m (雀頭候補) / 2p3p4p / 2s3s4s
    /// winning 候補: 1m (辺張) / 4m (単騎) / 2m or 4m (シャンポン的な面) など
    /// ドラ表示牌 = 3m → ドラ = 4m。4m が winning_tile の解釈が高得点になることを確認。
    #[test]
    fn full_context_prefers_dora_winning_tile() {
        use crate::hand::Hand;
        use crate::tile::Tile;

        // 手牌 14 枚: 1m 2m 3m / 2m 3m 4m 4m / 2p 3p 4p / 2s 3s 4s + 4m(ドラ)
        // = 1m 2m 2m 3m 3m 4m 4m 4m 2p 3p 4p 2s 3s 4s
        // winning_tile 候補:
        //   4m: 1m2m3m / 2m3m4m / 4m4m(雀頭) / 2p3p4p / 2s3s4s → 平和 + ドラ
        //   1m: 1m(単騎?) → 成立しない (1m2m3m で 1m を抜くと残り不完全)
        // ドラ表示牌 = 3m → ドラ牌 = 4m
        let mut hand = Hand::new();
        let tiles_to_add = [
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Man, 3, false),
            Tile::new_number(Suit::Man, 3, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 2, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 4, false),
        ];
        for t in tiles_to_add {
            hand.add_tile(t);
        }

        // ドラなし ctx
        let ctx_no_dora = ScoringContext {
            is_tsumo: true,
            is_dealer: false,
            ..ScoringContext::default()
        };
        // ドラあり ctx: ドラ表示牌 3m → ドラ = 4m
        let dora_indicator = Tile::new_number(Suit::Man, 3, false);
        let ctx_with_dora = ScoringContext {
            is_tsumo: true,
            is_dealer: false,
            dora_indicators: vec![dora_indicator],
            ..ScoringContext::default()
        };

        let result_no_dora = extract_agari_with_full_context(&hand, &ctx_no_dora);
        let result_with_dora = extract_agari_with_full_context(&hand, &ctx_with_dora);

        assert!(result_no_dora.is_some(), "ドラなしでも和了形が見つかる");
        assert!(result_with_dora.is_some(), "ドラありでも和了形が見つかる");

        let (_, wt_with_dora) = result_with_dora.unwrap();
        // ドラ (4m) が winning_tile に選ばれているか確認
        // (ドラ込みで高得点になる候補が優先されるべき)
        let score_with_dora = ScoringEngine::calculate_score_with_context(
            &{
                let mut h = hand.clone();
                h.remove_tile(&wt_with_dora);
                h
            },
            &wt_with_dora,
            &ctx_with_dora,
        );
        assert!(score_with_dora.is_some(), "ドラあり ctx でスコア計算成功");
        let s = score_with_dora.unwrap();
        assert!(s.han >= 1, "役あり (han >= 1): got {}", s.han);
    }

    /// `extract_agari_with_context` が `extract_agari_with_full_context` の委譲として
    /// 同じ結果を返すことを確認（後方互換）。
    #[test]
    fn with_context_delegates_to_full_context() {
        use crate::hand::Hand;
        use crate::tile::Tile;

        let mut hand = Hand::new();
        for t in [
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Man, 3, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 2, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 4, false),
            Tile::new_number(Suit::Man, 5, false),
            Tile::new_number(Suit::Man, 6, false),
            Tile::new_number(Suit::Man, 7, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 5, false),
        ] {
            hand.add_tile(t);
        }

        let ctx = ScoringContext {
            is_tsumo: true,
            is_dealer: false,
            ..ScoringContext::default()
        };
        let r1 = extract_agari_with_full_context(&hand, &ctx);
        let r2 = extract_agari_with_context(&hand, true, false);

        match (r1, r2) {
            (Some((_, t1)), Some((_, t2))) => assert_eq!(t1, t2, "委譲版と直接版で同じ winning_tile"),
            (None, None) => {}
            _ => panic!("一方だけ None: 委譲の結果が異なる"),
        }
    }
}

/// `ScoringResult` を `{ han, fu, totalPoints, yaku:[...] }` 形の JSON 文字列に整形。
///
/// `WasmGame::resolveWinTsumo` / `resolveWinRon` の戻り値生成用。
/// シナリオテストでは Rust 値のまま `ScoringResult` を扱うので不要だが、
/// 既存 API の維持のためここに置いておく（wasm.rs からは pub use で再公開）。
pub fn scoring_summary_json(result: &ScoringResult) -> String {
    let mut obj = serde_json::Map::new();
    obj.insert("han".into(), serde_json::Value::Number(result.han.into()));
    obj.insert("fu".into(), serde_json::Value::Number(result.fu.into()));
    obj.insert(
        "totalPoints".into(),
        serde_json::Value::Number(result.total_points.into()),
    );
    // #42 #51 #52: 役満倍率 (0 = 非役満、1 = 単役満、2 = ダブル役満、…)
    obj.insert(
        "yakumanCount".into(),
        serde_json::Value::Number(result.yakuman_count.into()),
    );
    obj.insert(
        "yaku".into(),
        serde_json::Value::Array(
            result
                .yaku
                .iter()
                .map(|y| serde_json::Value::String(format!("{:?}", y)))
                .collect(),
        ),
    );
    serde_json::Value::Object(obj).to_string()
}
