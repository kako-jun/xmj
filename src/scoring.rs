use crate::tile::{Tile, TileType, Honor, Suit};
use crate::hand::{Hand, MeldType};
use std::collections::{HashMap, HashSet};

/// ドラ表示牌から実際のドラ牌を計算する。
///
/// - 数牌: 値 +1 (9 の次は 1 にループ)
/// - 風牌: 東→南→西→北→東 ループ
/// - 三元牌: 白→發→中→白 ループ
pub fn dora_indicator_to_dora(indicator: &Tile) -> Tile {
    match indicator.tile_type {
        TileType::Number { suit, value } => {
            let next = if value >= 9 { 1 } else { value + 1 };
            Tile::new_number(suit, next, false)
        }
        TileType::Honor(h) => {
            let next = match h {
                Honor::Ton => Honor::Nan,
                Honor::Nan => Honor::Shaa,
                Honor::Shaa => Honor::Pei,
                Honor::Pei => Honor::Ton,
                Honor::Haku => Honor::Hatsu,
                Honor::Hatsu => Honor::Chun,
                Honor::Chun => Honor::Haku,
            };
            Tile::new_honor(next)
        }
    }
}

/// 役判定に必要な対局・プレイヤー状態。
///
/// 既存 `calculate_score(hand, tile, is_tsumo, is_dealer)` API は本構造を default で
/// 組んで呼ぶラッパとして残し、状況役を扱う新規呼び出し元は
/// `calculate_score_with_context` を経由する。
#[derive(Debug, Clone)]
pub struct ScoringContext {
    pub is_tsumo: bool,
    pub is_dealer: bool,
    /// プレイヤー状態
    pub is_riichi: bool,
    pub is_double_riichi: bool,
    pub is_ippatsu: bool,
    /// 状況役
    pub is_haitei: bool,
    pub is_houtei: bool,
    pub is_rinshan: bool,
    pub is_chankan: bool,
    /// 場風 (East/South/West/North のいずれか)
    pub round_wind: Honor,
    /// 自風 (East/South/West/North のいずれか)
    pub seat_wind: Honor,
    /// ドラ表示牌
    pub dora_indicators: Vec<Tile>,
    /// 裏ドラ表示牌 (立直成立時のみ集計対象)
    pub uradora_indicators: Vec<Tile>,
    /// #51: 天和 (親の配牌時点で和了)。winner == dealer かつ自家ツモ + 全員 discard 0 +
    /// 全員副露 0 のときに `Game::build_scoring_context` から true で渡される。
    pub is_tenhou: bool,
    /// #51: 地和 (子の第一ツモで和了)。winner != dealer かつ自家ツモ +
    /// 当該プレイヤーの discards 0 + これまでに誰も鳴いていないときに true。
    pub is_chiihou: bool,
}

impl Default for ScoringContext {
    fn default() -> Self {
        Self {
            is_tsumo: false,
            is_dealer: false,
            is_riichi: false,
            is_double_riichi: false,
            is_ippatsu: false,
            is_haitei: false,
            is_houtei: false,
            is_rinshan: false,
            is_chankan: false,
            round_wind: Honor::Ton,
            seat_wind: Honor::Ton,
            dora_indicators: Vec::new(),
            uradora_indicators: Vec::new(),
            is_tenhou: false,
            is_chiihou: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    // 混老頭。`check_honroutou` で判定し `calculate_score_with_context` から push 済み。
    Honroutou,

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

#[derive(Debug, Clone, Default)]
pub struct ScoringResult {
    pub han: u32,
    pub fu: u32,
    pub yaku: Vec<Yaku>,
    pub base_points: u32,
    pub total_points: u32,
    /// ドラ枚数 (han に加算済み、UI 表示用に枚数保持)
    pub dora: u32,
    /// 裏ドラ枚数 (立直成立時のみカウント、han に加算済み)
    pub uradora: u32,
    /// 赤ドラ枚数 (han に加算済み)
    pub akadora: u32,
    /// 槓ドラ枚数 (dora と合算してフィールド分離保持。
    /// 現状の Game 実装は明槓/暗槓時に `dora_indicators` に追加するため、
    /// `dora` には槓ドラぶんも含まれている。本フィールドは表示用の補助で
    /// 0 のままになるケースが多い)
    pub kandora: u32,
    /// 役満の倍率 (#42 #52)。単役満 = 1、ダブル役満 = 2、…。
    /// 0 のときは非役満手。`calculate_base_points` はこの値が 1 以上のとき
    /// `8000 * yakuman_count` を返す (han は 13 固定で参考値)。
    /// 複数の役満が同時成立した場合は単純加算 (例: 大三元 + 字一色 = 2)。
    pub yakuman_count: u32,
}

pub struct ScoringEngine;

impl ScoringEngine {
    /// 旧 API。後方互換用。新規コードは `calculate_score_with_context` を使う。
    pub fn calculate_score(
        hand: &Hand,
        winning_tile: &Tile,
        is_tsumo: bool,
        is_dealer: bool,
    ) -> Option<ScoringResult> {
        let ctx = ScoringContext {
            is_tsumo,
            is_dealer,
            ..ScoringContext::default()
        };
        Self::calculate_score_with_context(hand, winning_tile, &ctx)
    }

    /// 役判定に必要な対局・プレイヤー状態をフルに受け取って点数計算する。
    ///
    /// 立直系 (#49) / 状況役 (#50) / 場風自風 (#53) / ドラ (#54) を含む。
    pub fn calculate_score_with_context(
        hand: &Hand,
        winning_tile: &Tile,
        ctx: &ScoringContext,
    ) -> Option<ScoringResult> {
        let is_tsumo = ctx.is_tsumo;
        let is_dealer = ctx.is_dealer;
        let mut yaku = Vec::new();
        let mut han = 0;
        // #42 #51 #52: 役満の倍率カウンタ。単役満で +1、ダブル役満で +2 ずつ加算する。
        // 役満の有無は `yakuman_count > 0` で判定し、base_points は `8000 * yakuman_count`。
        let mut yakuman_count: u32 = 0;
        // 門前判定: 暗槓 (is_open=false) は門前を崩さない。チー / ポン / 大明槓 / 加槓
        // (is_open=true) のいずれかがあれば非門前。暗槓のみの手は立直・門前ツモ・
        // 一盃口・二盃口・平和 (槓があると形上不成立だが) などの門前役の対象になる。
        let is_menzen = hand.get_melds().iter().all(|m| !m.is_open);

        // 手牌情報の取得
        // Issue #33: 副露がある場合、手牌 (`hand.get_tiles()`) は 11/8/5/2 枚しかない。
        // 全 14 枚を見る役 (タンヤオ / 清一色 / 混一色 / 国士 / 字一色 / 緑一色 / 清老頭) では
        // 副露の構成牌も含めて評価する必要があるため、ここで結合する。
        let mut all_tiles = hand.get_tiles().clone();
        for meld in hand.get_melds() {
            for t in &meld.tiles {
                all_tiles.push(*t);
            }
        }
        all_tiles.push(*winning_tile);

        // 役満チェック
        // #42: 国士無双は和了形と「13 面待ち (純正)」を別判定する。
        // 13 面: 和了前の手牌 13 枚に 13 種すべての么九牌が 1 枚ずつ揃っており、
        // winning_tile が么九牌のどれかに合流する形。yakuman_count += 2 (ダブル役満)。
        if Self::check_kokushi(hand, &all_tiles) {
            yaku.push(Yaku::Kokushi);
            han += 13;
            if Self::check_kokushi_juusan_mendachi(hand, winning_tile) {
                yakuman_count += 2;
            } else {
                yakuman_count += 1;
            }
        }

        if Self::check_suuankou(hand, winning_tile, is_tsumo) {
            yaku.push(Yaku::Suuankou);
            han += 13;
            yakuman_count += 1;
        }

        if Self::check_daisangen(hand) {
            yaku.push(Yaku::Daisangen);
            han += 13;
            yakuman_count += 1;
        }

        if Self::check_tsuuiisou(&all_tiles) {
            yaku.push(Yaku::Tsuuiisou);
            han += 13;
            yakuman_count += 1;
        }

        if Self::check_ryuuiisou(&all_tiles) {
            yaku.push(Yaku::Ryuuiisou);
            han += 13;
            yakuman_count += 1;
        }

        if Self::check_chinroutou(&all_tiles) {
            yaku.push(Yaku::Chinroutou);
            han += 13;
            yakuman_count += 1;
        }

        // #42: 九蓮宝燈は通常 + 9 面待ち (純正) を区別する。
        // 9 面: 和了前の手牌 13 枚が 1112345678999 (同色) かつ winning_tile が同色任意。
        // → yakuman_count += 2 (ダブル役満)。
        if Self::check_chuuren(&all_tiles, is_menzen) {
            yaku.push(Yaku::Chuuren);
            han += 13;
            if Self::check_chuuren_kyuumendachi(hand, winning_tile, is_menzen) {
                yakuman_count += 2;
            } else {
                yakuman_count += 1;
            }
        }

        // #52: 大四喜 / 小四喜 (排他)。
        if Self::check_daisuushii(hand, winning_tile) {
            yaku.push(Yaku::Daisuushii);
            han += 26;
            yakuman_count += 2; // ダブル役満
        } else if Self::check_shousuushii(hand, winning_tile) {
            yaku.push(Yaku::Shousuushii);
            han += 13;
            yakuman_count += 1;
        }

        // #52: 四槓子 (役満)。四槓子なら三槓子は重複させない (後段でガード)。
        if Self::check_suukantsu(hand) {
            yaku.push(Yaku::Suukantsu);
            han += 13;
            yakuman_count += 1;
        }

        // #51: 天和 (親配牌時和了) / 地和 (子第一ツモ和了)。
        // ScoringContext からフラグを受け取って push する。
        if ctx.is_tenhou {
            yaku.push(Yaku::Tenhou);
            han += 13;
            yakuman_count += 1;
        }
        if ctx.is_chiihou {
            yaku.push(Yaku::Chiihou);
            han += 13;
            yakuman_count += 1;
        }

        // 役満がある場合は他の役をチェックしない
        if yakuman_count > 0 {
            // 役満は符を点数計算に使わない。表示用に標準的な 20/30 符を入れておく
            // (ツモ +2 → 切り上げ 30、ロン 20)。
            let fu = if is_tsumo { 30 } else { 20 };
            let base_points = 8000 * yakuman_count;
            let total_points = Self::calculate_total_points(base_points, is_dealer, is_tsumo);

            return Some(ScoringResult {
                han,
                fu,
                yaku,
                base_points,
                total_points,
                dora: 0,
                uradora: 0,
                akadora: 0,
                kandora: 0,
                yakuman_count,
            });
        }

        // #49: 立直系 (門前のみ)
        if is_menzen {
            if ctx.is_double_riichi {
                yaku.push(Yaku::DoubleRiichi);
                han += 2;
            } else if ctx.is_riichi {
                yaku.push(Yaku::Riichi);
                han += 1;
            }
            // 一発は立直 (or ダブル立直) 成立時のみ意味を持つ
            if ctx.is_ippatsu && (ctx.is_riichi || ctx.is_double_riichi) {
                yaku.push(Yaku::Ippatsu);
                han += 1;
            }
        }

        // #50: 状況役
        if ctx.is_haitei {
            yaku.push(Yaku::Haitei);
            han += 1;
        }
        if ctx.is_houtei {
            yaku.push(Yaku::Houtei);
            han += 1;
        }
        if ctx.is_rinshan {
            yaku.push(Yaku::Rinshan);
            han += 1;
        }
        if ctx.is_chankan {
            yaku.push(Yaku::Chankan);
            han += 1;
        }

        // ===== 共通役 (分解非依存: 牌集合 / 副露ベース) =====
        // 七対子・通常形いずれの解釈でも同じく成立し得る役。これらをまず `yaku`/`han`
        // に積んだあと、構造依存役 (一盃口/三色/一通/チャンタ/三暗刻/平和 など) を
        // `yaku_struct::evaluate_best` で通常形分解から、または七対子分岐で計上し、
        // 高得点の解釈を採用する。

        // タンヤオ
        if Self::check_tanyao(&all_tiles) {
            yaku.push(Yaku::Tanyao);
            han += 1;
        }

        // 門前ツモ
        if is_tsumo && is_menzen {
            yaku.push(Yaku::Tsumo);
            han += 1;
        }

        // 役牌 (#53): 三元牌 + 場風 + 自風。場風 == 自風 (連風) は 2 回計上。
        let honor_yakuhai_targets: Vec<Honor> = vec![
            Honor::Haku,
            Honor::Hatsu,
            Honor::Chun,
            ctx.round_wind,
            ctx.seat_wind,
        ];
        for honor in honor_yakuhai_targets {
            if Self::check_yakuhai_full(hand, winning_tile, honor) {
                yaku.push(Yaku::Yakuhai(honor));
                han += 1;
            }
        }

        // 対々和
        if Self::check_toitoi(hand, winning_tile) {
            yaku.push(Yaku::Toitoi);
            han += 2;
        }

        // 三槓子 (#52)
        if Self::check_sankantsu(hand) {
            yaku.push(Yaku::Sankantsu);
            han += 2;
        }

        // 混老頭 (#52)
        if Self::check_honroutou(&all_tiles) {
            yaku.push(Yaku::Honroutou);
            han += 2;
        }

        // 混一色 / 清一色
        if Self::check_honitsu(&all_tiles) {
            yaku.push(Yaku::Honitsu);
            han += if is_menzen { 3 } else { 2 };
        }
        if Self::check_chinitsu(&all_tiles) {
            yaku.retain(|y| y != &Yaku::Honitsu);
            yaku.push(Yaku::Chinitsu);
            han = han.saturating_sub(if is_menzen { 3 } else { 2 }) + if is_menzen { 6 } else { 5 };
        }

        // ===== 構造依存役: 通常形 vs 七対子 で高得点の解釈を採用 =====
        // 候補ごとに (yaku, yaku_han, fu) を作り、(han, fu) が最大のものを選ぶ。
        let mut candidates: Vec<(Vec<Yaku>, u32, u32)> = Vec::new();

        // 通常形 (4 面子 1 雀頭): 構造役 + 符を分解から算出。
        if let Some(s) = crate::yaku_struct::evaluate_best(hand, winning_tile, ctx, is_menzen) {
            let mut y = yaku.clone();
            y.extend(s.yaku.iter().cloned());
            candidates.push((y, han + s.han, s.fu));
        }

        // 七対子 (門前のみ): 固定 2 飜・25 符。構造役は付かない。
        if is_menzen && Self::check_chiitoitsu(&all_tiles) {
            let mut y = yaku.clone();
            y.push(Yaku::Chiitoitsu);
            candidates.push((y, han + 2, 25));
        }

        // どの和了形も取れない場合 (理論上 caller が和了を保証するため稀)。
        let (final_yaku, mut final_han, fu) = match candidates
            .into_iter()
            .max_by_key(|(_, h, f)| (*h, *f))
        {
            Some(best) => best,
            None => return None,
        };
        yaku = final_yaku;
        han = final_han;

        if han == 0 {
            // 役なし。ドラのみでは和了不可なので None を返す。
            return None;
        }

        // #54: ドラ / 裏ドラ / 赤ドラ を計算して han に加算する。
        let dora = Self::count_dora(hand, winning_tile, &ctx.dora_indicators);
        let uradora = if ctx.is_riichi || ctx.is_double_riichi {
            Self::count_dora(hand, winning_tile, &ctx.uradora_indicators)
        } else {
            0
        };
        let akadora = Self::count_akadora(hand, winning_tile);
        let kandora = 0;
        final_han += dora + uradora + akadora;
        han = final_han;

        let base_points = Self::calculate_base_points(han, fu);
        let total_points = Self::calculate_total_points(base_points, is_dealer, is_tsumo);

        Some(ScoringResult {
            han,
            fu,
            yaku,
            base_points,
            total_points,
            dora,
            uradora,
            akadora,
            kandora,
            yakuman_count: 0,
        })
    }
    
    // タンヤオ（断么九）
    fn check_tanyao(tiles: &[Tile]) -> bool {
        tiles.iter().all(|tile| match tile.tile_type {
            TileType::Number { value, .. } => value >= 2 && value <= 8,
            TileType::Honor(_) => false,
        })
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

    /// 副露 + 手牌内暗刻を含む役牌判定 (#53)。
    ///
    /// 簡易実装:
    /// - 副露 (Pon/Kan) で対象 honor を抱えていれば成立
    /// - 手牌 (winning_tile を含めた全字牌) に同種 honor が 3 枚以上あれば成立
    ///   (雀頭 2 枚での成立は意図的に弾く)
    ///
    /// TODO(#53 follow-up): agari パターン完全分解版に置き換え、
    /// 「雀頭が役牌」「暗刻として実際に組まれている」を厳密に判定する。
    fn check_yakuhai_full(hand: &Hand, winning_tile: &Tile, honor: Honor) -> bool {
        if Self::check_yakuhai(hand, honor) {
            return true;
        }
        // 手牌内 (副露は除く) の同種 honor 数 + winning_tile が同種なら +1
        let mut count = 0;
        for t in hand.get_tiles() {
            if let TileType::Honor(h) = t.tile_type {
                if h == honor {
                    count += 1;
                }
            }
        }
        if let TileType::Honor(h) = winning_tile.tile_type {
            if h == honor {
                count += 1;
            }
        }
        count >= 3
    }

    /// ドラ枚数を数える (#54)。
    ///
    /// 各 dora_indicator を `dora_indicator_to_dora` で「実際のドラ牌」に変換し、
    /// 手牌 (tiles + 副露 tiles + winning_tile) に何枚含まれるかを集計する。
    /// 赤ドラは含まない (別カウント)。
    fn count_dora(hand: &Hand, winning_tile: &Tile, indicators: &[Tile]) -> u32 {
        if indicators.is_empty() {
            return 0;
        }
        let mut all_tiles = hand.get_tiles().clone();
        for meld in hand.get_melds() {
            for t in &meld.tiles {
                all_tiles.push(*t);
            }
        }
        all_tiles.push(*winning_tile);
        let mut count = 0u32;
        for ind in indicators {
            let dora = dora_indicator_to_dora(ind);
            for t in &all_tiles {
                // tile_type のみ一致で判定 (赤ドラは別カウント、is_red 違いは
                // PartialEq で別牌扱いになるため tile_type だけ比較する)
                if t.tile_type == dora.tile_type {
                    count += 1;
                }
            }
        }
        count
    }

    /// 赤ドラ枚数を数える (#54)。
    fn count_akadora(hand: &Hand, winning_tile: &Tile) -> u32 {
        let mut count = 0u32;
        for t in hand.get_tiles() {
            if t.is_red {
                count += 1;
            }
        }
        for meld in hand.get_melds() {
            for t in &meld.tiles {
                if t.is_red {
                    count += 1;
                }
            }
        }
        if winning_tile.is_red {
            count += 1;
        }
        count
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

    // 対々和
    fn check_toitoi(hand: &Hand, winning_tile: &Tile) -> bool {
        // 全面子が刻子（または槓子）。副露があれば Chi が混じった瞬間に不成立。
        // Issue #33: 旧実装は `melds.len() + 4 != 4`（実質 `melds.len() == 0`）で
        // 副露ありトイトイを一律弾いていた。副露込みの 4 面子 1 雀頭で
        // 全面子が刻子なら成立する。
        if !hand.get_melds().iter().all(|meld| {
            matches!(meld.meld_type, MeldType::Pon | MeldType::Kan)
        }) {
            return false;
        }
        // 残り手牌（雀頭 + 残りの刻子）を分解できるか確認
        // 副露が N 個 → 残り手牌（+winning_tile を含めて） (4-N) 個の刻子 + 雀頭
        Self::check_remaining_all_triplets(hand, winning_tile)
    }

    /// 副露を除く残り手牌に winning_tile を加えたものが、
    /// `(4 - melds.len())` 個の刻子 + 雀頭 だけで構成できるかをチェックする。
    fn check_remaining_all_triplets(hand: &Hand, winning_tile: &Tile) -> bool {
        let mut tiles = hand.get_tiles().clone();
        tiles.push(*winning_tile);
        let melds_needed = 4 - hand.get_melds().len();
        // 残り手牌（+ winning_tile）の枚数は (melds_needed * 3 + 2) であるはず
        if tiles.len() != melds_needed * 3 + 2 {
            return false;
        }
        let mut tile_map: HashMap<Tile, usize> = HashMap::new();
        for t in &tiles {
            *tile_map.entry(*t).or_insert(0) += 1;
        }
        // 雀頭候補を順に試す
        let unique: Vec<Tile> = tile_map.keys().copied().collect();
        for head in &unique {
            if tile_map.get(head).copied().unwrap_or(0) < 2 {
                continue;
            }
            // 副露で暗カン化済みの前提。手牌内の 4 枚抱えは現実的に起こらない
            let mut m = tile_map.clone();
            *m.get_mut(head).unwrap() -= 2;
            // 残りがすべて count == 3 / 0 ならトイトイ
            // (count==4 になるケースは前述の通り 4 枚抱えで暗カン化していない異常系のみ)
            if m.values().all(|&c| c == 0 || c == 3) {
                return true;
            }
        }
        false
    }

    // 三暗刻 / 三色同刻 / 小三元 は `yaku_struct::evaluate_best` (面子分解ベース) で判定する。

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
    fn check_kokushi(hand: &Hand, tiles: &[Tile]) -> bool {
        // 副露があれば国士無双は不成立 (check_chiitoitsu との対称性)
        if !hand.get_melds().is_empty() {
            return false;
        }
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
    //
    // 4 つの面子全てが暗刻 (暗槓を含む)、雀頭 1 つ。
    // - 単騎和了 (winning_tile が雀頭) → ロン / ツモどちらでも成立 (四暗刻単騎)
    // - シャンポン待ち → ツモのみ成立 (ロンだと最後の刻子が明刻扱いで三暗刻に格下げ)
    // #130: 暗槓 (is_open=false の Kan) は暗刻として四暗刻に数える。手牌側は
    // (4 - 暗槓数) 個の暗刻 + 雀頭で構成できればよい。チー/ポン/明槓/加槓
    // (is_open=true) が 1 つでもあれば四暗刻不可。
    fn check_suuankou(hand: &Hand, winning_tile: &Tile, is_tsumo: bool) -> bool {
        let melds = hand.get_melds();
        // 開いた副露があれば四暗刻不可。暗槓以外の副露 (= is_open=true) を弾く。
        if melds.iter().any(|m| m.is_open) {
            return false;
        }
        // 暗槓以外の closed meld は存在しない想定だが、念のため Kan のみ許可。
        if melds
            .iter()
            .any(|m| !matches!(m.meld_type, MeldType::Kan))
        {
            return false;
        }
        let ankan_count = melds.len();
        let melds_needed = match 4usize.checked_sub(ankan_count) {
            Some(n) => n,
            None => return false,
        };
        // 暗槓 4 つ (四槓子) は四暗刻の判定対象外 (雀頭のみ残る)。melds_needed=0 のときは
        // 手牌 + winning が雀頭のみで、刻子が手牌側に無いので四暗刻ではない (四槓子側で処理)。
        if melds_needed == 0 {
            return false;
        }
        let mut concealed = hand.get_tiles().clone();
        concealed.push(*winning_tile);
        crate::agari::is_suuankou_n(&concealed, winning_tile, is_tsumo, melds_needed)
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
    //
    // Issue #34: 1112345678999 + 同色のどれか 1 枚で構成される清一色 14 枚手。
    // 副露なしの場合のみ役満として認める。
    fn check_chuuren(tiles: &[Tile], is_menzen: bool) -> bool {
        if !is_menzen || tiles.len() != 14 {
            return false;
        }
        crate::agari::is_chuuren(tiles)
    }

    /// #42: 国士無双 13 面待ち (純正国士、ダブル役満)。
    ///
    /// 和了前の手牌 13 枚に 13 種の么九牌が 1 枚ずつ全て揃っており、
    /// winning_tile が么九牌のどれかであれば 13 面待ち。
    /// 副露があれば国士は不成立 (本関数は呼び出し側で `check_kokushi` が
    /// true を返した後に呼ばれる前提)。
    fn check_kokushi_juusan_mendachi(hand: &Hand, winning_tile: &Tile) -> bool {
        if !hand.get_melds().is_empty() {
            return false;
        }
        // winning_tile が么九牌か?
        if !Self::is_yaochuu(winning_tile) {
            return false;
        }
        let tiles = hand.get_tiles();
        if tiles.len() != 13 {
            return false;
        }
        let yaochuus: [Tile; 13] = [
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
        let mut tile_map: HashMap<Tile, usize> = HashMap::new();
        for t in tiles {
            *tile_map.entry(*t).or_insert(0) += 1;
        }
        // 13 種それぞれが手牌に 1 枚ずつ含まれる (= 13 面待ち)
        yaochuus
            .iter()
            .all(|y| tile_map.get(y).copied().unwrap_or(0) == 1)
    }

    /// 么九牌 (1, 9, 字牌) 判定。
    fn is_yaochuu(tile: &Tile) -> bool {
        match tile.tile_type {
            TileType::Number { value, .. } => value == 1 || value == 9,
            TileType::Honor(_) => true,
        }
    }

    /// #42: 九蓮宝燈 9 面待ち (純正九蓮、ダブル役満)。
    ///
    /// 和了前の手牌 13 枚が `1112345678999` (同色) かつ winning_tile が同色任意の牌
    /// のときに 9 面待ち成立。副露なしのときのみ。
    fn check_chuuren_kyuumendachi(hand: &Hand, winning_tile: &Tile, is_menzen: bool) -> bool {
        if !is_menzen {
            return false;
        }
        if !hand.get_melds().is_empty() {
            return false;
        }
        let tiles = hand.get_tiles();
        if tiles.len() != 13 {
            return false;
        }
        // winning_tile が数牌で、手牌と同色か
        let win_suit = match winning_tile.tile_type {
            TileType::Number { suit, .. } => suit,
            _ => return false,
        };
        // 手牌すべてが同色数牌 (win_suit と同じ) か確認しつつ各数値をカウント
        let mut counts = [0u8; 10];
        for t in tiles {
            match t.tile_type {
                TileType::Number { suit, value } if suit == win_suit => {
                    counts[value as usize] += 1;
                }
                _ => return false,
            }
        }
        // 1112345678999 = [_,3,1,1,1,1,1,1,1,3]
        counts[1] == 3
            && counts[2] == 1
            && counts[3] == 1
            && counts[4] == 1
            && counts[5] == 1
            && counts[6] == 1
            && counts[7] == 1
            && counts[8] == 1
            && counts[9] == 3
    }

    /// #52: 大四喜。4 種の風牌すべてを刻子 / 槓子で抱える (ダブル役満)。
    ///
    /// 副露 (Pon/Kan) と手牌中の暗刻 (winning_tile を含めて 3 枚以上) の合計で
    /// 4 種すべてを抑えていれば成立。雀頭が風牌になっている場合は小四喜になり、
    /// 大四喜は不成立。
    ///
    /// #73: `wind_count_raw` + winning_tile の組み合わせで刻子判定を行う。
    /// タンキ待ち (手牌 1 枚 + winning_tile) は count_raw=1 のため刻子に算入されない。
    fn check_daisuushii(hand: &Hand, winning_tile: &Tile) -> bool {
        let winds = [Honor::Ton, Honor::Nan, Honor::Shaa, Honor::Pei];
        winds
            .iter()
            .all(|h| Self::wind_is_triplet(hand, winning_tile, *h))
    }

    /// #52: 小四喜。4 種の風牌のうち 3 種が刻子 / 槓子、残り 1 種が雀頭 (役満)。
    ///
    /// #73: 雀頭判定は「count_raw == 2 かつ winning_tile がその風でない」または
    /// 「count_raw == 1 かつ winning_tile がその風 (タンキ)」とする。
    fn check_shousuushii(hand: &Hand, winning_tile: &Tile) -> bool {
        let winds = [Honor::Ton, Honor::Nan, Honor::Shaa, Honor::Pei];
        let mut triplet = 0;
        let mut pair = 0;
        for h in &winds {
            if Self::wind_is_triplet(hand, winning_tile, *h) {
                triplet += 1;
            } else if Self::wind_is_pair(hand, winning_tile, *h) {
                pair += 1;
            }
        }
        triplet == 3 && pair == 1
    }

    /// 指定風牌が刻子 / 槓子として成立するか判定する。
    ///
    /// 以下のいずれかを満たせば刻子とみなす:
    /// - 副露 Pon/Kan がある (= それだけで刻子/槓子確定)
    /// - 手牌内に 3 枚以上ある (暗刻)
    /// - 手牌内に 2 枚あり、かつ winning_tile が同じ風 (シャンポン和了で刻子完成)
    ///
    /// #73: タンキ待ち (手牌 1 枚 + winning_tile) は count_raw=1 のため刻子不成立。
    fn wind_is_triplet(hand: &Hand, winning_tile: &Tile, honor: Honor) -> bool {
        // 副露 Pon/Kan があれば刻子/槓子確定
        for meld in hand.get_melds() {
            if matches!(meld.meld_type, MeldType::Pon | MeldType::Kan) {
                if let Some(t) = meld.tiles.first() {
                    if let TileType::Honor(h) = t.tile_type {
                        if h == honor {
                            return true;
                        }
                    }
                }
            }
        }
        let raw = Self::wind_count_raw(hand, honor);
        // 手牌 3 枚以上 → 暗刻
        if raw >= 3 {
            return true;
        }
        // 手牌 2 枚 + winning_tile が同じ風 → シャンポン和了で刻子完成
        if raw >= 2 {
            if let TileType::Honor(h) = winning_tile.tile_type {
                if h == honor {
                    return true;
                }
            }
        }
        false
    }

    /// 指定風牌が雀頭として成立するか判定する。
    ///
    /// 以下のいずれかを満たせば雀頭とみなす:
    /// - 手牌内に 2 枚あり、winning_tile がこの風でない (手牌 2 枚で雀頭完成)
    /// - 手牌内に 1 枚あり、winning_tile が同じ風 (タンキ和了で雀頭完成)
    fn wind_is_pair(hand: &Hand, winning_tile: &Tile, honor: Honor) -> bool {
        // 副露 Pon/Kan があれば雀頭ではなく刻子/槓子
        for meld in hand.get_melds() {
            if matches!(meld.meld_type, MeldType::Pon | MeldType::Kan) {
                if let Some(t) = meld.tiles.first() {
                    if let TileType::Honor(h) = t.tile_type {
                        if h == honor {
                            return false;
                        }
                    }
                }
            }
        }
        let raw = Self::wind_count_raw(hand, honor);
        let is_winning = matches!(winning_tile.tile_type, TileType::Honor(h) if h == honor);
        // 手牌 2 枚で winning_tile がこの風でない → 手牌のみで雀頭完成
        if raw == 2 && !is_winning {
            return true;
        }
        // 手牌 1 枚 + winning_tile がこの風 → タンキ和了で雀頭完成
        if raw == 1 && is_winning {
            return true;
        }
        false
    }

    /// 手牌 (add_meld 時に tiles から除去済みのため副露牌は含まない) における
    /// 指定風牌の枚数を返す。winning_tile は含めない。
    ///
    /// #73: winning_tile を含めない生カウントを返す。刻子/雀頭の判定は
    /// `wind_is_triplet` / `wind_is_pair` が winning_tile を考慮して行う。
    fn wind_count_raw(hand: &Hand, honor: Honor) -> u32 {
        let mut count: u32 = 0;
        for t in hand.get_tiles() {
            if let TileType::Honor(h) = t.tile_type {
                if h == honor {
                    count += 1;
                }
            }
        }
        count
    }

    /// #52: 四槓子。4 つの槓子で和了 (役満)。
    fn check_suukantsu(hand: &Hand) -> bool {
        hand.get_melds()
            .iter()
            .filter(|m| matches!(m.meld_type, MeldType::Kan))
            .count()
            == 4
    }

    /// #52: 三槓子 (二飜)。3 つの槓子で和了。
    fn check_sankantsu(hand: &Hand) -> bool {
        hand.get_melds()
            .iter()
            .filter(|m| matches!(m.meld_type, MeldType::Kan))
            .count()
            == 3
    }

    /// #52: 混老頭 (二飜)。すべての構成牌が么九 (1/9/字牌) で、字牌を 1 枚以上含む。
    ///
    /// 字牌が無い場合は清老頭 (役満) になるため、ここでは「字牌を含む」を必須条件にする。
    fn check_honroutou(tiles: &[Tile]) -> bool {
        let mut has_honor = false;
        for t in tiles {
            match t.tile_type {
                TileType::Number { value, .. } => {
                    if value != 1 && value != 9 {
                        return false;
                    }
                }
                TileType::Honor(_) => {
                    has_honor = true;
                }
            }
        }
        has_honor
    }
    
    /// 基本点 (base points) を算出する。
    ///
    /// 1-4 飜は `fu * 2^(han+2)`。ただし満貫 (2000) を超えたら満貫で頭打ち (#108 #4)。
    /// 5 飜=満貫 / 6-7=跳満 / 8-10=倍満 / 11-12=三倍満 / 13+=数え役満。
    fn calculate_base_points(han: u32, fu: u32) -> u32 {
        match han {
            1..=4 => {
                let raw = fu * (1 << (han + 2));
                raw.min(2000) // 満貫頭打ち (例: 4飜40符=2560 → 2000)
            }
            5 => 2000,       // 満貫
            6..=7 => 3000,   // 跳満
            8..=10 => 4000,  // 倍満
            11..=12 => 6000, // 三倍満
            _ => 8000,       // 数え役満
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

/// 5 枚麻雀の簡易点数計算。
///
/// 5 枚麻雀（FiveTile モード）専用の和了点数。
/// - 基礎点: 1000
/// - タンヤオ（手牌 5 枚すべて 2-8 の数牌）成立で +1000
///
/// 5 枚麻雀の和了形では「アガリ牌は手牌 5 枚の構成牌の 1 つ」であるため、
/// 手牌のみ評価すれば十分。`win_tile` は和了役（ツモ/ロン等）の拡張用の引数として保持する。
///
/// 戻り値は和了者が受け取る合計点数。
pub fn score_five_tile(hand: &Hand, _win_tile: &Tile) -> i32 {
    let tiles = hand.get_tiles();

    let mut score: i32 = 1000;

    let is_tanyao = tiles.iter().all(|tile| match tile.tile_type {
        TileType::Number { value, .. } => (2..=8).contains(&value),
        TileType::Honor(_) => false,
    });
    if is_tanyao {
        score += 1000;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tile::{Tile, Suit};

    // #108 #4: 満貫頭打ちの回帰テスト。
    #[test]
    fn test_base_points_mangan_cap() {
        // 4 飜 40 符 = 40 * 2^6 = 2560 → 満貫 2000 で頭打ち。
        assert_eq!(ScoringEngine::calculate_base_points(4, 40), 2000);
        // 3 飜 70 符 = 70 * 32 = 2240 → 2000。
        assert_eq!(ScoringEngine::calculate_base_points(3, 70), 2000);
        // 4 飜 30 符 = 30 * 64 = 1920 < 2000 → 頭打ちなし。
        assert_eq!(ScoringEngine::calculate_base_points(4, 30), 1920);
        // 3 飜 30 符 = 30 * 32 = 960。
        assert_eq!(ScoringEngine::calculate_base_points(3, 30), 960);
        // 5 飜 = 満貫固定 2000。
        assert_eq!(ScoringEngine::calculate_base_points(5, 20), 2000);
        // 13 飜以上 = 数え役満 8000。
        assert_eq!(ScoringEngine::calculate_base_points(13, 20), 8000);
    }

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

    /// 5 枚麻雀: タンヤオ（手牌 5 枚すべて 2-8）成立で 2000 点
    /// 手牌: 2m 2m 5p 5p 5p（雀頭 + 刻子の完成形、すべて 2-8）
    #[test]
    fn test_score_five_tile_tanyao() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Man, 2, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));

        // win_tile は手牌の構成牌の 1 つ（新仕様）
        let win_tile = Tile::new_number(Suit::Man, 2, false);
        let score = score_five_tile(&hand, &win_tile);

        // 基礎点 1000 + タンヤオ 1000 = 2000
        assert_eq!(score, 2000, "タンヤオで 2000 点");
    }

    /// 5 枚麻雀: タンヤオ不成立（1 or 9 or 字牌含む）なら基礎点のみ
    /// 手牌: 1m 1m 5p 5p 5p（1m を含むのでタンヤオ不可）
    #[test]
    fn test_score_five_tile_no_tanyao() {
        let mut hand = Hand::new();
        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Man, 1, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));
        hand.add_tile(Tile::new_number(Suit::Pin, 5, false));

        let win_tile = Tile::new_number(Suit::Man, 1, false);
        let score = score_five_tile(&hand, &win_tile);

        // 基礎点 1000 のみ
        assert_eq!(score, 1000, "タンヤオなしは基礎点のみ");
    }

    // ==================== Issue #33: 副露あり scoring 回帰防止テスト ====================
    // S2: all_tiles 結合バグ修正 + check_toitoi 修正の回帰防止
    // Q10: 大三元の副露成立を確認

    use crate::hand::Meld;

    fn open_pon(tile: Tile) -> Meld {
        Meld {
            meld_type: MeldType::Pon,
            tiles: vec![tile, tile, tile],
            is_open: true,
            ..Default::default()
        }
    }

    fn open_chi(suit: Suit, start: u8) -> Meld {
        Meld {
            meld_type: MeldType::Chi,
            tiles: vec![
                Tile::new_number(suit, start, false),
                Tile::new_number(suit, start + 1, false),
                Tile::new_number(suit, start + 2, false),
            ],
            is_open: true,
            ..Default::default()
        }
    }

    /// 副露あり混一色:
    /// 副露: ポン 2m2m2m (鳴き), 残り手牌: 3m 4m 5m 6m 7m 8m 9m9m + 東東 + 白, 和了: 白
    /// 構成: [2m2m2m] + 3m4m5m + 6m7m8m + 9m9m9m? — 9m が 2 枚 → 雀頭。
    /// 整理: [2m2m2m] + 3m4m5m + 6m7m8m + 東東東 (刻子) + 白白 (雀頭)
    /// → 残り手牌 10 枚: 3m 4m 5m 6m 7m 8m 東 東 東 白, 和了: 白 (シャンポン待ち)
    #[test]
    fn test_honitsu_with_pon() {
        let mut hand = Hand::new();
        for t in [
            Tile::new_number(Suit::Man, 3, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Man, 5, false),
            Tile::new_number(Suit::Man, 6, false),
            Tile::new_number(Suit::Man, 7, false),
            Tile::new_number(Suit::Man, 8, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Haku),
        ] {
            hand.add_tile(t);
        }
        // 副露 (add_meld は meld.tiles を hand.tiles から削除しようとするが、
        // tiles に 2m を入れていないので no-op になる)
        hand.add_meld(open_pon(Tile::new_number(Suit::Man, 2, false)));

        let win = Tile::new_honor(Honor::Haku);
        let result = ScoringEngine::calculate_score(&hand, &win, false, false);
        assert!(result.is_some(), "副露あり混一色は scoring 成立");
        let r = result.unwrap();
        assert!(
            r.yaku.contains(&Yaku::Honitsu),
            "Honitsu が検出されるべき: {:?}",
            r.yaku
        );
    }

    /// 副露あり清一色:
    /// ポン 2p2p2p + 残り手牌: 3p 4p 5p 6p 7p 8p 9p 9p 9p 5p, 和了: 5p (シャンポン)
    /// 構成: [2p2p2p] + 3p4p5p + 5p5p? — 重複面倒。
    /// 整理: ポン 1p1p1p + 残り: 2p3p4p 5p6p7p 8p8p8p 9p, 和了: 9p (単騎)
    /// 構成: [1p1p1p] + 2p3p4p + 5p6p7p + 8p8p8p + 9p9p (雀頭)
    #[test]
    fn test_chinitsu_with_pon() {
        let mut hand = Hand::new();
        for t in [
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 6, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Pin, 8, false),
            Tile::new_number(Suit::Pin, 8, false),
            Tile::new_number(Suit::Pin, 8, false),
            Tile::new_number(Suit::Pin, 9, false),
        ] {
            hand.add_tile(t);
        }
        hand.add_meld(open_pon(Tile::new_number(Suit::Pin, 1, false)));

        let win = Tile::new_number(Suit::Pin, 9, false);
        let result = ScoringEngine::calculate_score(&hand, &win, false, false);
        assert!(result.is_some(), "副露あり清一色は scoring 成立");
        let r = result.unwrap();
        assert!(
            r.yaku.contains(&Yaku::Chinitsu),
            "Chinitsu が検出されるべき: {:?}",
            r.yaku
        );
    }

    /// 喰いタン:
    /// チー 3m4m5m + 残り手牌: 2p 2p 3p 4p 5p 6p 7p 8p 7s 8s, 和了: 6s
    /// 構成: [3m4m5m] + 2p2p + 3p4p5p + 6p7p8p + 6s7s8s
    /// 全部 2-8 のみ → タンヤオ成立
    #[test]
    fn test_tanyao_with_chi() {
        let mut hand = Hand::new();
        for t in [
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 2, false),
            Tile::new_number(Suit::Pin, 3, false),
            Tile::new_number(Suit::Pin, 4, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 6, false),
            Tile::new_number(Suit::Pin, 7, false),
            Tile::new_number(Suit::Pin, 8, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_number(Suit::Sou, 8, false),
        ] {
            hand.add_tile(t);
        }
        hand.add_meld(open_chi(Suit::Man, 3));

        let win = Tile::new_number(Suit::Sou, 6, false);
        let result = ScoringEngine::calculate_score(&hand, &win, false, false);
        assert!(result.is_some(), "喰いタンは scoring 成立");
        let r = result.unwrap();
        assert!(
            r.yaku.contains(&Yaku::Tanyao),
            "Tanyao が検出されるべき: {:?}",
            r.yaku
        );
    }

    /// 副露ありトイトイ:
    /// ポン 2m2m2m + ポン 5p5p5p + 残り手牌: 7s 7s 7s 東 東 東 西, 和了: 西 (単騎雀頭)
    /// 構成: [2m2m2m] + [5p5p5p] + 7s7s7s + 東東東 + 西西 (雀頭)
    /// 全面子刻子 → トイトイ
    #[test]
    fn test_toitoi_with_pon() {
        let mut hand = Hand::new();
        for t in [
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_number(Suit::Sou, 7, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Shaa),
        ] {
            hand.add_tile(t);
        }
        hand.add_meld(open_pon(Tile::new_number(Suit::Man, 2, false)));
        hand.add_meld(open_pon(Tile::new_number(Suit::Pin, 5, false)));

        let win = Tile::new_honor(Honor::Shaa);
        let result = ScoringEngine::calculate_score(&hand, &win, false, false);
        assert!(result.is_some(), "副露ありトイトイは scoring 成立");
        let r = result.unwrap();
        assert!(
            r.yaku.contains(&Yaku::Toitoi),
            "Toitoi が検出されるべき: {:?}",
            r.yaku
        );
    }

    /// Q10: 大三元の副露成立
    /// ポン 白白白 + ポン 發發發 + ポン 中中中 + 残り手牌: 2m 3m 4m 5p 5p, 和了: 5p
    /// 構成: [白白白] + [發發發] + [中中中] + 2m3m4m + 5p5p (雀頭)
    #[test]
    fn test_daisangen_with_pon() {
        let mut hand = Hand::new();
        for t in [
            Tile::new_number(Suit::Man, 2, false),
            Tile::new_number(Suit::Man, 3, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 5, false),
        ] {
            hand.add_tile(t);
        }
        hand.add_meld(open_pon(Tile::new_honor(Honor::Haku)));
        hand.add_meld(open_pon(Tile::new_honor(Honor::Hatsu)));
        hand.add_meld(open_pon(Tile::new_honor(Honor::Chun)));

        let win = Tile::new_number(Suit::Pin, 5, false);
        let result = ScoringEngine::calculate_score(&hand, &win, false, false);
        assert!(result.is_some(), "大三元 (副露) は scoring 成立");
        let r = result.unwrap();
        assert!(
            r.yaku.contains(&Yaku::Daisangen),
            "Daisangen が検出されるべき: {:?}",
            r.yaku
        );
    }
}