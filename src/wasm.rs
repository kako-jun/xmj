//! WebAssembly bindings for xmj麻雀ゲーム

#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
use crate::{Game, Tile, Hand, Player, AiEngine, AiLevel};
#[cfg(feature = "wasm")]
use crate::game::{RoundOutcome, WinKind};
#[cfg(feature = "wasm")]
use crate::scoring::ScoringEngine;
// extract_agari* / scoring_summary_json は Issue #66 で `agari_extract` モジュールに切り出した。
// wasm.rs からは re-export で呼び出し側の `crate::wasm::extract_agari` 等を温存する。
#[cfg(feature = "wasm")]
pub(crate) use crate::agari_extract::{
    extract_agari, extract_agari_with_context, extract_agari_with_full_context,
};

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmGame {
    game: Game,
    human_player_index: Option<usize>, // ハイブリッドモード用：人間プレイヤーの位置（0-3）
}

/// #143 本場縛りブロック専用のセンチネル戻り値。
///
/// `resolve_win_*` は「役無し・和了形不成立」と「本場縛りで弾かれた合法和了」を
/// 区別する必要がある。前者は従来通り空文字 `""` を返すが、後者はこの JSON を返す。
/// UI 側 (parseSummaryAsWin / App.ts) はこのセンチネルを検知して
/// 「和了形不成立」ではなく「本場縛り未達」のメッセージに分岐する。
/// なお resolve_win 自体は呼ばれない (点数移動は発生しない)。
#[cfg(feature = "wasm")]
pub(crate) const SHIBARI_BLOCKED_JSON: &str = "{\"shibariBlocked\":true}";

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmGame {
    #[wasm_bindgen(constructor)]
    pub fn new(player_names: Vec<String>) -> Self {
        // パニック時にコンソールにログを出力
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        Self {
            game: Game::new(player_names),
            human_player_index: None, // 通常モードは全員人間
        }
    }

    /// ハイブリッドゲームを作成（1人間 + 3CPU）
    #[wasm_bindgen(js_name = newHybrid)]
    pub fn new_hybrid(human_name: String, human_position: usize) -> Self {
        // パニック時にコンソールにログを出力
        #[cfg(feature = "console_error_panic_hook")]
        console_error_panic_hook::set_once();

        let mut names = vec![
            "CPU 東".to_string(),
            "CPU 南".to_string(),
            "CPU 西".to_string(),
            "CPU 北".to_string(),
        ];

        // 人間プレイヤーの位置を設定（0=東, 1=南, 2=西, 3=北）
        let position = human_position % 4;
        names[position] = human_name;

        let game = Game::new(names);

        Self {
            game,
            human_player_index: Some(position),
        }
    }

    /// 現在のプレイヤーが人間かどうか
    #[wasm_bindgen(js_name = isCurrentPlayerHuman)]
    pub fn is_current_player_human(&self) -> bool {
        match self.human_player_index {
            Some(human_idx) => self.game.current_player == human_idx,
            None => true, // ハイブリッドモードでない場合は全員人間扱い
        }
    }

    /// 現在のプレイヤーが牌をツモする
    #[wasm_bindgen(js_name = drawTile)]
    pub fn draw_tile(&mut self) -> bool {
        self.game.current_player_draw()
    }

    /// 牌を打牌する
    #[wasm_bindgen(js_name = discardTile)]
    pub fn discard_tile(&mut self, tile_str: &str) -> bool {
        if let Some(tile) = Tile::from_string(tile_str) {
            self.game.discard_tile(tile)
        } else {
            false
        }
    }

    /// ゲーム状態を取得（JSON文字列）
    #[wasm_bindgen(js_name = getGameState)]
    pub fn get_game_state(&self) -> String {
        self.game.get_game_state_string()
    }

    /// 現在のプレイヤーの手牌を取得
    #[wasm_bindgen(js_name = getCurrentHand)]
    pub fn get_current_hand(&self) -> String {
        self.game.get_current_player().get_hand_string()
    }

    /// 現在のプレイヤーのシャンテン数を取得
    #[wasm_bindgen(js_name = getShanten)]
    pub fn get_shanten(&self) -> i32 {
        self.game.get_current_player().hand.shanten()
    }

    /// 山牌の残り枚数を取得
    #[wasm_bindgen(js_name = getWallCount)]
    pub fn get_wall_count(&self) -> usize {
        self.game.get_wall_count()
    }

    /// ゲームが終了したかチェック
    #[wasm_bindgen(js_name = isGameOver)]
    pub fn is_game_over(&self) -> bool {
        self.game.is_game_over()
    }

    /// 現在のプレイヤーIDを取得
    #[wasm_bindgen(js_name = getCurrentPlayerId)]
    pub fn get_current_player_id(&self) -> usize {
        self.game.current_player
    }

    /// チー可能かチェック
    #[wasm_bindgen(js_name = canChi)]
    pub fn can_chi(&self, player_idx: usize) -> bool {
        self.game.can_chi(player_idx)
    }

    /// ポン可能かチェック
    #[wasm_bindgen(js_name = canPon)]
    pub fn can_pon(&self, player_idx: usize) -> bool {
        self.game.can_pon(player_idx)
    }

    /// カン可能かチェック
    #[wasm_bindgen(js_name = canKan)]
    pub fn can_kan(&self, player_idx: usize) -> bool {
        self.game.can_kan(player_idx)
    }

    /// チーを実行
    #[wasm_bindgen(js_name = doChi)]
    pub fn do_chi(&mut self, player_idx: usize, pattern: usize) -> bool {
        self.game.do_chi(player_idx, pattern)
    }

    /// ポンを実行
    #[wasm_bindgen(js_name = doPon)]
    pub fn do_pon(&mut self, player_idx: usize) -> bool {
        self.game.do_pon(player_idx)
    }

    /// カンを実行
    #[wasm_bindgen(js_name = doKan)]
    pub fn do_kan(&mut self, player_idx: usize) -> bool {
        self.game.do_kan(player_idx)
    }

    /// 暗槓可能な牌一覧を返す (空白区切り tile-string)。
    ///
    /// 例: 手牌に 8m が 4 枚揃っていれば `"8m"`、複数候補がある場合は
    /// 空白で連結 (`"8m 5p"`)。候補なしは空文字。
    /// TS 側は `splitTiles` 系ヘルパで分解する想定。
    #[wasm_bindgen(js_name = canAnkan)]
    pub fn can_ankan(&self, player_idx: usize) -> String {
        if player_idx >= self.game.players.len() {
            return String::new();
        }
        self.game
            .can_ankan(player_idx)
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// 暗槓を実行
    #[wasm_bindgen(js_name = doAnkan)]
    pub fn do_ankan(&mut self, player_idx: usize, tile_str: &str) -> bool {
        let Some(tile) = Tile::from_string(tile_str) else {
            return false;
        };
        self.game.do_ankan(player_idx, tile)
    }

    /// 加槓可能な牌一覧を返す (空白区切り tile-string)。
    /// Pon 副露と同じ牌が手牌に 1 枚以上あれば候補。
    #[wasm_bindgen(js_name = canShouminkan)]
    pub fn can_shouminkan(&self, player_idx: usize) -> String {
        if player_idx >= self.game.players.len() {
            return String::new();
        }
        self.game
            .can_shouminkan(player_idx)
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// 加槓宣言を開始する。pending_chankan を立てて槍槓ロン候補を返す。
    ///
    /// 戻り値 JSON: `{"ok": bool, "candidates": [player_idx, ...]}`
    /// - ok=false: 宣言不可 (候補に含まれない / 既に pending 中等)
    /// - candidates: 当該 tile でロンできる他家の座席 index 一覧
    ///   (UI 側は空なら即 completeShouminkan を呼んでよい)
    #[wasm_bindgen(js_name = startShouminkan)]
    pub fn start_shouminkan(&mut self, player_idx: usize, tile_str: &str) -> String {
        let Some(tile) = Tile::from_string(tile_str) else {
            return r#"{"ok":false,"candidates":[]}"#.to_string();
        };
        if !self.game.start_shouminkan(player_idx, tile) {
            return r#"{"ok":false,"candidates":[]}"#.to_string();
        }
        // 候補列挙: 当該 tile でロン可能な他家
        // 槍槓は通常ロンと同じ Player::can_win 判定でよい (フリテン考慮)
        let mut candidates: Vec<usize> = Vec::new();
        for (i, p) in self.game.players.iter().enumerate() {
            if i == player_idx {
                continue;
            }
            // 国士無双以外は加槓に対する槍槓を許す。本実装では Player::can_win で
            // 一律判定する (国士の暗槓不可ルールは別 Issue で扱う)。
            if !p.is_furiten() && p.can_win(&tile) {
                candidates.push(i);
            }
        }
        let mut obj = serde_json::Map::new();
        obj.insert("ok".into(), serde_json::Value::Bool(true));
        obj.insert(
            "candidates".into(),
            serde_json::Value::Array(
                candidates
                    .iter()
                    .map(|i| serde_json::Value::Number((*i).into()))
                    .collect(),
            ),
        );
        serde_json::Value::Object(obj).to_string()
    }

    /// 加槓を完了する (誰もロン宣言しなかった場合)。
    /// 内部で Pon meld → Kan meld 書き換え + 嶺上ツモ + 槓ドラ追加。
    #[wasm_bindgen(js_name = completeShouminkan)]
    pub fn complete_shouminkan(&mut self, player_idx: usize, tile_str: &str) -> bool {
        let Some(tile) = Tile::from_string(tile_str) else {
            return false;
        };
        self.game.complete_shouminkan(player_idx, tile)
    }

    /// 加槓をキャンセルする (誰かが槍槓ロンを宣言した場合に呼ぶ)。
    /// `pending_chankan` を None に戻すだけのべき等な API。
    #[wasm_bindgen(js_name = cancelShouminkan)]
    pub fn cancel_shouminkan(&mut self) {
        self.game.cancel_shouminkan();
    }

    /// 加槓中の牌に対するロン (槍槓) を確定する。
    ///
    /// `pending_chankan` の tile を winning_tile として使い、
    /// is_chankan=true の ScoringContext で点数を計算する。
    /// 成功すると `pending_chankan` は自動的にクリアされる
    /// (`build_scoring_context` で is_chankan を参照するため、
    /// `resolve_win` 後に明示的にクリアする)。
    #[wasm_bindgen(js_name = resolveWinChankan)]
    pub fn resolve_win_chankan(&mut self, winner_idx: usize, from_idx: usize) -> String {
        if winner_idx >= self.game.players.len() || from_idx >= self.game.players.len() {
            return String::new();
        }
        let Some(winning_tile) = self.game.pending_chankan else {
            return String::new();
        };
        let ctx = self.game.build_scoring_context(winner_idx, false);
        let hand = &self.game.players[winner_idx].hand;
        if !hand.can_win(&winning_tile) {
            return String::new();
        }
        let Some(result) = ScoringEngine::calculate_score_with_context(hand, &winning_tile, &ctx) else {
            return String::new();
        };
        // #61 本場縛り: 最低点数縛りを満たさない和了は無効
        // #143 縛りブロックは「和了形不成立」と区別するためセンチネルを返す
        if !self.game.meets_shibari(&result) {
            return SHIBARI_BLOCKED_JSON.to_string();
        }
        let summary = scoring_summary_json(&result);
        self.game
            .resolve_win(winner_idx, WinKind::Ron { from: from_idx }, result);
        // 槍槓ロン後は加槓宣言を取り消し扱いにする (加槓自体は無効になる)
        self.game.pending_chankan = None;
        summary
    }

    /// CPU（AI）のターンを実行
    #[wasm_bindgen(js_name = executeCpuTurn)]
    pub fn execute_cpu_turn(&mut self) -> String {
        // ツモ
        if !self.game.current_player_draw() {
            return "山牌がありません".to_string();
        }

        // AIで打牌選択
        let ai = AiEngine::new(AiLevel::Intermediate);
        let hand = &self.game.get_current_player().hand;

        if let Some(discard_tile) = ai.select_discard(hand) {
            let tile_str = discard_tile.to_string();
            self.game.discard_tile(discard_tile);
            tile_str
        } else {
            "打牌できません".to_string()
        }
    }

    /// 現在のプレイヤーがCPUかどうか
    #[wasm_bindgen(js_name = isCurrentPlayerCpu)]
    pub fn is_current_player_cpu(&self) -> bool {
        match self.human_player_index {
            Some(human_idx) => self.game.current_player != human_idx,
            None => self.game.current_player != 0, // 通常モード（後方互換性）
        }
    }

    /// プレイヤーの点数を取得
    #[wasm_bindgen(js_name = getPlayerScore)]
    pub fn get_player_score(&self, player_idx: usize) -> i32 {
        if player_idx < self.game.players.len() {
            self.game.players[player_idx].score
        } else {
            0
        }
    }

    /// プレイヤー名を取得
    #[wasm_bindgen(js_name = getPlayerName)]
    pub fn get_player_name(&self, player_idx: usize) -> String {
        if player_idx < self.game.players.len() {
            self.game.players[player_idx].name.clone()
        } else {
            "Unknown".to_string()
        }
    }

    /// ドラ表示牌を取得
    #[wasm_bindgen(js_name = getDoraIndicators)]
    pub fn get_dora_indicators(&self) -> String {
        self.game
            .get_dora_indicators()
            .iter()
            .map(|t| t.to_string())
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// プレイヤーの河（捨て牌）を取得
    #[wasm_bindgen(js_name = getPlayerDiscards)]
    pub fn get_player_discards(&self, player_idx: usize) -> String {
        if player_idx < self.game.players.len() {
            self.game.players[player_idx].get_discards_string()
        } else {
            String::new()
        }
    }

    /// プレイヤーの副露 (鳴き面子) を JSON で取得する (#83 副露表示)。
    ///
    /// 戻り値 JSON 配列の各要素:
    /// ```json
    /// {
    ///   "kind": "chi" | "pon" | "ankan" | "minkan" | "kakan",
    ///   "tiles": ["1m", "2m", "3m"],
    ///   "fromOffset": 0|1|2|3|null,   // 自家から見た鳴き元の相対 offset
    ///   "claimedIndex": 0|1|2|null     // tiles[claimedIndex] が他家から取った牌
    /// }
    /// ```
    /// - `kind` は `MeldType` + `is_open` + `is_kakan` から決定する。
    /// - `fromOffset = (from_player - player_idx + 4) % 4`。暗槓のときは null。
    /// - 副露が無い / player_idx 不正は `"[]"`。
    #[wasm_bindgen(js_name = getPlayerMelds)]
    pub fn get_player_melds(&self, player_idx: usize) -> String {
        if player_idx >= self.game.players.len() {
            return "[]".to_string();
        }
        let melds = self.game.players[player_idx].hand.get_melds();
        let mut arr: Vec<serde_json::Value> = Vec::with_capacity(melds.len());
        for m in melds {
            let kind = match m.meld_type {
                crate::hand::MeldType::Chi => "chi",
                crate::hand::MeldType::Pon => "pon",
                crate::hand::MeldType::Kan => {
                    if !m.is_open {
                        "ankan"
                    } else if m.is_kakan {
                        "kakan"
                    } else {
                        "minkan"
                    }
                }
            };
            let tiles: Vec<serde_json::Value> = m
                .tiles
                .iter()
                .map(|t| serde_json::Value::String(t.to_string()))
                .collect();
            let from_offset = match m.from_player {
                Some(from) => serde_json::Value::Number(
                    (((from + 4) - player_idx) % 4).into(),
                ),
                None => serde_json::Value::Null,
            };
            let claimed_index = match m.claimed_index {
                Some(i) => serde_json::Value::Number(i.into()),
                None => serde_json::Value::Null,
            };
            let mut obj = serde_json::Map::new();
            obj.insert("kind".into(), serde_json::Value::String(kind.into()));
            obj.insert("tiles".into(), serde_json::Value::Array(tiles));
            obj.insert("fromOffset".into(), from_offset);
            obj.insert("claimedIndex".into(), claimed_index);
            arr.push(serde_json::Value::Object(obj));
        }
        serde_json::Value::Array(arr).to_string()
    }

    /// リーチ可能かチェック (#91)
    ///
    /// `Player::can_riichi()` (門前 / テンパイ / 持ち点 1000 以上 / 未リーチ) に加え、
    /// 山牌残り 4 枚以上の麻雀標準ルールも `Game::can_riichi` 経由で担保する。
    /// これにより UI 側の canRiichi=true / declareRiichi=false の食い違いを防ぐ。
    #[wasm_bindgen(js_name = canRiichi)]
    pub fn can_riichi(&self) -> bool {
        let idx = self.game.current_player;
        self.game.can_riichi(idx)
    }

    /// リーチを宣言
    #[wasm_bindgen(js_name = declareRiichi)]
    pub fn declare_riichi(&mut self) -> bool {
        let current_idx = self.game.current_player;
        self.game.declare_riichi(current_idx)
    }

    /// #60 オープンリーチを宣言する。通常立直 + 手牌公開 + 和了時 +1 飜。
    #[wasm_bindgen(js_name = declareOpenRiichi)]
    pub fn declare_open_riichi(&mut self) -> bool {
        let current_idx = self.game.current_player;
        self.game.declare_open_riichi(current_idx)
    }

    /// #60 指定プレイヤーがオープンリーチしているか。
    #[wasm_bindgen(js_name = isPlayerOpenRiichi)]
    pub fn is_player_open_riichi(&self, player_idx: usize) -> bool {
        player_idx < self.game.players.len() && self.game.players[player_idx].open_riichi
    }

    /// プレイヤーがリーチしているかチェック
    #[wasm_bindgen(js_name = isPlayerRiichi)]
    pub fn is_player_riichi(&self, player_idx: usize) -> bool {
        if player_idx < self.game.players.len() {
            self.game.players[player_idx].is_riichi
        } else {
            false
        }
    }

    // ==================== Round loop (Issue #27) ====================
    //
    // 局結着 (`resolve_win` / `resolve_draw`) 〜 次局遷移 (`next_round`) を
    // Web UI から駆動するための bridge。`get_last_outcome_json` は UI が
    // 直前局の結果画面（和了 / 流局）を描画するための JSON を返す。
    //
    // closed: #28 (誠京役満ご祝儀の自動授受)、#29 (東西戦 team_yaku 自動呼出) は
    //   別 PR で resolve_win 側に配線済み。本 ファイル の bridge 層では追加処理不要。

    /// 山牌 0 / 全員ノーテンの簡易流局。
    /// 呼び出し側がテンパイ者の座席 index を渡す。
    /// テンパイ者の自動算出は `compute_tenpai_players` を併用する。
    #[wasm_bindgen(js_name = resolveDraw)]
    pub fn resolve_draw(&mut self, tenpai_player_indices: Vec<usize>) {
        self.game.resolve_draw(tenpai_player_indices);
    }

    /// 流局時のテンパイ者の座席 index を全プレイヤーから抽出する。
    /// `Player::is_tenpai()` で判定し、テンパイしているプレイヤーの index 配列を返す。
    /// `resolve_draw` に渡してノーテン罰符を正しく徴収するための補助 API。
    #[wasm_bindgen(js_name = computeTenpaiPlayers)]
    pub fn compute_tenpai_players(&self) -> Vec<usize> {
        self.game.compute_tenpai_players()
    }

    /// 指定プレイヤーがツモ和了可能か。
    ///
    /// 手牌が 14 枚相当（ツモ直後）であり、`extract_agari` が
    /// 「抜くと残りで `can_win` 成立」の 1 枚を見つけられる場合のみ true。
    /// UI 側はこれが true のときだけ「ツモ」ボタンを enable する。
    #[wasm_bindgen(js_name = canTsumo)]
    pub fn can_tsumo(&self, player_idx: usize) -> bool {
        if player_idx >= self.game.players.len() {
            return false;
        }
        let hand = &self.game.players[player_idx].hand;
        if hand.tile_count() != 14 {
            return false;
        }
        extract_agari(hand).is_some()
    }

    /// 指定プレイヤーが直前の打牌に対してロン可能か。
    ///
    /// 条件: 直前打牌者ではなく、`last_discard` が存在し、闇牌で隠蔽されておらず、
    /// フリテン状態でもなく、`Player::can_win(last_discard)` が true。
    /// `Game::can_someone_win` の単一プレイヤー版 + フリテン消費 (Issue #56)。
    #[wasm_bindgen(js_name = canRon)]
    pub fn can_ron(&self, player_idx: usize) -> bool {
        if player_idx >= self.game.players.len() {
            return false;
        }
        if player_idx == self.game.current_player {
            return false;
        }
        if self.game.last_discard_hidden {
            return false;
        }
        let Some(tile) = self.game.last_discard else {
            return false;
        };
        // Issue #56: フリテン (通常 / 同巡 / 立直後永続) ならロン不可
        if self.game.players[player_idx].is_furiten() {
            return false;
        }
        self.game.players[player_idx].can_win(&tile)
    }

    /// 直前打牌に対するロンを見逃したことを宣言する (Issue #56)。
    ///
    /// 呼び出し側 (TS の `skipMeldCall` 等) は「`can_ron(player_idx)` が true の状態で
    /// ロンを選ばずに通常進行に戻した」場面でのみ本 API を呼ぶ。本関数自体は
    /// `can_ron` の再判定はせず、`Player::notify_ron_skipped` を呼んでフラグを立てる
    /// だけのべき等な API。
    /// - 同巡フリテン: 当該プレイヤーの `skipped_ron_this_turn = true`
    /// - 立直済みなら永続フリテン: `permanent_furiten = true`
    #[wasm_bindgen(js_name = skipRon)]
    pub fn skip_ron(&mut self, player_idx: usize) {
        if player_idx >= self.game.players.len() {
            return;
        }
        self.game.players[player_idx].notify_ron_skipped();
    }

    /// 直前打牌者の座席 index を返す。ロン宣言の `from_idx` 引数に渡すための補助。
    /// `last_discard` が存在しない場合は `None`（JS 側は `undefined`）。
    #[wasm_bindgen(js_name = getLastDiscarder)]
    pub fn get_last_discarder(&self) -> Option<usize> {
        // #77: last_discarder フィールドを直接返す。
        // 旧実装の (current_player + 3) % 4 計算式は門前進行前提の暫定実装で、
        // 鳴きで current_player が任意席に飛ぶと誤判定が発生していた。
        self.game.last_discarder
    }

    /// ツモ和了を確定する。`ScoringResult` は本関数内で `ScoringEngine` に計算させる。
    ///
    /// winning_tile の決定:
    ///   `Hand` は `add_tile` 時に自動ソートされるため「最後に引いた牌」を
    ///   末尾位置から復元できない。代わりに「手牌から 1 枚抜いて
    ///   `can_win(その牌)` が true になる牌」を winning_tile とみなす。
    ///   候補が複数あっても点数計算には大きく影響しないため最初の候補を採用。
    ///
    /// 戻り値: 計算できた `ScoringResult` のサマリ JSON
    ///   `{ "han": n, "fu": n, "totalPoints": n, "yaku": [...] }`
    ///   和了形でなければ "" を返す（呼び出し側の安全網）。
    #[wasm_bindgen(js_name = resolveWinTsumo)]
    pub fn resolve_win_tsumo(&mut self, winner_idx: usize) -> String {
        if winner_idx >= self.game.players.len() {
            return String::new();
        }
        let hand_clone = self.game.players[winner_idx].hand.clone();
        // #74: ctx を先に構築し、ドラ/立直/状況役を考慮した full context で最適 winning_tile を選ぶ
        let ctx = self.game.build_scoring_context(winner_idx, true);
        let Some((sub_hand, winning_tile)) = extract_agari_with_full_context(&hand_clone, &ctx) else {
            return String::new();
        };
        // #49/#50/#53/#54: 立直系・状況役・場風自風・ドラを反映した ScoringContext を組む
        let Some(result) = ScoringEngine::calculate_score_with_context(&sub_hand, &winning_tile, &ctx) else {
            return String::new();
        };
        // #61 本場縛り: 最低点数縛りを満たさない和了は無効
        // #143 縛りブロックは「和了形不成立」と区別するためセンチネルを返す
        if !self.game.meets_shibari(&result) {
            return SHIBARI_BLOCKED_JSON.to_string();
        }
        let summary = scoring_summary_json(&result);
        self.game
            .resolve_win(winner_idx, WinKind::Tsumo, result);
        summary
    }

    /// ロン和了を確定する。打牌者は `from_idx` で指定。
    /// winning_tile は `game.last_discard` を使用する。
    #[wasm_bindgen(js_name = resolveWinRon)]
    pub fn resolve_win_ron(&mut self, winner_idx: usize, from_idx: usize) -> String {
        if winner_idx >= self.game.players.len() || from_idx >= self.game.players.len() {
            return String::new();
        }
        let Some(winning_tile) = self.game.last_discard else {
            return String::new();
        };
        // #75: pending_chankan と winning_tile が一致するときだけ is_chankan=true になるよう
        // winning_tile を渡す版で ScoringContext を構築する。
        let ctx = self.game.build_scoring_context_with_tile(winner_idx, false, Some(&winning_tile));
        let hand = &self.game.players[winner_idx].hand;
        if !hand.can_win(&winning_tile) {
            return String::new();
        }
        let Some(result) = ScoringEngine::calculate_score_with_context(hand, &winning_tile, &ctx) else {
            return String::new();
        };
        // #61 本場縛り: 最低点数縛りを満たさない和了は無効
        // #143 縛りブロックは「和了形不成立」と区別するためセンチネルを返す
        if !self.game.meets_shibari(&result) {
            return SHIBARI_BLOCKED_JSON.to_string();
        }
        let summary = scoring_summary_json(&result);
        self.game
            .resolve_win(winner_idx, WinKind::Ron { from: from_idx }, result);
        summary
    }

    /// 次の局へ。戻り値: true = 続行 / false = 対局終了。
    #[wasm_bindgen(js_name = nextRound)]
    pub fn next_round(&mut self) -> bool {
        self.game.next_round()
    }

    /// #89: 嘘リーチ（黙聴での虚偽リーチ）を許可するかどうかを設定する。
    /// true のとき canRiichi のテンパイ・点数要件を外し、門前 + 未リーチのみで宣言可能にする。
    /// 嘘リーチに追加の罰符は無く、流局で露見した場合の損失は普通の不成立リーチと同じ
    /// （リーチ棒 1000 点の供託没収 + テンパイ外なのでノーテン罰符）。
    #[wasm_bindgen(js_name = setUsoRiichiEnabled)]
    pub fn set_uso_riichi_enabled(&mut self, enabled: bool) {
        self.game.uso_riichi_enabled = enabled;
    }

    /// #89: 嘘リーチ設定の現在値を返す。
    #[wasm_bindgen(js_name = isUsoRiichiEnabled)]
    pub fn is_uso_riichi_enabled(&self) -> bool {
        self.game.uso_riichi_enabled
    }

    /// #89: 指定プレイヤーが嘘リーチ中かどうかを返す（流局時の手牌公開判定に使用）。
    #[wasm_bindgen(js_name = isUsoRiichi)]
    pub fn is_uso_riichi(&self, player_idx: usize) -> bool {
        if player_idx >= self.game.players.len() {
            return false;
        }
        self.game.players[player_idx].uso_riichi
    }

    /// #80: 手牌自動ソート（理牌）の ON/OFF を設定する。
    /// true（デフォルト）のとき ツモ後に自動で手牌を整列する。
    #[wasm_bindgen(js_name = setAutoSort)]
    pub fn set_auto_sort(&mut self, enabled: bool) {
        self.game.auto_sort = enabled;
    }

    /// #80: 手牌自動ソート設定の現在値。
    #[wasm_bindgen(js_name = isAutoSortEnabled)]
    pub fn is_auto_sort_enabled(&self) -> bool {
        self.game.auto_sort
    }

    /// #80: 現在の手番プレイヤーの手牌を即時ソートする。
    #[wasm_bindgen(js_name = sortCurrentHand)]
    pub fn sort_current_hand(&mut self) {
        let idx = self.game.current_player;
        if idx < self.game.players.len() {
            self.game.players[idx].hand.sort_hand();
        }
    }

    /// #81: 人間プレイヤーのターン開始時に自動ツモを行うかどうかを設定する。
    /// true（デフォルト）= 現状動作。false = 手動ツモが必要。
    #[wasm_bindgen(js_name = setAutoDraw)]
    pub fn set_auto_draw(&mut self, enabled: bool) {
        self.game.auto_draw = enabled;
    }

    /// #81: 自動ツモ設定の現在値。
    #[wasm_bindgen(js_name = isAutoDrawEnabled)]
    pub fn is_auto_draw_enabled(&self) -> bool {
        self.game.auto_draw
    }

    /// #59: 食い替え禁止を強制するかどうかを設定する。
    /// true（デフォルト）= 鳴き直後に現物 / 筋を切れない。false = ローカルルールで許可。
    #[wasm_bindgen(js_name = setEnforceKuikae)]
    pub fn set_enforce_kuikae(&mut self, enabled: bool) {
        self.game.enforce_kuikae = enabled;
    }

    /// #59: 食い替え禁止設定の現在値。
    #[wasm_bindgen(js_name = isEnforceKuikae)]
    pub fn is_enforce_kuikae(&self) -> bool {
        self.game.enforce_kuikae
    }

    /// #129: 喰いタン (鳴きタンヤオ) を認めるかを設定する。
    /// true（デフォルト）= 非門前でも断么九有効。false = 非門前は無効。
    #[wasm_bindgen(js_name = setAllowOpenTanyao)]
    pub fn set_allow_open_tanyao(&mut self, allowed: bool) {
        self.game.allow_open_tanyao = allowed;
    }

    /// #129: 喰いタン設定の現在値。
    #[wasm_bindgen(js_name = isAllowOpenTanyao)]
    pub fn is_allow_open_tanyao(&self) -> bool {
        self.game.allow_open_tanyao
    }

    /// #58: ローカル役満 (人和/大車輪/四連刻/百万石/三連刻) を認めるかを設定する。
    /// デフォルト false。
    #[wasm_bindgen(js_name = setAllowLocalYakuman)]
    pub fn set_allow_local_yakuman(&mut self, allowed: bool) {
        self.game.allow_local_yakuman = allowed;
    }

    /// #58: ローカル役満設定の現在値。
    #[wasm_bindgen(js_name = isAllowLocalYakuman)]
    pub fn is_allow_local_yakuman(&self) -> bool {
        self.game.allow_local_yakuman
    }

    /// #61: 本場縛りルールを設定する。
    /// 0 = Standard（1飜縛り）/ 1 = 5本場以降2飜縛り / 2 = 5本場以降満貫縛り /
    /// 3 = 7本場以降役満縛り。不正値は Standard 扱い。
    #[wasm_bindgen(js_name = setShibariRule)]
    pub fn set_shibari_rule(&mut self, rule: u8) {
        use crate::game::ShibariRule;
        self.game.shibari_rule = match rule {
            1 => ShibariRule::TwoHanFromFiveHonba,
            2 => ShibariRule::ManganFromFiveHonba,
            3 => ShibariRule::YakumanFromSevenHonba,
            _ => ShibariRule::Standard,
        };
    }

    /// #61: 現在の本場縛りルール (上記の数値) を返す。
    #[wasm_bindgen(js_name = getShibariRule)]
    pub fn get_shibari_rule(&self) -> u8 {
        use crate::game::ShibariRule;
        match self.game.shibari_rule {
            ShibariRule::Standard => 0,
            ShibariRule::TwoHanFromFiveHonba => 1,
            ShibariRule::ManganFromFiveHonba => 2,
            ShibariRule::YakumanFromSevenHonba => 3,
        }
    }

    /// #57: 包 (責任払い) を適用するかを設定する。デフォルト true (標準ルール)。
    #[wasm_bindgen(js_name = setEnforcePao)]
    pub fn set_enforce_pao(&mut self, enabled: bool) {
        self.game.enforce_pao = enabled;
    }

    /// #57: 包設定の現在値。
    #[wasm_bindgen(js_name = isEnforcePao)]
    pub fn is_enforce_pao(&self) -> bool {
        self.game.enforce_pao
    }

    /// #118: 割れ目プレイヤーを設定する。0-3 で指定、負値 (例 -1) で無効化。
    #[wasm_bindgen(js_name = setWarimePlayer)]
    pub fn set_warime_player(&mut self, idx: i32) {
        self.game.warime_player = if idx >= 0 && (idx as usize) < self.game.players.len() {
            Some(idx as usize)
        } else {
            None
        };
    }

    /// #118: 割れ目プレイヤー (無効なら -1) を返す。
    #[wasm_bindgen(js_name = getWarimePlayer)]
    pub fn get_warime_player(&self) -> i32 {
        match self.game.warime_player {
            Some(i) => i as i32,
            None => -1,
        }
    }

    /// #55: 特殊（途中）流局を有効にするかを設定する。デフォルト true。
    #[wasm_bindgen(js_name = setAllowAbortiveDraws)]
    pub fn set_allow_abortive_draws(&mut self, allowed: bool) {
        self.game.allow_abortive_draws = allowed;
    }

    /// #117: 差し馬の賭けを追加する。対局終了時に最終点数が高い方が低い方から
    /// `amount` を受け取る。
    #[wasm_bindgen(js_name = addSashimaBet)]
    pub fn add_sashima_bet(&mut self, player_a: usize, player_b: usize, amount: i32) {
        use crate::game::SashimaBet;
        if player_a < self.game.players.len()
            && player_b < self.game.players.len()
            && player_a != player_b
        {
            self.game.sashima_bets.push(SashimaBet {
                player_a,
                player_b,
                amount,
            });
        }
    }

    /// #117: 登録済みの差し馬の賭け数。
    #[wasm_bindgen(js_name = sashimaBetCount)]
    pub fn sashima_bet_count(&self) -> usize {
        self.game.sashima_bets.len()
    }

    /// #55: 四風連打が成立しているか。
    #[wasm_bindgen(js_name = checkSuufonRenda)]
    pub fn check_suufon_renda(&self) -> bool {
        self.game.check_suufon_renda()
    }

    /// #55: 四家立直が成立しているか。
    #[wasm_bindgen(js_name = checkSuuchaRiichi)]
    pub fn check_suucha_riichi(&self) -> bool {
        self.game.check_suucha_riichi()
    }

    /// #55: 四槓散了が成立しているか。
    #[wasm_bindgen(js_name = checkSuukanSanra)]
    pub fn check_suukan_sanra(&self) -> bool {
        self.game.check_suukan_sanra()
    }

    /// #55: 現在の手番プレイヤーが九種九牌を宣言できるか。
    #[wasm_bindgen(js_name = canDeclareKyuushu)]
    pub fn can_declare_kyuushu(&self) -> bool {
        self.game.can_declare_kyuushu(self.game.current_player)
    }

    /// #55: 九種九牌を宣言して途中流局にする。宣言不可なら false。
    #[wasm_bindgen(js_name = declareKyuushu)]
    pub fn declare_kyuushu(&mut self) -> bool {
        use crate::game::AbortiveDrawKind;
        if !self.game.can_declare_kyuushu(self.game.current_player) {
            return false;
        }
        self.game.apply_abortive_draw(AbortiveDrawKind::KyuushuKyuuhai);
        true
    }

    /// #55: 自動検出した途中流局 (四風連打/四家立直/四槓散了) を確定させる。
    /// 0=四風連打 / 1=四家立直 / 2=四槓散了。条件未成立なら false。
    #[wasm_bindgen(js_name = applyAbortiveDraw)]
    pub fn apply_abortive_draw_kind(&mut self, kind: u8) -> bool {
        use crate::game::AbortiveDrawKind;
        let (ok, k) = match kind {
            0 => (self.game.check_suufon_renda(), AbortiveDrawKind::SuufonRenda),
            1 => (self.game.check_suucha_riichi(), AbortiveDrawKind::SuuchaRiichi),
            2 => (self.game.check_suukan_sanra(), AbortiveDrawKind::SuukanSanra),
            _ => return false,
        };
        if !ok {
            return false;
        }
        self.game.apply_abortive_draw(k);
        true
    }

    /// #79: 指定プレイヤーの手牌を文字列（CUI コード）で返す。
    /// デバッグモードで CPU 手牌を表向き表示するために使用する。
    #[wasm_bindgen(js_name = getPlayerHandString)]
    pub fn get_player_hand_string(&self, player_idx: usize) -> String {
        if player_idx >= self.game.players.len() {
            return String::new();
        }
        self.game.players[player_idx].hand.to_string()
    }

    #[wasm_bindgen(js_name = getRound)]
    pub fn get_round(&self) -> u32 {
        self.game.round
    }

    #[wasm_bindgen(js_name = getHonba)]
    pub fn get_honba(&self) -> u32 {
        self.game.honba
    }

    #[wasm_bindgen(js_name = getDealer)]
    pub fn get_dealer(&self) -> usize {
        self.game.dealer
    }

    #[wasm_bindgen(js_name = getRiichiSticks)]
    pub fn get_riichi_sticks(&self) -> u32 {
        self.game.riichi_sticks
    }

    /// 直前局の結果。
    /// - 和了: `{"kind":"win","winner":idx,"winType":"tsumo"|"ron","from":idx?,"han":n,"fu":n,"totalPoints":n,"yaku":[...]}`
    /// - 流局: `{"kind":"draw","tenpaiPlayers":[...]}`
    /// - 未確定 (`last_outcome.is_none()`): "" を返す。
    #[wasm_bindgen(js_name = getLastOutcomeJson)]
    pub fn get_last_outcome_json(&self) -> String {
        match &self.game.last_outcome {
            None => String::new(),
            Some(RoundOutcome::Win { winner, kind, result }) => {
                let (win_type, from) = match kind {
                    WinKind::Tsumo => ("tsumo", None),
                    WinKind::Ron { from } => ("ron", Some(*from)),
                };
                let mut obj = serde_json::Map::new();
                obj.insert("kind".into(), serde_json::Value::String("win".into()));
                obj.insert("winner".into(), serde_json::Value::Number((*winner).into()));
                obj.insert("winType".into(), serde_json::Value::String(win_type.into()));
                if let Some(f) = from {
                    obj.insert("from".into(), serde_json::Value::Number(f.into()));
                }
                obj.insert("han".into(), serde_json::Value::Number(result.han.into()));
                obj.insert("fu".into(), serde_json::Value::Number(result.fu.into()));
                obj.insert(
                    "totalPoints".into(),
                    serde_json::Value::Number(result.total_points.into()),
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
            Some(RoundOutcome::Draw { tenpai_players }) => {
                let mut obj = serde_json::Map::new();
                obj.insert("kind".into(), serde_json::Value::String("draw".into()));
                obj.insert(
                    "tenpaiPlayers".into(),
                    serde_json::Value::Array(
                        tenpai_players
                            .iter()
                            .map(|i| serde_json::Value::Number((*i).into()))
                            .collect(),
                    ),
                );
                serde_json::Value::Object(obj).to_string()
            }
            Some(RoundOutcome::AbortiveDraw { kind }) => {
                let mut obj = serde_json::Map::new();
                obj.insert("kind".into(), serde_json::Value::String("abortive".into()));
                obj.insert(
                    "abortiveKind".into(),
                    serde_json::Value::String(format!("{:?}", kind)),
                );
                serde_json::Value::Object(obj).to_string()
            }
        }
    }
}

// scoring_summary_json は agari_extract に切り出し済み (Issue #66)。
// `crate::wasm::scoring_summary_json` 互換のため pub(crate) で再 export。
#[cfg(feature = "wasm")]
pub(crate) use crate::agari_extract::scoring_summary_json;

// extract_agari / extract_agari_with_context は `crate::agari_extract` に移動し
// wasm.rs 冒頭で pub(crate) use 経由で再 export している (Issue #66)。

// ==================== Nostr P2P機能 ====================

#[cfg(feature = "wasm")]
use crate::wasm_nostr::{WasmNostrKeys, WasmNostrClient, WasmMatchState, WasmGameEvent, WasmGameEventType};

/// Nostr鍵管理
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmNostrKeyManager {}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmNostrKeyManager {
    /// 新しい鍵を生成して保存
    #[wasm_bindgen(js_name = generateAndSave)]
    pub fn generate_and_save() -> Result<String, String> {
        let keys = WasmNostrKeys::generate();
        let pubkey = keys.public_key.clone();
        keys.save()?;
        Ok(pubkey)
    }

    /// 保存された鍵を読み込み
    #[wasm_bindgen(js_name = loadKeys)]
    pub fn load_keys() -> Result<String, String> {
        let keys = WasmNostrKeys::load()?;
        Ok(keys.public_key)
    }

    /// 鍵を削除
    #[wasm_bindgen(js_name = deleteKeys)]
    pub fn delete_keys() -> Result<(), String> {
        WasmNostrKeys::delete()
    }

    /// 鍵が保存されているかチェック
    #[wasm_bindgen(js_name = hasKeys)]
    pub fn has_keys() -> bool {
        WasmNostrKeys::load().is_ok()
    }
}

/// Nostrクライアント
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmNostrP2PClient {
    client: WasmNostrClient,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmNostrP2PClient {
    /// 新しいクライアントを作成
    #[wasm_bindgen(constructor)]
    pub fn new(relay_url: String) -> Result<WasmNostrP2PClient, String> {
        let client = WasmNostrClient::new(relay_url)?;
        Ok(Self { client })
    }

    /// 公開鍵を取得
    #[wasm_bindgen(js_name = getPublicKey)]
    pub fn get_public_key(&self) -> String {
        self.client.get_public_key()
    }

    /// マッチング募集イベントを作成
    #[wasm_bindgen(js_name = createMatchSeekEvent)]
    pub fn create_match_seek_event(&self, max_players: usize) -> String {
        let event = self.client.create_match_seek_event(max_players);
        event.to_json()
    }

    /// マッチング参加イベントを作成
    #[wasm_bindgen(js_name = createMatchJoinEvent)]
    pub fn create_match_join_event(&self, match_id: String) -> String {
        let event = self.client.create_match_join_event(match_id);
        event.to_json()
    }

    /// 打牌イベントを作成
    #[wasm_bindgen(js_name = createDiscardEvent)]
    pub fn create_discard_event(&self, match_id: String, tile: String) -> String {
        let event = self.client.create_game_event(
            WasmGameEventType::DiscardTile,
            match_id,
            tile,
        );
        event.to_json()
    }

    /// リーチイベントを作成
    #[wasm_bindgen(js_name = createRiichiEvent)]
    pub fn create_riichi_event(&self, match_id: String) -> String {
        let event = self.client.create_game_event(
            WasmGameEventType::Riichi,
            match_id,
            String::new(),
        );
        event.to_json()
    }

    /// ロンイベントを作成
    #[wasm_bindgen(js_name = createRonEvent)]
    pub fn create_ron_event(&self, match_id: String) -> String {
        let event = self.client.create_game_event(
            WasmGameEventType::Ron,
            match_id,
            String::new(),
        );
        event.to_json()
    }

    /// ツモイベントを作成
    #[wasm_bindgen(js_name = createTsumoEvent)]
    pub fn create_tsumo_event(&self, match_id: String) -> String {
        let event = self.client.create_game_event(
            WasmGameEventType::Tsumo,
            match_id,
            String::new(),
        );
        event.to_json()
    }
}

// ==================== WebRTC P2P通信 ====================

#[cfg(feature = "wasm")]
use crate::wasm_webrtc::{WasmWebRtcManager, SignalingData};
#[cfg(feature = "wasm")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "wasm")]
use std::collections::HashMap;

/// WebRTCマネージャーのラッパー
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmWebRtcP2PManager {
    manager: Arc<Mutex<WasmWebRtcManager>>,
}

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmWebRtcP2PManager {
    /// 新しいマネージャーを作成
    #[wasm_bindgen(constructor)]
    pub fn new(local_id: String) -> Self {
        let manager = WasmWebRtcManager::new(local_id);
        Self {
            manager: Arc::new(Mutex::new(manager)),
        }
    }

    /// ピア接続を作成
    #[wasm_bindgen(js_name = createPeerConnection)]
    pub fn create_peer_connection(&self, peer_id: String) -> Result<(), String> {
        let mut manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.create_peer_connection(peer_id)
    }

    /// データチャネルを作成
    #[wasm_bindgen(js_name = createDataChannel)]
    pub fn create_data_channel(&self, peer_id: String, label: String) -> Result<(), String> {
        let mut manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.create_data_channel(peer_id, &label)
    }

    /// オファーを作成（非同期）
    /// JavaScript側でawaitして使用する
    #[wasm_bindgen(js_name = createOffer)]
    pub async fn create_offer(&self, peer_id: String) -> Result<String, String> {
        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        let sdp = manager.create_offer(&peer_id).await?;
        let offer_data = SignalingData::Offer { sdp };
        Ok(offer_data.to_json())
    }

    /// アンサーを作成（非同期）
    #[wasm_bindgen(js_name = createAnswer)]
    pub async fn create_answer(&self, peer_id: String, offer_json: String) -> Result<String, String> {
        let offer_data: SignalingData = SignalingData::from_json(&offer_json)?;

        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;

        if let SignalingData::Offer { sdp } = offer_data {
            let answer_sdp = manager.create_answer(&peer_id, &sdp).await?;
            let answer_data = SignalingData::Answer { sdp: answer_sdp };
            Ok(answer_data.to_json())
        } else {
            Err("Invalid offer data".to_string())
        }
    }

    /// アンサーを設定（非同期）
    #[wasm_bindgen(js_name = setAnswer)]
    pub async fn set_answer(&self, peer_id: String, answer_json: String) -> Result<(), String> {
        let answer_data: SignalingData = SignalingData::from_json(&answer_json)?;

        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;

        if let SignalingData::Answer { sdp } = answer_data {
            manager.set_answer(&peer_id, &sdp).await
        } else {
            Err("Invalid answer data".to_string())
        }
    }

    /// ICE候補を追加（非同期）
    #[wasm_bindgen(js_name = addIceCandidate)]
    pub async fn add_ice_candidate(&self, peer_id: String, candidate_json: String) -> Result<(), String> {
        let candidate_data: SignalingData = SignalingData::from_json(&candidate_json)?;

        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;

        if let SignalingData::IceCandidate { candidate, sdp_mid, sdp_m_line_index } = candidate_data {
            manager.add_ice_candidate(&peer_id, &candidate, &sdp_mid, sdp_m_line_index).await
        } else {
            Err("Invalid ICE candidate data".to_string())
        }
    }

    /// データを送信
    #[wasm_bindgen(js_name = sendData)]
    pub fn send_data(&self, peer_id: String, data: String) -> Result<(), String> {
        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.send_data(&peer_id, &data)
    }

    /// 全ピアに送信
    #[wasm_bindgen(js_name = broadcast)]
    pub fn broadcast(&self, data: String) -> Result<(), String> {
        let manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.broadcast(&data)
    }

    /// 接続を閉じる
    #[wasm_bindgen(js_name = closeConnection)]
    pub fn close_connection(&self, peer_id: String) -> Result<(), String> {
        let mut manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.close_connection(&peer_id)
    }

    /// 全接続を閉じる
    #[wasm_bindgen(js_name = closeAll)]
    pub fn close_all(&self) -> Result<(), String> {
        let mut manager = self.manager.lock().map_err(|_| "Lock failed".to_string())?;
        manager.close_all();
        Ok(())
    }
}

/// バージョン情報を返す
#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// ゲーム名を返す
#[cfg(feature = "wasm")]
#[wasm_bindgen(js_name = gameName)]
pub fn game_name() -> String {
    "邪雀 Xtreme Mahjong".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "wasm")]
    fn test_wasm_game() {
        let player_names = vec![
            "Player1".to_string(),
            "Player2".to_string(),
            "Player3".to_string(),
            "Player4".to_string(),
        ];

        let mut wasm_game = WasmGame::new(player_names);

        assert!(!wasm_game.is_game_over());
        assert_eq!(wasm_game.get_current_player_id(), 0);

        let state = wasm_game.get_game_state();
        assert!(!state.is_empty());
    }

    // ==================== Round loop bridge tests (Issue #27) ====================

    fn make_game() -> WasmGame {
        WasmGame::new(vec![
            "P1".into(),
            "P2".into(),
            "P3".into(),
            "P4".into(),
        ])
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn round_getters_initial_values() {
        let g = make_game();
        assert_eq!(g.get_round(), 1);
        assert_eq!(g.get_honba(), 0);
        assert_eq!(g.get_dealer(), 0);
        assert_eq!(g.get_riichi_sticks(), 0);
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn last_outcome_empty_initially() {
        let g = make_game();
        assert_eq!(g.get_last_outcome_json(), "");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_draw_writes_outcome_json() {
        let mut g = make_game();
        g.resolve_draw(vec![0, 2]);
        let json = g.get_last_outcome_json();
        assert!(json.contains("\"kind\":\"draw\""), "got {}", json);
        assert!(json.contains("\"tenpaiPlayers\":[0,2]"), "got {}", json);
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_tsumo_non_winning_hand_returns_empty() {
        // 配牌直後の親 (13 枚) は 14 枚ではないので extract_agari が失敗する
        let mut g = make_game();
        let s = g.resolve_win_tsumo(0);
        assert_eq!(s, "");
        // last_outcome も書かれていない
        assert_eq!(g.get_last_outcome_json(), "");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn compute_tenpai_players_classifies_hands() {
        // 配牌直後は player.hand が 13 枚ランダムに埋まる。
        // テンパイ判定の本物の挙動はゲーム進行で発生する。ここでは
        // 「player 0 だけ強制的にテンパイ手 (1-9m + 1-3p + 1z 雀頭候補) を持たせる」
        // 形で挙動を直接確認する。
        use crate::tile::{Tile, Suit, Honor};
        let mut g = make_game();
        // player 0 を七対子テンパイ (6 対子 + 1 単騎) にする
        let tenpai_tiles = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_honor(Honor::Ton),
        ];
        g.game.players[0].hand = crate::Hand::new();
        for t in tenpai_tiles {
            g.game.players[0].hand.add_tile(t);
        }
        // player 1 を完全ノーテン (バラバラ)
        let noten_tiles = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Sou, 5, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_honor(Honor::Ton),
        ];
        g.game.players[1].hand = crate::Hand::new();
        for t in noten_tiles {
            g.game.players[1].hand.add_tile(t);
        }
        let tenpai = g.compute_tenpai_players();
        assert!(tenpai.contains(&0), "player0 should be tenpai: got {:?}", tenpai);
        assert!(!tenpai.contains(&1), "player1 should be noten: got {:?}", tenpai);
    }

    // ==================== Issue #33: extract_agari 副露込みテスト ====================

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_returns_none_for_incomplete_hand() {
        // 13 枚（聴牌前）では和了確定できない
        use crate::tile::{Tile, Suit};
        let mut hand = crate::Hand::new();
        for _ in 0..13 {
            hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        }
        assert!(extract_agari(&hand).is_none());
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_succeeds_with_pon_meld() {
        // ポン 1m1m1m + 残り手牌 11 枚 (= 14 枚相当) で和了形
        use crate::tile::{Tile, Suit};
        use crate::hand::{Meld, MeldType};
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Pin, 8, false),
            Tile::new_number(Suit::Pin, 9, false),
            Tile::new_number(Suit::Sou, 6, false),
            Tile::new_number(Suit::Sou, 6, false),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Pon,
            tiles: vec![
                Tile::new_number(Suit::Man, 1, false),
                Tile::new_number(Suit::Man, 1, false),
                Tile::new_number(Suit::Man, 1, false),
            ],
            is_open: true,
            ..Default::default()
        });
        let result = extract_agari(&hand);
        assert!(result.is_some(), "ポン込み和了形は extract_agari が成立");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_succeeds_with_chi_meld() {
        // チー 4m5m6m + 残り手牌 11 枚
        use crate::tile::{Tile, Suit};
        use crate::hand::{Meld, MeldType};
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 5, false),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Chi,
            tiles: vec![
                Tile::new_number(Suit::Man, 4, false),
                Tile::new_number(Suit::Man, 5, false),
                Tile::new_number(Suit::Man, 6, false),
            ],
            is_open: true,
            ..Default::default()
        });
        let result = extract_agari(&hand);
        assert!(result.is_some(), "チー込み和了形は extract_agari が成立");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_succeeds_with_kan_meld() {
        // 明槓 9m + 残り手牌 11 枚 (Hand::tile_count では槓も +3 扱い)
        use crate::tile::{Tile, Suit, Honor};
        use crate::hand::{Meld, MeldType};
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Pin, 1, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Sou, 4, false),
            Tile::new_number(Suit::Sou, 5, false),
            Tile::new_number(Suit::Sou, 6, false),
            Tile::new_honor(Honor::Haku),
            Tile::new_honor(Honor::Haku),
            Tile::new_honor(Honor::Haku),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Ton),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Kan,
            tiles: vec![
                Tile::new_number(Suit::Man, 9, false),
                Tile::new_number(Suit::Man, 9, false),
                Tile::new_number(Suit::Man, 9, false),
                Tile::new_number(Suit::Man, 9, false),
            ],
            is_open: true,
            ..Default::default()
        });
        let result = extract_agari(&hand);
        assert!(result.is_some(), "明槓込み和了形は extract_agari が成立");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_fails_when_remaining_not_winning() {
        // 副露あり、残り手牌バラバラ → 和了不成立
        use crate::tile::{Tile, Suit, Honor};
        use crate::hand::{Meld, MeldType};
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 6, false),
            Tile::new_number(Suit::Man, 8, false),
            Tile::new_number(Suit::Pin, 1, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Man, 5, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Nan),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Pon,
            tiles: vec![
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
            ],
            is_open: true,
            ..Default::default()
        });
        assert!(extract_agari(&hand).is_none(), "副露あり・バラバラ手は不成立");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_tsumo_succeeds_with_meld() {
        // 副露あり手で resolve_win_tsumo が成功する (空文字でない結果) ことの確認。
        // 役牌白ポン + 1m1m1m暗刻 + 2p3p4p順子 + 7s8s9s順子 + 雀頭東 を作る。
        use crate::tile::{Tile, Suit, Honor};
        use crate::hand::{Meld, MeldType};
        let mut g = make_game();
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Ton),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Pon,
            tiles: vec![
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
            ],
            is_open: true,
            ..Default::default()
        });
        g.game.players[0].hand = hand;
        let s = g.resolve_win_tsumo(0);
        assert!(!s.is_empty(), "副露あり和了で resolve_win_tsumo が成功するはず: got {:?}", s);
        // 役牌 (白) が含まれる
        assert!(s.contains("Yakuhai") || s.contains("\"han\""), "yaku/han 情報が含まれる: {}", s);
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_tsumo_shibari_blocked_returns_sentinel() {
        // #143: 本場縛りで弾かれた合法和了は空文字ではなく
        // SHIBARI_BLOCKED_JSON センチネルを返し、役無し (空文字) と区別する。
        // 役牌白ポン (1飜) の手を 5 本場 + 2飜縛りに掛けるとブロックされる。
        use crate::game::ShibariRule;
        let mut g = setup_shibari_blocked_tsumo_hand();
        g.game.shibari_rule = ShibariRule::TwoHanFromFiveHonba;
        // 天和 (役満) が乗ると 2飜縛りを満たしてしまうので進行済みにして無効化する
        g.game.draws_this_round = 1;
        let s = g.resolve_win_tsumo(0);
        assert_eq!(s, SHIBARI_BLOCKED_JSON, "本場縛りブロックはセンチネルを返す: got {:?}", s);
        // resolve_win は呼ばれていないので局結果は書かれない
        assert_eq!(g.get_last_outcome_json(), "");
    }

    // ==================== #143 本場縛りブロックのセンチネル化 ====================

    /// 1飜の合法和了 (役牌白ポン) を完成形 (手牌 11 枚 + 白ポン) として player[0] に持たせ、
    /// 5本場をセットした WasmGame を返すツモ用共通ヘルパ。
    /// 雀頭は東対子。手牌は和了済み形 (打牌不要) なので resolve_win_tsumo(0) を直接呼べる。
    /// shibari_rule と draws_this_round (天和の有効/無効) は検証ごとに違うため、
    /// 呼び出し側で設定する。
    #[cfg(feature = "wasm")]
    fn setup_shibari_blocked_tsumo_hand() -> WasmGame {
        use crate::tile::{Tile, Suit, Honor};
        use crate::hand::{Meld, MeldType};
        let mut g = make_game();
        let mut hand = crate::Hand::new();
        for tile in [
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Ton),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Pon,
            tiles: vec![
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
            ],
            is_open: true,
            ..Default::default()
        });
        g.game.players[0].hand = hand;
        g.game.honba = 5;
        g
    }

    /// 1飜の合法和了 (役牌白ポン) を 13 枚 + 当たり牌 (Ton) に分割して player[winner] に持たせ、
    /// 2飜縛り / 5本場 (天和無効化のため進行済み) をセットする共通ヘルパ。
    /// Ron / Chankan のセンチネル検証で使う。winning_tile は東 (Ton)。
    #[cfg(feature = "wasm")]
    fn setup_shibari_blocked_meld_hand(winner: usize) -> WasmGame {
        use crate::tile::{Tile, Suit, Honor};
        use crate::hand::{Meld, MeldType};
        use crate::game::ShibariRule;
        let mut g = make_game();
        let mut hand = crate::Hand::new();
        // 13 枚相当 (手牌 10 + ポン 3)。東は 1 枚だけ持ち、もう 1 枚 (Ton) が当たり牌。
        for tile in [
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
        ] {
            hand.add_tile(tile);
        }
        hand.add_meld(Meld {
            meld_type: MeldType::Pon,
            tiles: vec![
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
                Tile::new_honor(Honor::Haku),
            ],
            is_open: true,
            ..Default::default()
        });
        g.game.players[winner].hand = hand;
        g.game.shibari_rule = ShibariRule::TwoHanFromFiveHonba;
        g.game.honba = 5;
        // 天和 (役満) が乗ると 2飜縛りを満たしてしまうので進行済みにして無効化する
        g.game.draws_this_round = 1;
        g
    }

    /// #143/観点2: 本場縛りで弾かれた合法ロン和了は空文字ではなく
    /// SHIBARI_BLOCKED_JSON センチネルを返す (Tsumo 同様 Ron でも区別する)。
    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_ron_shibari_blocked_returns_sentinel() {
        use crate::tile::{Tile, Honor};
        let mut g = setup_shibari_blocked_meld_hand(1);
        // 当たり牌 (東) を放銃牌としてセット
        g.game.last_discard = Some(Tile::new_honor(Honor::Ton));
        g.game.last_discard_hidden = false;
        let s = g.resolve_win_ron(1, 0);
        assert_eq!(s, SHIBARI_BLOCKED_JSON, "ロンの本場縛りブロックもセンチネルを返す: got {:?}", s);
        // resolve_win は呼ばれていないので局結果は書かれない (点数移動なし)
        assert_eq!(g.get_last_outcome_json(), "");
    }

    /// #143/観点3: 槍槓ロンが本場縛りで弾かれたときセンチネルを返し、
    /// かつ pending_chankan がクリアされない (= 加槓が続行できる) こと。
    ///
    /// 槍槓は Chankan 役 (1飜) が自動で乗るため、役牌白ポン (1飜) と合わせて
    /// 2飜となり 2飜縛り (TwoHanFromFiveHonba) は満たしてしまう。そこで満貫縛り
    /// (ManganFromFiveHonba) に差し替え、2飜 (満貫未満) を確実にブロックさせる。
    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_chankan_shibari_blocked_keeps_pending() {
        use crate::tile::{Tile, Honor};
        use crate::game::ShibariRule;
        let mut g = setup_shibari_blocked_meld_hand(1);
        g.game.shibari_rule = ShibariRule::ManganFromFiveHonba;
        let win_tile = Tile::new_honor(Honor::Ton);
        g.game.pending_chankan = Some(win_tile);
        let s = g.resolve_win_chankan(1, 0);
        assert_eq!(s, SHIBARI_BLOCKED_JSON, "槍槓ロンの本場縛りブロックもセンチネルを返す: got {:?}", s);
        // ブロック時は resolve_win に進まず pending_chankan を None にしない → 加槓続行できる
        assert_eq!(
            g.game.pending_chankan,
            Some(win_tile),
            "本場縛りブロック時は pending_chankan を維持する (加槓続行)"
        );
        assert_eq!(g.get_last_outcome_json(), "", "点数移動なし");
    }

    /// #143/観点4: 二重実行不変条件。縛り未達の手で resolve_win_tsumo を 2 回呼んでも
    /// 両回ともセンチネルで、局結果は一度も書かれない (点数が一切動かない)。
    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_tsumo_shibari_blocked_is_idempotent() {
        use crate::game::ShibariRule;
        let mut g = setup_shibari_blocked_tsumo_hand();
        g.game.shibari_rule = ShibariRule::TwoHanFromFiveHonba;
        g.game.draws_this_round = 1;
        let first = g.resolve_win_tsumo(0);
        let second = g.resolve_win_tsumo(0);
        assert_eq!(first, SHIBARI_BLOCKED_JSON, "1 回目もセンチネル");
        assert_eq!(second, SHIBARI_BLOCKED_JSON, "2 回目もセンチネル (不変)");
        assert_eq!(g.get_last_outcome_json(), "", "再実行しても局結果は書かれない (点数移動ゼロ)");
    }

    /// #143/観点5: 和了形不成立 (役無し・ばらばら手) のときは従来通り空文字を返し、
    /// センチネルを返さない (センチネル導入で空文字パスを壊していないことの回帰)。
    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_tsumo_invalid_hand_returns_empty_not_sentinel() {
        use crate::tile::{Tile, Suit, Honor};
        use crate::game::ShibariRule;
        let mut g = make_game();
        let mut hand = crate::Hand::new();
        // ばらばらの 14 枚 (和了形でない)
        for tile in [
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Man, 7, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 8, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 6, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Nan),
            Tile::new_honor(Honor::Shaa),
            Tile::new_honor(Honor::Pei),
            Tile::new_honor(Honor::Haku),
        ] {
            hand.add_tile(tile);
        }
        g.game.players[0].hand = hand;
        // 縛りが効く状況でも、そもそも和了形でないので meets_shibari に到達しない
        g.game.shibari_rule = ShibariRule::TwoHanFromFiveHonba;
        g.game.honba = 5;
        let s = g.resolve_win_tsumo(0);
        assert_eq!(s, "", "和了形不成立は空文字 (センチネルではない): got {:?}", s);
        assert_ne!(s, SHIBARI_BLOCKED_JSON, "和了形不成立をセンチネル扱いしない");
    }

    /// #143/観点6: 縛りを満たす通常和了は正規サマリ JSON を返し、
    /// センチネル文字列が混入しないこと。
    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_tsumo_passing_shibari_returns_summary_not_sentinel() {
        use crate::game::ShibariRule;
        let mut g = setup_shibari_blocked_tsumo_hand();
        // 縛りなし (Standard) なら 1飜でも通る → 正規サマリ
        g.game.shibari_rule = ShibariRule::Standard;
        let s = g.resolve_win_tsumo(0);
        assert_ne!(s, SHIBARI_BLOCKED_JSON, "通常和了はセンチネルでない");
        assert!(!s.is_empty(), "通常和了は空文字でない");
        assert!(!s.contains("shibariBlocked"), "正規サマリにセンチネルキーが混入しない: {}", s);
        assert!(s.contains("\"han\""), "正規サマリは han を含む: {}", s);
    }

    /// #143/観点9 (過去事故): 縛り飜未満でも役満が乗っていれば縛りを満たし、
    /// センチネルを返さない (役満時に誤ブロックしない回帰)。天和を有効化して役満を作る。
    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_tsumo_yakuman_passes_shibari_no_sentinel() {
        use crate::game::ShibariRule;
        let mut g = setup_shibari_blocked_tsumo_hand();
        g.game.shibari_rule = ShibariRule::TwoHanFromFiveHonba;
        // draws_this_round = 0 のまま親 (idx 0) のツモ → 天和成立 (役満)
        g.game.draws_this_round = 0;
        let s = g.resolve_win_tsumo(0);
        assert_ne!(s, SHIBARI_BLOCKED_JSON, "役満は縛りを満たすのでセンチネルにならない: got {:?}", s);
        assert!(!s.is_empty(), "役満和了は空文字でない");
    }

    /// #143/観点10: ManganFromFiveHonba 縛りでも、満貫未満の合法和了は
    /// センチネルを返す (別 shibari_rule でも wasm 戻り値経由でセンチネル化される)。
    #[test]
    #[cfg(feature = "wasm")]
    fn resolve_win_tsumo_mangan_shibari_blocks_below_mangan() {
        use crate::game::ShibariRule;
        let mut g = setup_shibari_blocked_tsumo_hand();
        g.game.shibari_rule = ShibariRule::ManganFromFiveHonba;
        g.game.draws_this_round = 1; // 天和無効化
        let s = g.resolve_win_tsumo(0);
        assert_eq!(
            s, SHIBARI_BLOCKED_JSON,
            "満貫未満は ManganFromFiveHonba 縛りでブロック → センチネル: got {:?}", s
        );
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn can_tsumo_false_for_13_tile_hand() {
        // 配牌直後の player は手牌 13 枚 (親は 14 枚だが extract_agari がランダム手では基本失敗)
        let g = make_game();
        // 子 (idx 1) は 13 枚なので tile_count != 14 → 必ず false
        assert!(!g.can_tsumo(1));
        // 範囲外 idx も false
        assert!(!g.can_tsumo(99));
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn can_tsumo_true_for_completed_hand() {
        // 七対子完成 14 枚を強制的に持たせる
        use crate::tile::{Tile, Suit, Honor};
        let mut g = make_game();
        let tiles = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_number(Suit::Sou, 8, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Ton),
        ];
        g.game.players[0].hand = crate::Hand::new();
        for t in tiles {
            g.game.players[0].hand.add_tile(t);
        }
        assert!(g.can_tsumo(0), "完成手なので can_tsumo は true");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn can_ron_returns_false_when_no_last_discard() {
        let g = make_game();
        // last_discard が None なので全員 false
        assert!(!g.can_ron(0));
        assert!(!g.can_ron(1));
        assert!(!g.can_ron(2));
        assert!(!g.can_ron(3));
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn can_ron_self_discarder_returns_false() {
        // current_player には絶対ロンさせない (自摸ロン防止)
        use crate::tile::{Tile, Suit};
        let mut g = make_game();
        g.game.last_discard = Some(Tile::new_number(Suit::Man, 1, false));
        g.game.last_discard_hidden = false;
        // current_player は 0 → can_ron(0) は false
        assert!(!g.can_ron(g.game.current_player));
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn get_last_discarder_none_initially() {
        let g = make_game();
        assert!(g.get_last_discarder().is_none());
    }

    /// S1: 親 14 枚 (配牌直後 + ツモ無し相当で 14 枚を持たせた状態) でも
    /// 和了形が組めなければ can_tsumo は false。手作り 14 枚で extract_agari 不成立。
    #[test]
    #[cfg(feature = "wasm")]
    fn can_tsumo_false_for_14_tile_non_winning_hand() {
        use crate::tile::{Tile, Suit, Honor};
        let mut g = make_game();
        // 14 枚で和了形が組めないバラバラの手牌を作る
        let tiles = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 3, false),
            Tile::new_number(Suit::Man, 5, false),
            Tile::new_number(Suit::Man, 7, false),
            Tile::new_number(Suit::Man, 9, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Pin, 6, false),
            Tile::new_number(Suit::Pin, 8, false),
            Tile::new_number(Suit::Sou, 1, false),
            Tile::new_number(Suit::Sou, 5, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Nan),
        ];
        g.game.players[0].hand = crate::Hand::new();
        for t in tiles {
            g.game.players[0].hand.add_tile(t);
        }
        assert!(!g.can_tsumo(0), "14 枚でも和了形不成立なら false");
    }

    /// S1 (補): 親 14 枚 (配牌直後) で完成形なら can_tsumo は true。
    /// `can_tsumo_true_for_completed_hand` で実質カバー済みだが「14 枚親直後」を明示する。
    #[test]
    #[cfg(feature = "wasm")]
    fn can_tsumo_true_for_14_tile_dealer_initial_winning() {
        use crate::tile::{Tile, Suit, Honor};
        let mut g = make_game();
        // 七対子 14 枚を親 (idx 0) に持たせる
        let tiles = vec![
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Man, 5, false),
            Tile::new_number(Suit::Man, 5, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 8, false),
            Tile::new_number(Suit::Pin, 8, false),
            Tile::new_number(Suit::Sou, 4, false),
            Tile::new_number(Suit::Sou, 4, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_honor(Honor::Nan),
            Tile::new_honor(Honor::Nan),
        ];
        g.game.players[0].hand = crate::Hand::new();
        for t in tiles {
            g.game.players[0].hand.add_tile(t);
        }
        assert!(g.can_tsumo(0), "親配牌相当の 14 枚完成形は can_tsumo=true");
    }

    /// S2: last_discard が存在し completed な手牌に対しても、`last_discard_hidden=true`
    /// (闇牌・暗槓中など) なら can_ron は false を返す。
    #[test]
    #[cfg(feature = "wasm")]
    fn can_ron_returns_false_when_last_discard_hidden() {
        use crate::tile::{Tile, Suit};
        let mut g = make_game();
        // 単騎待ち 13 枚を player 1 に持たせる (1m * 13 → 1m が出れば四暗刻相当だが
        // ここでは can_win の判定のみが目的)
        let win_tile = Tile::new_number(Suit::Man, 1, false);
        let mut tiles = vec![];
        // 簡易: 二二三三四四 五五六六 七七 + 単騎 → 七対子テンパイ (待ち 1m)
        for v in [2u8, 3, 4, 5, 6, 7] {
            tiles.push(Tile::new_number(Suit::Man, v, false));
            tiles.push(Tile::new_number(Suit::Man, v, false));
        }
        tiles.push(Tile::new_number(Suit::Man, 1, false)); // 単騎の片割れ
        g.game.players[1].hand = crate::Hand::new();
        for t in tiles {
            g.game.players[1].hand.add_tile(t);
        }
        g.game.last_discard = Some(win_tile);

        // まず可視で can_ron=true (前提条件確認)
        g.game.last_discard_hidden = false;
        let visible = g.can_ron(1);

        // 闇牌で can_ron=false (本テストの主目的)
        g.game.last_discard_hidden = true;
        assert!(!g.can_ron(1), "last_discard_hidden=true なら can_ron は必ず false");

        // 前提条件 (可視時) が崩れていないことも確認
        // 注: 七対子テンパイなので can_win は true のはず
        assert!(visible, "可視時は和了形なので can_ron=true (前提条件)");
    }

    /// S3: 副露ありで手牌 11 枚 + ポン 1 = tile_count() 14 のとき、can_tsumo は
    /// 「tile_count == 14」の関門を通過し extract_agari にかかる。現在の
    /// `extract_agari` は副露込み 14 枚相当を許容するため、和了形なら true を返す。
    ///
    /// TODO(#33 副露): 副露ありの和了形を確定させるためのスコアリング統合は未完。
    /// `extract_agari` を呼ぶこと自体は通るが、`resolveWinTsumo` 側で `ScoringEngine` が
    /// 副露込みの手牌を正しく評価できる保証はない。本テストは現状の挙動を固定するだけ。
    #[test]
    #[cfg(feature = "wasm")]
    fn can_tsumo_with_open_meld_reflects_extract_agari() {
        use crate::tile::{Tile, Suit, Honor};
        use crate::hand::{Meld, MeldType};
        let mut g = make_game();

        // 手牌 11 枚 + ポン 1 = 14 枚相当。
        // 和了形例: 234m / 567p / 11s / 333s + ポン(発発発)
        let tiles = vec![
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Man, 3, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 6, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Sou, 1, false),
            Tile::new_number(Suit::Sou, 1, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 3, false),
            Tile::new_number(Suit::Sou, 3, false),
        ];
        let mut hand = crate::Hand::new();
        for t in tiles {
            hand.add_tile(t);
        }
        // 発のポンを追加 (add_meld は対応する tiles を tiles 配列から remove するが、
        // ここでは元から発を持っていないので tiles からは何も消えない → tile_count は
        // 11 + 3 = 14 になる)
        let hatsu = Tile::new_honor(Honor::Hatsu);
        hand.add_meld(Meld {
            meld_type: MeldType::Pon,
            tiles: vec![hatsu, hatsu, hatsu],
            is_open: true,
            ..Default::default()
        });
        assert_eq!(hand.tile_count(), 14, "副露 1 + 手牌 11 = tile_count 14");
        g.game.players[0].hand = hand;

        // 現状の挙動: tile_count == 14 関門は通過する。
        // extract_agari の結果が true / false どちらかは hand.can_win の副露対応に依存。
        // ここでは「副露ありで一律 false になっていないこと」を確認する。
        // もし将来「副露ありで一生 can_tsumo=false」になるリグレッションが入った場合、
        // このアサーションが落ちて気づける。
        let result = g.can_tsumo(0);
        // 副露込みでも和了形なら true (現状の extract_agari 実装が許容するなら)。
        // 副露未対応で false になるなら本テストが赤くなり Issue #33 のシグナルになる。
        assert!(
            result,
            "副露 1 + 手牌 11 の和了形は can_tsumo=true (もし false なら Issue #33 副露対応のリグレッション)"
        );
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn extract_agari_prefers_higher_score() {
        // 多面待ち手で「高得点解釈が選ばれる」ことを検証する。
        // 構成: 2m3m4m / 2p3p4p / 5p6p7p / 2s3s4s / 5s6s7s (14 枚、全順子・断么)。
        // 14 枚の状態から各 unique tile を winning 候補として試した時、
        // 全候補で「順子完成 + 平和形 + 断么九」が成立し、最高得点解釈が選ばれる。
        // 重要なのは extract_agari_with_context が「和了形 + 最高得点」候補を実際に返すこと。
        use crate::tile::Suit;
        let mut hand = crate::Hand::new();
        for t in [
            Tile::new_number(Suit::Man, 2, false), Tile::new_number(Suit::Man, 3, false), Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 2, false), Tile::new_number(Suit::Pin, 3, false), Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Pin, 5, false), Tile::new_number(Suit::Pin, 6, false), Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Sou, 2, false), Tile::new_number(Suit::Sou, 3, false), Tile::new_number(Suit::Sou, 4, false),
            Tile::new_number(Suit::Sou, 5, false), Tile::new_number(Suit::Sou, 5, false),
        ] {
            hand.add_tile(t);
        }
        // 雀頭 5s5s, 234m / 234p / 567p / 234s で 4 面子 1 雀頭 → 和了形
        // winning=5s ならシャンポンや単騎は出ない (5s5s 雀頭固定)、suit 内に 567s が無いので両面/嵌張なし。
        // → 実際は 5s5s 雀頭での単一解釈で和了する
        let result = extract_agari_with_context(&hand, true, false);
        assert!(result.is_some(), "和了形が見つかる: {:?}", result);
        let (sub_hand, winning_tile) = result.unwrap();
        // 返り値が実際に和了形であることを確認
        assert!(sub_hand.can_win(&winning_tile), "返された (sub_hand, winning_tile) は和了形");
        // 点数計算が成功し、役ありで点数が付くことを確認 (高得点選択ロジックが走った証拠)
        let score = crate::scoring::ScoringEngine::calculate_score(&sub_hand, &winning_tile, true, false);
        assert!(score.is_some(), "スコア計算成功");
        let s = score.unwrap();
        assert!(s.total_points > 0, "最高得点候補は役あり (total_points > 0): got {}", s.total_points);
        assert!(s.han >= 1, "最低 1 翻 (断么 or 平和): han={}", s.han);
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn next_round_progresses_or_ends() {
        let mut g = make_game();
        // 子和了相当: dealer_won_last=false にして親流れさせる
        g.resolve_draw(vec![]); // 全員ノーテン → dealer_won_last=false
        let cont = g.next_round();
        // 東風戦の途中なので continue するはず
        assert!(cont);
        assert_eq!(g.get_round(), 2);
        assert_eq!(g.get_dealer(), 1);
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn get_last_discarder_uses_last_discarder_field() {
        // #77 regression: last_discarder フィールドを使うこと（計算式ではなく）
        let mut g = make_game();
        use crate::tile::{Tile, Suit};
        let tiles = vec![
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Man, 3, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Man, 5, false),
            Tile::new_number(Suit::Man, 6, false),
            Tile::new_number(Suit::Man, 7, false),
            Tile::new_number(Suit::Man, 8, false),
            Tile::new_number(Suit::Man, 9, false),
            Tile::new_number(Suit::Pin, 1, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Pin, 5, false),
        ];
        for t in tiles {
            g.game.players[0].hand.add_tile(t);
        }
        g.game.current_player = 0;
        let ok = g.discard_tile("1m");
        assert!(ok, "打牌成功前提");
        assert_eq!(g.get_last_discarder(), Some(0), "#77: last_discarder フィールドを返すこと");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn build_scoring_context_chankan_requires_matching_tile() {
        // #75 regression: pending_chankan != winning_tile のとき is_chankan=false
        use crate::tile::{Tile, Suit};
        let mut g = make_game();
        let winning_tile = Tile::new_number(Suit::Pin, 1, false);
        let different_tile = Tile::new_number(Suit::Pin, 5, false);
        g.game.pending_chankan = Some(different_tile);
        // winning_tile != pending_chankan → is_chankan=false
        let ctx = g.game.build_scoring_context_with_tile(0, false, Some(&winning_tile));
        assert!(!ctx.is_chankan, "#75: pending_chankan != winning_tile のとき is_chankan=false");
        // winning_tile == pending_chankan → is_chankan=true
        let ctx2 = g.game.build_scoring_context_with_tile(0, false, Some(&different_tile));
        assert!(ctx2.is_chankan, "#75: pending_chankan == winning_tile のとき is_chankan=true");
        // winning_tile 未指定 (後方互換) → pending_chankan.is_some() なので is_chankan=true
        let ctx3 = g.game.build_scoring_context(0, false);
        assert!(ctx3.is_chankan, "後方互換: pending_chankan.is_some() なら is_chankan=true");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn uso_riichi_disabled_by_default() {
        // #89: デフォルトでは嘘リーチ無効
        let g = make_game();
        assert!(!g.is_uso_riichi_enabled());
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn uso_riichi_can_riichi_without_tenpai() {
        // #89: uso_riichi_enabled=true のとき非テンパイでも can_riichi=true
        // Game::can_riichi を直接呼び、uso_riichi_enabled の有無による差を検証する
        use crate::tile::{Tile, Suit};
        let mut g = make_game();
        // 手牌を 0 枚にしてテンパイ不成立を確実にする（shanten 簡易実装の影響を避ける）
        g.game.players[0].hand = crate::hand::Hand::new();
        // uso_riichi_enabled=false のときは can_riichi=false（手牌なし=非テンパイ）
        assert!(!g.game.can_riichi(0), "uso_riichi_disabled: 手牌なしは can_riichi=false");
        // uso_riichi_enabled=true のときは門前+未リーチなら can_riichi=true
        g.set_uso_riichi_enabled(true);
        assert!(g.can_riichi(), "#89: uso_riichi_enabled=true なら空手牌でも可");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn uso_riichi_sets_uso_flag_on_declare() {
        // #89: テンパイ不成立で declare_riichi すると uso_riichi=true になる
        let mut g = make_game();
        g.set_uso_riichi_enabled(true);
        // 手牌を空にして確実に非テンパイ（uso ルートを通る）
        g.game.players[0].hand = crate::hand::Hand::new();
        let ok = g.game.declare_riichi(0);
        assert!(ok, "宣言成功前提");
        assert!(g.game.players[0].is_riichi, "is_riichi=true");
        assert!(g.game.players[0].uso_riichi, "#89: uso_riichi=true");
        assert!(g.is_uso_riichi(0), "isUsoRiichi API");
    }

    #[test]
    #[cfg(feature = "wasm")]
    fn uso_riichi_at_draw_loses_only_riichi_stick() {
        // #89: 嘘リーチに「追加の罰符」は無い。流局時の損失は普通の不成立リーチと同じく
        // リーチ棒 1000 点の供託没収のみ（テンパイ判定がノーテンなら別途ノーテン罰符も）。
        // ここでは全員ノーテン（tenpai_count=0 → per_noten=0）なのでノーテン罰符は発生せず、
        // 嘘リーチ者の損失はリーチ棒 1000 点だけになる。
        let mut g = make_game();
        g.set_uso_riichi_enabled(true);
        // 手牌を空にして確実に uso ルートを通す
        g.game.players[0].hand = crate::hand::Hand::new();
        let before: i32 = g.game.players.iter().map(|p| p.score).sum();
        let before_p0 = g.game.players[0].score;
        let sticks_before = g.game.riichi_sticks;
        g.game.declare_riichi(0); // 1000 点を供託（立直棒）→ riichi_sticks += 1
        // 流局: 全員ノーテン
        g.game.resolve_draw(vec![]);
        // 嘘リーチ者はリーチ棒 1000 点を失うだけ（追加罰符なし）
        assert_eq!(
            g.game.players[0].score,
            before_p0 - 1000,
            "#89: 嘘リーチ者の損失はリーチ棒 1000 点のみ（追加罰符は無い）"
        );
        // リーチ棒は供託へ積まれている（次局の和了者が回収）
        assert_eq!(
            g.game.riichi_sticks,
            sticks_before + 1,
            "#89: リーチ棒は供託 (riichi_sticks) に積まれる"
        );
        // ゼロサム保存: 全員の score 合計 + 供託リーチ棒(1000×本数) は不変
        let after: i32 = g.game.players.iter().map(|p| p.score).sum();
        assert_eq!(
            after + (g.game.riichi_sticks as i32) * 1000,
            before + (sticks_before as i32) * 1000,
            "#89: 点棒総和 (scores + riichi_sticks) はゼロサムで保存される"
        );
    }
}
