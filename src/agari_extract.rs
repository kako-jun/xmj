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
