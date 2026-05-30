use crate::player::Player;
use crate::realtime::{self, PlayerTimer};
use crate::scoring::Yaku;
use crate::tile::{Tile, TileType, Suit, Honor};
use rand::seq::SliceRandom;
use rand::thread_rng;
use std::collections::{HashMap, HashSet};

/// ゲームモード
///
/// - `Standard`: 通常ルール
/// - `Seikyo`: 誠京麻雀（『天』『アカギ』の裏ルール）。場代・二度ヅモ・役満祝儀
/// - `Washizu`: 鷲巣麻雀（『アカギ』）。全牌の 3/4 が透明で他家からも見える
/// - `FiveTile`: 5枚麻雀（クライマックスだけ麻雀）。手牌 5 枚（親 6 枚）スタート、
///   雀頭+面子1組で和了、タンヤオのみ判定
/// - `EastWest`: 東西戦（クリア麻雀、『天』のチーム戦ルール）。東家+西家＝東チーム、
///   南家+北家＝西チーム。指定二翻役5種を先にチームとして全て揃えた方の勝利。
/// - `Yamima`: 闇麻。プレイヤーは点棒 1000 点を支払って打牌を**裏向き（闇牌）**で
///   河に置ける。他家からは「闇牌」（種類非公開）として見え、鳴き・ロンの対象に
///   できない。ターンプレイヤーは点棒 500 点を支払って「照射」を宣言することで
///   他家の闇牌を公開させられる。
/// - `RealTime`: リアルタイム麻雀。ターン制を廃止、全員が独立タイマー（5 秒）で
///   ツモ→打牌を繰り返す。タイムアウトで自動ツモ切り。鳴き宣言は早い者勝ちで
///   優先順位は Ron > Pon > Kan > Chi。本実装は Rust core のロジック層のみで、
///   完全な同時打牌入力ループは CLI 同期版の範疇外（web/wasm follow-up）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    Standard,
    Seikyo,
    Washizu,
    FiveTile,
    EastWest,
    Yamima,
    RealTime,
}

/// 東西戦（クリア麻雀）のチーム。
///
/// 麻雀の座席名（東家 = ton, 南家 = nan, 西家 = shaa, 北家 = pei）と
/// チーム名（East / West）は別概念であることに注意。
/// 座席 0 (東家) + 座席 2 (西家) → East チーム、
/// 座席 1 (南家) + 座席 3 (北家) → West チーム。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Team {
    East,
    West,
}

/// 座席 index からチームを返すヘルパー。
///
/// - 座席 0 (東家) と 2 (西家) → `Team::East`
/// - 座席 1 (南家) と 3 (北家) → `Team::West`
pub fn team_of(seat_idx: usize) -> Team {
    match seat_idx {
        0 | 2 => Team::East,
        _ => Team::West,
    }
}

/// 東西戦のクリア対象役5種（指定二翻役）。
///
/// 三色同順 / 一気通貫 / 対々和 / 全帯么 / 混老頭
pub fn east_west_target_yaku() -> [Yaku; 5] {
    [
        Yaku::SanshokuDoujun,
        Yaku::Ittsu,
        Yaku::Toitoi,
        Yaku::Chanta,
        Yaku::Honroutou,
    ]
}

/// 誠京麻雀の固定額
pub const SEIKYO_SEAT_FEE: i32 = 1000;
pub const SEIKYO_YAKUMAN_TIP: i32 = 8000;

/// 本場あたりのボーナス点（和了者が受け取る／放銃者または全員が支払う）。
///
/// 分担:
/// - **ロン**: 放銃者から `HONBA_BONUS * honba` を全額徴収
/// - **ツモ**: 他家 3 人から `HONBA_BONUS * honba / 3` ずつ均等徴収
///
/// **ツモ時 3 等分で割り切れる前提のため、3 の倍数で固定**。
/// 100 点単位への切り上げは `apply_payment` 内の `ceil_to_hundred` で吸収する
/// （例: `300 / 3 = 100` のような端数の出ないケースに加え、和了点側の端数も含めて
/// 各支払者の合計を 100 点単位に揃える）。
pub const HONBA_BONUS: i32 = 300;

/// 対局の長さ。東風戦 = 4 局、半荘戦 = 8 局。
///
/// `Default` は最も一般的な半荘戦 (`Hanchan`)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Length {
    /// 東風戦（東 1 局〜東 4 局）
    Tonpuusen,
    /// 半荘戦（東 1 局〜南 4 局）
    #[default]
    Hanchan,
}

/// #61 本場連動の縛り（最低点数縛り）ルール。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShibariRule {
    /// 1 飜縛り（常時、= 役ありで和了可）。標準。
    #[default]
    Standard,
    /// 5 本場以降は役 2 飜以上が必要（ドラは縛りに数えない）。
    TwoHanFromFiveHonba,
    /// 5 本場以降は満貫以上が必要。
    ManganFromFiveHonba,
    /// 7 本場以降は役満のみ和了可（ローカル極北）。
    YakumanFromSevenHonba,
}

/// #57 包（責任払い）の責任関係。
/// 大三元 / 大四喜 / 四槓子 を「他家の打牌から鳴かせて確定させた」打牌者が、
/// その役満で和了されたとき総得点を負担する。
#[derive(Debug, Clone)]
pub struct PaoLiability {
    /// 役満を確定させたプレイヤー（= 和了予定者）。
    pub beneficiary: usize,
    /// 確定打牌をした責任者（鳴かれた牌の打牌者）。
    pub responsible: usize,
    /// 対象役満。
    pub yaku: Yaku,
}

/// 和了の種類。
#[derive(Debug, Clone, Copy)]
pub enum WinKind {
    /// 自摸和了
    Tsumo,
    /// ロン和了。`from` は放銃者の player_idx。
    Ron { from: usize },
}

/// 1 局の結着結果。`Game::last_outcome` に保持し、UI 側はこれを読んで「和了画面」
/// 「流局画面」を分岐する。`next_round` で None にクリアされる。
#[derive(Debug, Clone)]
pub enum RoundOutcome {
    /// 和了で終了。
    Win {
        /// 和了者の座席 index
        winner: usize,
        /// ツモ or ロン
        kind: WinKind,
        /// `scoring::ScoringEngine::calculate_score` の結果
        result: crate::scoring::ScoringResult,
    },
    /// 流局で終了。聴牌者の座席 index 一覧。
    Draw {
        tenpai_players: Vec<usize>,
    },
    /// #55 特殊（途中）流局。親はそのまま連荘、聴牌料は発生しない。
    AbortiveDraw {
        kind: AbortiveDrawKind,
    },
}

/// #55 特殊（途中）流局の種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbortiveDrawKind {
    /// 四風連打: 第一巡で全員が同じ風牌を打牌。
    SuufonRenda,
    /// 四家立直: 4 人全員が立直宣言。
    SuuchaRiichi,
    /// 四槓散了: 4 つの槓が複数人で打たれた（1 人 4 槓は継続）。
    SuukanSanra,
    /// 九種九牌: 第一ツモ・無鳴きで么九牌 9 種以上を宣言。
    KyuushuKyuuhai,
    /// 三家和: 同一打牌に 3 人が同時ロン。
    SanchaaHou,
}

/// 闇麻の固定額
/// `Vec<Yaku>` に含まれる役満の個数を数える (倍役満は 2 として扱う等の拡張は将来)。
/// 現状は単役満 = 1、複数の役満が同時成立した場合はその個数を返す。
pub fn count_yakuman(yaku: &[Yaku]) -> u32 {
    yaku.iter()
        .filter(|y| {
            matches!(
                y,
                Yaku::Kokushi
                    | Yaku::Suuankou
                    | Yaku::Daisangen
                    | Yaku::Tsuuiisou
                    | Yaku::Shousuushii
                    | Yaku::Daisuushii
                    | Yaku::Ryuuiisou
                    | Yaku::Chinroutou
                    | Yaku::Chuuren
                    | Yaku::Suukantsu
                    | Yaku::Tenhou
                    | Yaku::Chiihou
            )
        })
        .count() as u32
}

pub const YAMIMA_HIDDEN_COST: i32 = 1000;
pub const YAMIMA_LIGHT_UP_COST: i32 = 500;

#[derive(Debug, Clone)]
pub struct Game {
    pub players: Vec<Player>,
    pub wall: Vec<Tile>,
    pub dora_indicators: Vec<Tile>,
    pub current_player: usize,
    pub round: u32,
    pub dealer: usize,
    pub last_discard: Option<Tile>,
    /// 直近の打牌が闇牌（Yamima ルール）かどうか。
    ///
    /// `discard_hidden_tile` で打牌したときに true、通常 `discard_tile` で false に
    /// 戻る。`can_someone_win` / `can_pon` / `can_chi` / `can_kan` はこのフラグが
    /// true のとき**全てロン・鳴き不可**として扱う（闇牌は鳴けない仕様）。
    /// 照射（`light_up`）で実際に公開された後でも、`last_discard_hidden` は
    /// 「最後の打牌時点での裏/表」を表すフィールドであり、過去ターンの照射成功で
    /// 鳴きが復活する設計ではない（鳴きは原則直後 1 ターン以内）。
    pub last_discard_hidden: bool,
    /// ゲームモード（Standard / Seikyo / Washizu / FiveTile / EastWest / Yamima）
    pub mode: GameMode,
    /// 供託（誠京麻雀の場代合計）。和了者が回収・流局で持ち越し
    pub pot: i32,
    /// 前局で親が和了したか（= 連荘フラグ。二度ヅモ判定に使う）。
    ///
    /// `resolve_win` / `resolve_draw` で自動的に更新される（親和了・親テンパイで true、
    /// それ以外で false）。`next_round` 内で連荘判定（dealer 据え置き + honba +1）に
    /// 使われる。
    pub dealer_won_last: bool,
    /// 対局の長さ（東風戦 / 半荘戦）。is_game_over の判定に使う。
    pub length: Length,
    /// 本場（連荘・流局でインクリメントされる）。和了者に `HONBA_BONUS * honba` が乗る。
    pub honba: u32,
    /// 供託リーチ棒の本数。`riichi_sticks * 1000` 点が次の和了者に渡る。
    /// 流局では持ち越し。
    pub riichi_sticks: u32,
    /// 直前局の結果。UI 側で読み、`next_round` で None にクリアされる。
    pub last_outcome: Option<RoundOutcome>,
    /// 対局終了フラグ。`next_round` が false を返したときに true になる。
    pub game_over: bool,
    /// 東西戦（クリア麻雀）のチーム別役クリア進捗。
    ///
    /// `GameMode::EastWest` のときのみ実質的に使用する。
    /// `record_team_yaku` で和了者のチームに役を追加していき、
    /// `east_west_target_yaku()` 全 5 種が揃ったチームが勝利。
    /// EastWest 以外のモードでは初期化はされるが書き込まれない（no-op）。
    pub team_progress: HashMap<Team, HashSet<Yaku>>,
    /// リアルタイム麻雀のプレイヤー別タイマー（4 つ、各 5000ms 制限）。
    ///
    /// `GameMode::RealTime` のときのみ実質的に意味を持つが、コストが極小なので
    /// 全モードで `PlayerTimer::default_limit()` 初期化する。
    /// 進行は呼び出し側が `tick_timers(delta_ms)` を周期的に呼ぶ。
    pub player_timers: Vec<PlayerTimer>,

    // ============================================================
    // #50 状況役の状態追跡
    // ============================================================
    /// 直前の draw が wall 最終枚だったか (Haitei 候補)。
    ///
    /// `draw_tile` 内で `wall.is_empty()` (= 引いたら最後の 1 枚だった) を見て更新する。
    /// 次の draw / 鳴き / 局頭で false に戻る。
    pub is_last_draw: bool,
    /// 直前打牌が wall 空時の打牌だったか (Houtei 候補)。
    ///
    /// `discard_tile` 系で wall が空のときに true、次の draw / 局頭で false に戻る。
    pub is_last_discard: bool,
    /// 直前の draw が do_kan / do_ankan による嶺上ツモだったか (Rinshan 候補)。
    ///
    /// `do_kan` / `do_ankan` 完了直後に true、その後の draw / discard / 局頭で false に戻る。
    pub last_was_rinshan: bool,
    /// 加槓宣言中の牌 (Chankan 候補)。
    ///
    /// 加槓自体は #46 で未実装。現状はフィールドだけ保持し、加槓実装と同時に発火させる。
    pub pending_chankan: Option<Tile>,
    /// 直前打牌者の座席 index。
    ///
    /// 立直成立後の自家ツモタイミングで一発フラグを下ろすため、`current_player + 3 % 4`
    /// 依存を回避する。鳴きで `current_player` が任意席に飛ぶケースにも対応。
    pub last_discarder: Option<usize>,
    /// #51: 当該局でこれまでに任意の鳴き (Chi/Pon/Kan/Ankan/Shouminkan) が
    /// 1 度でも入ったか。地和は「子の第一ツモ + 鳴き未発生」が条件のため、
    /// このフラグが true なら地和は不成立になる。次局頭でリセットされる。
    pub any_call_made_this_round: bool,
    /// #51: 当該局における山牌からの draw 回数 (嶺上ツモは含まない)。
    /// 天和判定で「親がまだツモっていない (= 配牌時 14 枚のまま) 」を確認するため、
    /// および地和判定で「鳴きなしで自分の第 1 ツモ」を確認するために使う。
    /// 親=0, 子1=1, 子2=2, 子3=3 が各プレイヤーの「第 1 ツモ」相当のインデックス。
    pub draws_this_round: u32,
    /// #89: 嘘リーチ（黙聴での虚偽リーチ）を許可するかどうか。
    /// true のとき `can_riichi` のテンパイ・点数要件を外し、門前 + 未リーチのみで
    /// リーチ宣言可能にする。流局時に非テンパイのリーチ者へ罰符を課す。
    pub uso_riichi_enabled: bool,
    /// #80: ツモ後に手牌を自動ソート（理牌）するかどうか。
    /// true（デフォルト）のとき draw_tile 後に自動で sort_hand() を呼ぶ。
    /// false のとき手牌はツモ順のまま保持される。
    pub auto_sort: bool,
    /// #81: 人間プレイヤーのターン開始時に自動でツモを行うかどうか。
    /// true（デフォルト）のとき TS 側 shouldDrawHumanTile で自動ツモが走る。
    /// false のとき手動ツモが必要（T キーまたは山牌タップ）。
    pub auto_draw: bool,
    /// #59 食い替え禁止を強制するか（デフォルト true）。
    /// false にすると食い替え打牌を許可する（ローカルルール用 toggle）。
    pub enforce_kuikae: bool,
    /// #129 喰いタン (鳴きタンヤオ) を認めるか（デフォルト true）。
    /// false にすると非門前の断么九を無効化する（ローカルルール用 toggle）。
    pub allow_open_tanyao: bool,
    /// #59 直前のチー / ポンによって「次の打牌で切れない牌」の集合。
    /// 現物（鳴いた牌と同種）と筋（チーのリャンメン反対側）を入れる。
    /// 打牌が成立した時点でクリアする。チー / ポン以外の操作では空のまま。
    pub kuikae_forbidden: Vec<Tile>,
    /// #58 ローカル役満 (人和/大車輪/四連刻/百万石/三連刻) を認めるか（デフォルト false）。
    pub allow_local_yakuman: bool,
    /// #61 本場縛りルール（デフォルト Standard = 1 飜縛り）。
    pub shibari_rule: ShibariRule,
    /// #57 包（責任払い）を適用するか（デフォルト true、標準ルール）。
    pub enforce_pao: bool,
    /// #57 当該局で発生した包の責任関係。鳴きで役満が確定するたびに push、
    /// 局リセットでクリアする。
    pub pao_liabilities: Vec<PaoLiability>,
    /// #118 割れ目プレイヤー（None なら割れ目ルール無効）。
    /// このプレイヤーが絡む支払い（払う / 受け取る）は 2 倍になる。
    pub warime_player: Option<usize>,
    /// #55 特殊（途中）流局を有効にするか（デフォルト true、概ね標準ルール）。
    pub allow_abortive_draws: bool,
    /// #55 流し満貫判定用: 各プレイヤーの打牌が他家に鳴かれたか。
    /// 鳴かれていたら流し満貫不成立。局リセットで全 false。
    pub discard_taken_from: Vec<bool>,
}

impl Game {
    pub fn new(player_names: Vec<String>) -> Self {
        Self::new_with_mode(player_names, GameMode::Standard)
    }

    /// モードを指定してゲームを構築（長さは半荘デフォルト）
    pub fn new_with_mode(player_names: Vec<String>, mode: GameMode) -> Self {
        Self::new_with_mode_and_length(player_names, mode, Length::Hanchan)
    }

    /// モードと長さを指定してゲームを構築
    pub fn new_with_mode_and_length(
        player_names: Vec<String>,
        mode: GameMode,
        length: Length,
    ) -> Self {
        assert!(player_names.len() == 4, "Mahjong requires exactly 4 players");

        let players: Vec<Player> = player_names
            .into_iter()
            .enumerate()
            .map(|(i, name)| {
                let mut player = Player::new(i, name);
                if i == 0 {
                    player.is_dealer = true;
                }
                player
            })
            .collect();

        let mut team_progress = HashMap::new();
        team_progress.insert(Team::East, HashSet::new());
        team_progress.insert(Team::West, HashSet::new());

        // 4 人ぶんの PlayerTimer をデフォルト 5000ms 制限で初期化。
        // RealTime 以外のモードでも持つが書き込み・読み込みされないため実害なし。
        let player_timers = (0..4).map(|_| PlayerTimer::default_limit()).collect();

        let mut game = Self {
            players,
            wall: Vec::new(),
            dora_indicators: Vec::new(),
            current_player: 0,
            round: 1,
            dealer: 0,
            last_discard: None,
            last_discard_hidden: false,
            mode,
            pot: 0,
            dealer_won_last: false,
            length,
            honba: 0,
            riichi_sticks: 0,
            last_outcome: None,
            game_over: false,
            team_progress,
            player_timers,
            is_last_draw: false,
            is_last_discard: false,
            last_was_rinshan: false,
            pending_chankan: None,
            last_discarder: None,
            any_call_made_this_round: false,
            draws_this_round: 0,
            uso_riichi_enabled: false,
            auto_sort: true,
            auto_draw: true,
            enforce_kuikae: true,
            kuikae_forbidden: Vec::new(),
            allow_open_tanyao: true,
            allow_local_yakuman: false,
            shibari_rule: ShibariRule::Standard,
            enforce_pao: true,
            pao_liabilities: Vec::new(),
            warime_player: None,
            allow_abortive_draws: true,
            discard_taken_from: vec![false; 4],
        };

        game.initialize_wall();
        game.deal_initial_tiles();
        game
    }

    /// 誠京麻雀の場代を全員から徴収して供託に積む。
    /// Standard モードでは no-op。
    ///
    /// # Arguments
    /// - `amount`: 1 人あたりの場代額。標準は [`SEIKYO_SEAT_FEE`]（1000 点）。
    ///   テストや特殊バリアントで上書きできるよう引数化している。
    ///
    /// # 仕様メモ
    /// 「**各局開始時**」に呼ばれる想定。`Game::next_round` 内で Seikyo モードのときに
    /// 自動的に呼ばれる（次局突入のたびに 4 人 × `SEIKYO_SEAT_FEE` が pot に積まれる）。
    /// 初局ぶんはコンストラクタ後に呼び出し側が一度呼ぶこと。
    /// 複数回呼べば素直に pot が累積する設計（流局持ち越しと同じ挙動）。
    pub fn collect_seat_fee(&mut self, amount: i32) {
        if self.mode != GameMode::Seikyo {
            return;
        }
        for player in self.players.iter_mut() {
            player.subtract_score(amount);
        }
        self.pot += amount * self.players.len() as i32;
    }

    /// 供託（pot）を winner に渡してリセット。移動した点数を返す。
    pub fn winner_takes_pot(&mut self, winner_idx: usize) -> i32 {
        if winner_idx >= self.players.len() {
            return 0;
        }
        let moved = self.pot;
        if moved > 0 {
            self.players[winner_idx].add_score(moved);
            self.pot = 0;
        }
        moved
    }

    /// 親の二度ヅモ。**2 枚ツモするだけ。打牌は呼び出し側の責務**。
    ///
    /// 誠京麻雀かつ前局親和了（連荘）かつ現在のプレイヤーが親のときのみ、
    /// 山牌から 2 枚連続でツモして親の手牌に追加する。
    /// 戻り値は (1 枚目, 2 枚目) のタプル。山牌が 2 枚未満なら None を返す。
    ///
    /// **注意**: この関数を呼んだ直後、親の手牌は 15 枚（通常 13 + 2 ツモ、
    /// または親初期 14 + 1 ツモから 2 枚追加）になる。和了判定 `tile_count() == 14`
    /// を維持するには、呼び出し側で 1 枚を即捨てる UX を実装する必要がある。
    /// `main.rs` の `handle_player_turn` がそのリファレンス実装。
    pub fn dealer_double_draw(&mut self) -> Option<(Tile, Tile)> {
        if self.mode != GameMode::Seikyo {
            return None;
        }
        if !self.dealer_won_last {
            return None;
        }
        if self.current_player != self.dealer {
            return None;
        }
        if self.wall.len() < 2 {
            return None;
        }
        let first = self.wall.pop()?;
        let second = self.wall.pop()?;
        self.draw_tile_to(self.current_player, first);
        self.draw_tile_to(self.current_player, second);
        Some((first, second))
    }

    fn initialize_wall(&mut self) {
        self.wall.clear();
        
        // 数牌（各4枚）
        for suit in [Suit::Man, Suit::Pin, Suit::Sou] {
            for value in 1..=9 {
                for _ in 0..4 {
                    let is_red = value == 5 && self.wall.iter().filter(|t| 
                        matches!(t.tile_type, TileType::Number { suit: s, value: 5 } if s == suit) && t.is_red
                    ).count() == 0; // 各色5の1枚目のみ赤ドラ
                    
                    self.wall.push(Tile::new_number(suit, value, is_red));
                }
            }
        }

        // 字牌（各4枚）
        for honor in [Honor::Ton, Honor::Nan, Honor::Shaa, Honor::Pei, Honor::Haku, Honor::Hatsu, Honor::Chun] {
            for _ in 0..4 {
                self.wall.push(Tile::new_honor(honor));
            }
        }

        // シャッフル
        self.wall.shuffle(&mut thread_rng());

        // 鷲巣麻雀: 全 136 牌のうちランダムに 3/4 を glass にする
        // 山牌のシャッフル後に 0..(len * 3 / 4) を glass マーク（既にシャッフル済みなので
        // インデックス先頭から塗ってもランダム分布になる）
        if self.mode == GameMode::Washizu {
            let total = self.wall.len();
            let glass_count = total * 3 / 4;
            for i in 0..glass_count {
                self.wall[i].is_glass = true;
            }
        }

        // ドラ表示牌を設定
        if let Some(dora_indicator) = self.wall.pop() {
            self.dora_indicators.push(dora_indicator);
        }
    }

    /// 観測者 (`observer_idx`) から見た対象 (`target_idx`) の手牌のうち、
    /// 視認可能な牌のリストを返す。
    ///
    /// - 自分自身 (`observer_idx == target_idx`) なら手牌すべて
    /// - 他家なら鷲巣ルール: `is_glass == true` の牌のみ
    /// - 非鷲巣モードでも、他家の手牌は通常見えないため空ベクタを返す
    ///   （UI 側は別経路で「自分の手牌のみ表示」しているのでこの API は鷲巣専用想定）
    ///
    /// 副槓子（meld）は鳴いた時点で公開されているので別 API で参照する想定。
    /// ここでは concealed hand の見え方だけ扱う。
    pub fn get_visible_tiles_of_opponent(
        &self,
        observer_idx: usize,
        target_idx: usize,
    ) -> Vec<Tile> {
        if observer_idx >= self.players.len() || target_idx >= self.players.len() {
            return Vec::new();
        }
        let target_hand = self.players[target_idx].hand.get_tiles();
        if observer_idx == target_idx {
            return target_hand.clone();
        }
        match self.mode {
            GameMode::Washizu => target_hand
                .iter()
                .filter(|t| t.is_glass)
                .cloned()
                .collect(),
            // 他モードでは他家の手牌は不可視（空）
            _ => Vec::new(),
        }
    }

    fn deal_initial_tiles(&mut self) {
        // FiveTile モード: 子 5 枚、親 6 枚（「ツモ番が回った」状態でスタート）
        if self.mode == GameMode::FiveTile {
            for player_idx in 0..4 {
                for _ in 0..5 {
                    if let Some(tile) = self.wall.pop() {
                        self.draw_tile_to(player_idx, tile);
                    }
                }
            }
            // 親に追加の 1 枚
            if let Some(tile) = self.wall.pop() {
                self.draw_tile_to(self.dealer, tile);
            }
            return;
        }

        // Standard / Seikyo / Washizu: 親は14枚、子は13枚配る
        for _round in 0..3 {
            for player_idx in 0..4 {
                for _ in 0..4 {
                    if let Some(tile) = self.wall.pop() {
                        self.draw_tile_to(player_idx, tile);
                    }
                }
            }
        }

        // 最後の1枚ずつ
        for player_idx in 0..4 {
            if let Some(tile) = self.wall.pop() {
                self.draw_tile_to(player_idx, tile);
            }
        }

        // 親に追加の1枚
        if let Some(tile) = self.wall.pop() {
            self.draw_tile_to(self.dealer, tile);
        }
    }

    /// #80: プレイヤーに牌をツモらせる。auto_sort=true なら直後に sort_hand() を呼ぶ。
    /// game.rs 内で `player.draw_tile(tile)` を呼ぶ代わりにこれを使う。
    fn draw_tile_to(&mut self, player_idx: usize, tile: Tile) {
        self.players[player_idx].draw_tile(tile);
        if self.auto_sort {
            self.players[player_idx].hand.sort_hand();
        }
    }

    pub fn draw_tile(&mut self) -> Option<Tile> {
        let tile = self.wall.pop();
        if tile.is_some() {
            // #50 海底候補: 引いた直後に wall が空なら is_last_draw=true
            self.is_last_draw = self.wall.is_empty();
            // 通常の draw が走ったので、嶺上 / 河底 / 加槓フラグは下ろす
            self.last_was_rinshan = false;
            self.is_last_discard = false;
            // #51 山牌からのツモ回数をインクリメント (嶺上ツモは含めない経路)
            self.draws_this_round = self.draws_this_round.saturating_add(1);
        }
        tile
    }

    pub fn current_player_draw(&mut self) -> bool {
        if let Some(tile) = self.draw_tile() {
            self.draw_tile_to(self.current_player, tile);
            true
        } else {
            false
        }
    }

    /// 通常打牌。`current_player` が `tile` を捨て、河に追加して次の手番に進む。
    ///
    /// - 手牌に `tile` が無ければ false（state は変化しない）
    /// - 成功時: 河に追加 + `last_discard` 更新 + `last_discard_hidden=false` + `next_player()`
    ///
    /// **タイマーセマンティクス**: 本関数は `current_player` のタイマー
    /// （`player_timers[current_player]`）には触れない。`GameMode::RealTime` で運用する
    /// ときは、打牌成功後に呼び出し側が別途
    /// [`Game::reset_player_timer`]`(打牌したプレイヤー idx)` を呼んで経過時間を 0 に
    /// 戻すこと。`Standard` 等のターン制モードではタイマーが実質未使用なので呼ばなく
    /// てもよい。
    pub fn discard_tile(&mut self, tile: Tile) -> bool {
        // #59 食い替え禁止: 鳴いた直後の打牌で現物 / 筋を切るのを拒否する。
        // tile_type のみ比較する (赤ドラの is_red 違いも同種とみなす)。
        if self.enforce_kuikae
            && self
                .kuikae_forbidden
                .iter()
                .any(|f| f.tile_type == tile.tile_type)
        {
            return false;
        }
        let discarder = self.current_player;
        if self.players[discarder].discard_tile(tile) {
            // 打牌が成立したので食い替え禁止牌をクリアする。
            self.kuikae_forbidden.clear();
            self.last_discard = Some(tile);
            self.last_discard_hidden = false;
            self.last_discarder = Some(discarder);
            // #50 河底候補: wall が空のときの打牌
            self.is_last_discard = self.wall.is_empty();
            // 嶺上開花フラグは打牌で確実に下ろす
            self.last_was_rinshan = false;
            // #49 一発: 打牌者自身の一発フラグは「打牌前のツモまで」が範囲。
            // 既に declare_riichi 時の自家ツモ → 即打牌ならフラグは ippatsu=true のまま。
            // ただし他家からの鳴きが入ると別経路 (do_chi/do_pon/do_kan) でクリアする。
            // 自家打牌した時点で自身の一発は次のツモ機会まで残らないため、
            // 次に自家ツモが回ってきた時点でクリアする運用とする (現状の declare 後 1 巡内ロン用)。
            self.next_player();
            true
        } else {
            false
        }
    }

    /// 闇牌打牌（Yamima ルール）。`Player::discard_hidden` を呼んで河に裏向きで置く。
    ///
    /// - `GameMode::Yamima` でないと no-op で false
    /// - 現プレイヤーの点数が 1000 未満なら false（`Player::discard_hidden` 内でも検査）
    /// - 手牌に `tile` が無ければ false
    /// - 成功時: 1000 点減 + 河に闇牌追加 + `last_discard_hidden=true` + 次の手番へ進む
    ///
    /// `last_discard` には実体牌をセットするが、`last_discard_hidden` が true の間は
    /// `can_someone_win` / `can_pon` / `can_chi` / `can_kan` が全て false を返す（鳴き不可）。
    /// 照射で公開してもこのフラグは戻らない（鳴きの再開ではなく河の可視化のみ）。
    pub fn discard_hidden_tile(&mut self, tile: Tile) -> bool {
        if self.mode != GameMode::Yamima {
            return false;
        }
        let discarder = self.current_player;
        if !self.players[discarder].discard_hidden(tile) {
            return false;
        }
        self.last_discard = Some(tile);
        self.last_discard_hidden = true;
        self.last_discarder = Some(discarder);
        self.is_last_discard = self.wall.is_empty();
        self.last_was_rinshan = false;
        // #59 闇牌打牌でも食い替え禁止牌はクリアする (鳴き直後の制約は 1 打のみ)
        self.kuikae_forbidden.clear();
        self.next_player();
        true
    }

    /// 照射（Yamima ルール）。観測者が 500 点支払って対象の闇牌を公開させる。
    ///
    /// - `GameMode::Yamima` でないと None
    /// - `observer_idx` / `target_idx` の範囲外なら None
    /// - 観測者の点数が 500 未満なら None
    /// - 対象 discard が既に公開済（is_hidden==false）なら None
    /// - 成功時: 観測者から 500 点減 + 対象の `is_hidden=false` に書き換え + 実体牌を返す
    ///
    /// 仕様メモ: 「照射するだけで必ず公開される」運用。空振り（外れ）はなく、
    /// 失敗するのは「点数不足」「対象 index 不正」「既に公開済」のみ。
    pub fn light_up(
        &mut self,
        observer_idx: usize,
        target_idx: usize,
        discard_idx: usize,
    ) -> Option<Tile> {
        if self.mode != GameMode::Yamima {
            return None;
        }
        if observer_idx >= self.players.len() || target_idx >= self.players.len() {
            return None;
        }
        // 自分で自分の闇牌を照射するのは無効（点棒だけ焼く操作になる）
        if observer_idx == target_idx {
            return None;
        }
        if self.players[observer_idx].score < YAMIMA_LIGHT_UP_COST {
            return None;
        }
        // 先に公開を試みる（失敗したら点数は引かない）
        let revealed = self.players[target_idx].reveal_discard(discard_idx)?;
        self.players[observer_idx].subtract_score(YAMIMA_LIGHT_UP_COST);
        Some(revealed)
    }

    pub fn next_player(&mut self) {
        self.current_player = (self.current_player + 1) % 4;
    }

    pub fn get_current_player(&self) -> &Player {
        &self.players[self.current_player]
    }

    pub fn get_current_player_mut(&mut self) -> &mut Player {
        &mut self.players[self.current_player]
    }

    pub fn can_someone_win(&self, tile: &Tile) -> Vec<usize> {
        // Yamima: `last_discard` が闇牌（裏向き）のときはロン不可。先に照射で公開してから
        // 改めて判定する仕様。引数 `tile` は呼び出し側が `last_discard` と同じ値を渡す前提で
        // 利用するため、`last_discard_hidden` のみで遮断してよい。
        if self.last_discard_hidden {
            return Vec::new();
        }
        let mut winners = Vec::new();
        for (i, player) in self.players.iter().enumerate() {
            if i == self.current_player {
                continue;
            }
            let can_win = match self.mode {
                GameMode::FiveTile => player.can_win_with_mode(tile, GameMode::FiveTile),
                _ => player.can_win(tile),
            };
            if can_win {
                winners.push(i);
            }
        }
        winners
    }

    /// チー可能かチェック（下家のみ）
    pub fn can_chi(&self, player_idx: usize) -> bool {
        if self.last_discard.is_none() {
            return false;
        }
        // Yamima: 闇牌はチー不可
        if self.last_discard_hidden {
            return false;
        }

        // チーは下家（前のプレイヤー）のみ可能
        let prev_player = (self.current_player + 3) % 4;
        if player_idx != prev_player {
            return false;
        }

        let tile = self.last_discard.unwrap();

        // 数牌のみチー可能
        if let TileType::Number { suit, value } = tile.tile_type {
            let hand = &self.players[player_idx].hand;
            let tiles = hand.get_tiles();

            // パターン1: n-2, n-1, n（鳴き牌がn）
            if value >= 3 {
                let t1 = Tile::new_number(suit, value - 2, false);
                let t2 = Tile::new_number(suit, value - 1, false);
                if tiles.contains(&t1) && tiles.contains(&t2) {
                    return true;
                }
            }

            // パターン2: n-1, n, n+1（鳴き牌がn）
            if value >= 2 && value <= 8 {
                let t1 = Tile::new_number(suit, value - 1, false);
                let t2 = Tile::new_number(suit, value + 1, false);
                if tiles.contains(&t1) && tiles.contains(&t2) {
                    return true;
                }
            }

            // パターン3: n, n+1, n+2（鳴き牌がn）
            if value <= 7 {
                let t1 = Tile::new_number(suit, value + 1, false);
                let t2 = Tile::new_number(suit, value + 2, false);
                if tiles.contains(&t1) && tiles.contains(&t2) {
                    return true;
                }
            }
        }

        false
    }

    /// ポン可能かチェック
    pub fn can_pon(&self, player_idx: usize) -> bool {
        if self.last_discard.is_none() || player_idx == self.current_player {
            return false;
        }
        // Yamima: 闇牌はポン不可
        if self.last_discard_hidden {
            return false;
        }

        let tile = self.last_discard.unwrap();
        let hand = &self.players[player_idx].hand;
        let tiles = hand.get_tiles();

        // 同じ牌が2枚以上あればポン可能
        tiles.iter().filter(|&&t| t == tile).count() >= 2
    }

    /// カン可能かチェック（明槓）
    pub fn can_kan(&self, player_idx: usize) -> bool {
        if self.last_discard.is_none() || player_idx == self.current_player {
            return false;
        }
        // Yamima: 闇牌は明槓不可
        if self.last_discard_hidden {
            return false;
        }

        let tile = self.last_discard.unwrap();
        let hand = &self.players[player_idx].hand;
        let tiles = hand.get_tiles();

        // 同じ牌が3枚あれば明槓可能
        tiles.iter().filter(|&&t| t == tile).count() >= 3
    }

    /// 暗槓可能な牌のリストを取得
    pub fn can_ankan(&self, player_idx: usize) -> Vec<Tile> {
        let hand = &self.players[player_idx].hand;
        let tiles = hand.get_tiles();
        let mut ankan_tiles = Vec::new();

        use std::collections::HashMap;
        let mut tile_counts = HashMap::new();
        for tile in tiles {
            *tile_counts.entry(*tile).or_insert(0) += 1;
        }

        for (tile, count) in tile_counts {
            if count >= 4 {
                ankan_tiles.push(tile);
            }
        }

        ankan_tiles
    }

    /// チーを実行
    pub fn do_chi(&mut self, player_idx: usize, pattern: usize) -> bool {
        if !self.can_chi(player_idx) {
            return false;
        }

        let tile = self.last_discard.unwrap();

        if let TileType::Number { suit, value } = tile.tile_type {
            // pattern ごとに「手牌から取り出す 2 枚 (t1, t2)」と、副露表示用に
            // 昇順で並べた tiles 配列 + claimed_index を組み立てる。
            // - pattern 0 (n-2, n-1, n): tiles = [t1, t2, tile]、claimed_index = 2 (右端)
            // - pattern 1 (n-1, n, n+1): tiles = [t1, tile, t2]、claimed_index = 1 (中央)
            // - pattern 2 (n, n+1, n+2): tiles = [tile, t1, t2]、claimed_index = 0 (左端)
            // いずれも tiles はチーの自然な並び (昇順)、claimed_index は last_discard の位置。
            let (t1, t2, tiles_vec, claimed_index_value) = match pattern {
                0 => {
                    // n-2, n-1, n
                    if value < 3 {
                        return false;
                    }
                    let a = Tile::new_number(suit, value - 2, false);
                    let b = Tile::new_number(suit, value - 1, false);
                    (a, b, vec![a, b, tile], 2usize)
                }
                1 => {
                    // n-1, n, n+1
                    if value < 2 || value > 8 {
                        return false;
                    }
                    let a = Tile::new_number(suit, value - 1, false);
                    let b = Tile::new_number(suit, value + 1, false);
                    (a, b, vec![a, tile, b], 1usize)
                }
                2 => {
                    // n, n+1, n+2
                    if value > 7 {
                        return false;
                    }
                    let a = Tile::new_number(suit, value + 1, false);
                    let b = Tile::new_number(suit, value + 2, false);
                    (a, b, vec![tile, a, b], 0usize)
                }
                _ => return false,
            };

            let player = &mut self.players[player_idx];
            if !player.hand.remove_tile(&t1) || !player.hand.remove_tile(&t2) {
                return false;
            }

            // #83 副露表示: 鳴き元 (= 直前打牌者) と claimed_index を保存する。
            // tiles は昇順、claimed_index は pattern から確定的に決める。
            let claimed_index = Some(claimed_index_value);
            let from_player = self.last_discarder;
            let meld = crate::hand::Meld {
                meld_type: crate::hand::MeldType::Chi,
                tiles: tiles_vec,
                is_open: true,
                from_player,
                is_kakan: false,
                claimed_index,
            };

            player.hand.push_meld(meld); // #132 二重除去回避 (t1/t2 は明示 remove 済み)
            self.last_discard = None;
            self.current_player = player_idx;
            // #49 鳴きで他家の一発を消す
            self.clear_ippatsu_others(player_idx);
            // #51 鳴き発生で地和を不成立にする
            self.any_call_made_this_round = true;
            // #55 流し満貫: 打牌が鳴かれた記録
            if let Some(f) = from_player {
                if f < self.discard_taken_from.len() {
                    self.discard_taken_from[f] = true;
                }
            }
            // #59 食い替え禁止: 現物 (鳴いた牌 value) と、リャンメンチーの筋を禁止する。
            // pattern 0 (手 value-2,value-1): 筋 = value-3
            // pattern 1 (手 value-1,value+1 嵌張): 筋なし
            // pattern 2 (手 value+1,value+2): 筋 = value+3
            let mut forbidden = vec![Tile::new_number(suit, value, false)];
            match pattern {
                0 if value >= 4 => forbidden.push(Tile::new_number(suit, value - 3, false)),
                2 if value <= 6 => forbidden.push(Tile::new_number(suit, value + 3, false)),
                _ => {}
            }
            self.kuikae_forbidden = forbidden;
            true
        } else {
            false
        }
    }

    /// ポンを実行
    pub fn do_pon(&mut self, player_idx: usize) -> bool {
        if !self.can_pon(player_idx) {
            return false;
        }

        let tile = self.last_discard.unwrap();
        let from_player = self.last_discarder;
        let player = &mut self.players[player_idx];

        // 同じ牌を2枚削除
        if !player.hand.remove_tile(&tile) || !player.hand.remove_tile(&tile) {
            return false;
        }

        // #83 副露表示: ポンは 3 枚同種なので claimed_index = 0 で OK。
        let meld = crate::hand::Meld {
            meld_type: crate::hand::MeldType::Pon,
            tiles: vec![tile, tile, tile],
            is_open: true,
            from_player,
            is_kakan: false,
            claimed_index: Some(0),
        };

        player.hand.push_meld(meld); // #132 二重除去回避 (2 枚は明示 remove 済み)
        self.last_discard = None;
        self.current_player = player_idx;
        // #49 鳴きで他家の一発を消す
        self.clear_ippatsu_others(player_idx);
        // #51 鳴き発生で地和を不成立にする
        self.any_call_made_this_round = true;
        // #59 食い替え禁止: ポンは現物 (鳴いた牌と同種) のみ禁止。筋食い替えは無い。
        // discard_tile 側は tile_type のみ比較するので is_red は無視される。
        self.kuikae_forbidden = vec![tile];
        // #57 包: 大三元/大四喜 がこのポンで確定したか
        self.check_pao_after_call(player_idx, from_player);
        // #55 流し満貫: 打牌が鳴かれた記録
        if let Some(f) = from_player {
            if f < self.discard_taken_from.len() {
                self.discard_taken_from[f] = true;
            }
        }
        true
    }

    /// 明槓を実行
    pub fn do_kan(&mut self, player_idx: usize) -> bool {
        if !self.can_kan(player_idx) {
            return false;
        }

        let tile = self.last_discard.unwrap();
        let from_player = self.last_discarder;
        let player = &mut self.players[player_idx];

        // 同じ牌を3枚削除
        for _ in 0..3 {
            if !player.hand.remove_tile(&tile) {
                return false;
            }
        }

        // #83 副露表示: 大明槓は claimed_index = 0、from_player に直前打牌者を入れる。
        let meld = crate::hand::Meld {
            meld_type: crate::hand::MeldType::Kan,
            tiles: vec![tile, tile, tile, tile],
            is_open: true,
            from_player,
            is_kakan: false,
            claimed_index: Some(0),
        };

        player.hand.push_meld(meld); // #132 二重除去回避 (3 枚は明示 remove 済み)
        self.last_discard = None;

        // 槓ドラ追加
        if let Some(dora_indicator) = self.wall.pop() {
            self.dora_indicators.push(dora_indicator);
        }

        // 嶺上牌をツモ
        if let Some(rinshan_tile) = self.wall.pop() {
            self.draw_tile_to(player_idx, rinshan_tile);
        }

        // #50 嶺上開花候補
        self.last_was_rinshan = true;
        // 鳴きが入ったので他家の一発フラグを下ろす
        self.clear_ippatsu_others(player_idx);
        // #51 鳴き発生で地和を不成立にする
        self.any_call_made_this_round = true;
        // #57 包: 大三元/大四喜/四槓子 がこの大明槓で確定したか
        self.check_pao_after_call(player_idx, from_player);
        // #55 流し満貫: 打牌が鳴かれた記録
        if let Some(f) = from_player {
            if f < self.discard_taken_from.len() {
                self.discard_taken_from[f] = true;
            }
        }

        self.current_player = player_idx;
        true
    }

    /// 暗槓を実行
    pub fn do_ankan(&mut self, player_idx: usize, tile: Tile) -> bool {
        let player = &mut self.players[player_idx];

        // 同じ牌を4枚削除
        for _ in 0..4 {
            if !player.hand.remove_tile(&tile) {
                return false;
            }
        }

        // #83 副露表示: 暗槓は from_player / claimed_index ともに None。
        let meld = crate::hand::Meld {
            meld_type: crate::hand::MeldType::Kan,
            tiles: vec![tile, tile, tile, tile],
            is_open: false,
            from_player: None,
            is_kakan: false,
            claimed_index: None,
        };

        player.hand.push_meld(meld); // #132 二重除去回避 (4 枚は明示 remove 済み)

        // 槓ドラ追加
        if let Some(dora_indicator) = self.wall.pop() {
            self.dora_indicators.push(dora_indicator);
        }

        // 嶺上牌をツモ
        if let Some(rinshan_tile) = self.wall.pop() {
            self.draw_tile_to(player_idx, rinshan_tile);
        }

        // #50 嶺上開花候補 (暗槓も対象)
        self.last_was_rinshan = true;
        // 暗槓も鳴きの一種で、他家の一発を消す (リーチ後の暗槓は条件付きで許容されるが
        // ここでは厳密ルールではなく「他家の一発はクリアする」シンプル運用)
        self.clear_ippatsu_others(player_idx);
        // #51 鳴き発生で地和を不成立にする
        self.any_call_made_this_round = true;

        true
    }

    /// 加槓 (小明槓) 可能な牌の一覧を返す (#46)。
    ///
    /// 既に副露している Pon meld と同じ牌が手牌に 1 枚以上ある場合のみ候補。
    /// 「同じ牌」は `Tile == Tile` で判定するため、5m と 5m(赤) は別扱い。
    /// 既存仕様の `Player::hand.remove_tile` も `==` ベースなので整合する。
    pub fn can_shouminkan(&self, player_idx: usize) -> Vec<Tile> {
        if player_idx >= self.players.len() {
            return Vec::new();
        }
        let hand = &self.players[player_idx].hand;
        let tiles = hand.get_tiles();
        let melds = hand.get_melds();
        let mut candidates: Vec<Tile> = Vec::new();
        for meld in melds {
            if !matches!(meld.meld_type, crate::hand::MeldType::Pon) {
                continue;
            }
            // Pon meld は同じ牌 3 枚で構成される。先頭で十分。
            let Some(meld_tile) = meld.tiles.first() else {
                continue;
            };
            // 手牌に 1 枚以上同じ牌が必要。等価判定は赤ドラ違いを別扱い。
            if tiles.iter().any(|t| t == meld_tile) && !candidates.contains(meld_tile) {
                candidates.push(*meld_tile);
            }
        }
        candidates
    }

    /// 加槓を宣言する (#46 槍槓発火配線)。
    ///
    /// 仕様:
    /// 1. `can_shouminkan(player_idx)` に `tile` が含まれていなければ false
    /// 2. `pending_chankan = Some(tile)` をセット (他家のロン猶予期間に入る)
    /// 3. **この時点では meld の書き換え・嶺上ツモ・dora 追加はまだしない**。
    ///    他家がロンを宣言したらキャンセル、しなければ `complete_shouminkan` で
    ///    完了する 2 段階 API。
    /// 4. 手牌から `tile` は**抜かない**。槍槓ロン判定で `Player::can_win(tile)`
    ///    が打牌相当に振る舞うため、wasm 側は `pending_chankan` をフラグとして
    ///    `is_chankan=true` の ScoringContext を組み立てる
    ///    (`build_scoring_context` で参照済み)。
    pub fn start_shouminkan(&mut self, player_idx: usize, tile: Tile) -> bool {
        if player_idx >= self.players.len() {
            return false;
        }
        if self.pending_chankan.is_some() {
            return false;
        }
        if !self.can_shouminkan(player_idx).contains(&tile) {
            return false;
        }
        self.pending_chankan = Some(tile);
        // 加槓宣言が出た時点では `last_discard` は変えない。槍槓ロンの牌は
        // pending_chankan を直接見る前提。
        true
    }

    /// 加槓を完了する (#46)。誰もロン宣言しなかった場合に呼ぶ。
    ///
    /// 1. 該当 Pon meld を探して Kan meld に書き換え (is_open=true 維持)
    /// 2. 手牌から `tile` を 1 枚除去
    /// 3. 嶺上ツモ + 槓ドラ追加 + `last_was_rinshan=true`
    /// 4. `pending_chankan = None` にして槍槓窓口を閉じる
    ///
    /// 該当 Pon meld が見つからない / 手牌に tile が無い場合は false (state は変えない)。
    pub fn complete_shouminkan(&mut self, player_idx: usize, tile: Tile) -> bool {
        if player_idx >= self.players.len() {
            return false;
        }
        // start で立てた pending_chankan と一致しない呼び出しは弾く
        // (== なら同一 tile。違う場合は API 利用が不整合)
        if self.pending_chankan != Some(tile) {
            return false;
        }

        // Pon meld の index を探す
        let pon_index = {
            let melds = self.players[player_idx].hand.get_melds();
            melds.iter().position(|m| {
                matches!(m.meld_type, crate::hand::MeldType::Pon)
                    && m.tiles.first().map_or(false, |t| *t == tile)
            })
        };
        let Some(idx) = pon_index else {
            return false;
        };

        // 手牌から該当牌を 1 枚除去
        if !self.players[player_idx].hand.remove_tile(&tile) {
            return false;
        }

        // Pon meld を Kan meld に書き換える。is_open は加槓なので true 維持。
        // tiles ベクタを 4 枚に拡張する形で更新する。
        {
            let melds = self.players[player_idx].hand.get_melds_mut();
            if let Some(meld) = melds.get_mut(idx) {
                meld.meld_type = crate::hand::MeldType::Kan;
                meld.tiles.push(tile);
                meld.is_open = true;
                // #83 副露表示: 加槓 (Pon → Kan 昇格) フラグを立てる。from_player は
                // 元 Pon の値を維持 (= 元の鳴き元家を保持)、claimed_index も触らない。
                meld.is_kakan = true;
            } else {
                // 直前に position で見つけているので通常到達不能
                return false;
            }
        }

        // 槓ドラ追加
        if let Some(dora_indicator) = self.wall.pop() {
            self.dora_indicators.push(dora_indicator);
        }
        // 嶺上ツモ
        if let Some(rinshan_tile) = self.wall.pop() {
            self.draw_tile_to(player_idx, rinshan_tile);
        }
        self.last_was_rinshan = true;
        // 他家の一発を消す
        self.clear_ippatsu_others(player_idx);
        // #51 鳴き発生で地和を不成立にする
        self.any_call_made_this_round = true;
        // 槍槓窓口を閉じる
        self.pending_chankan = None;
        self.current_player = player_idx;
        true
    }

    /// 加槓をキャンセルする (#46)。誰かが槍槓ロンを宣言した場合に呼ぶ。
    ///
    /// `pending_chankan = None` にするだけのべき等な API。meld 書き換えも
    /// 嶺上ツモもまだしていないので、手牌・副露は加槓宣言前の状態のまま。
    /// 槍槓ロン側 (`resolve_win_ron`) は本関数が呼ばれる前に `pending_chankan`
    /// を読んで `is_chankan=true` の ScoringContext を組むため、呼び順は
    /// 「resolve_win_ron → cancel_shouminkan」が正しい。
    pub fn cancel_shouminkan(&mut self) {
        self.pending_chankan = None;
    }

    /// 自家以外のプレイヤーの一発フラグを下ろす (#49)。
    ///
    /// 鳴き (チー / ポン / カン) が入ったとき、立直していた他家の一発が消える。
    fn clear_ippatsu_others(&mut self, actor: usize) {
        for (i, p) in self.players.iter_mut().enumerate() {
            if i != actor {
                p.clear_ippatsu();
            }
        }
    }

    /// 指定プレイヤーが立直宣言可能かを Game コンテキスト込みで判定する (#91)。
    ///
    /// `Player::can_riichi()` の門前 / テンパイ / 持ち点 1000 以上 / 未リーチ
    /// に加え、麻雀標準ルールである **「山牌残り 4 枚以上」** をここで担保する。
    ///
    /// `uso_riichi_enabled=true` のとき (#89):
    ///   テンパイ・点数（1000点以上）の要件を外し、門前 + 未リーチ + 山牌 4 枚以上のみで
    ///   立直宣言可能にする。
    pub fn can_riichi(&self, player_idx: usize) -> bool {
        if player_idx >= self.players.len() {
            return false;
        }
        // 標準ルール: 山牌 4 枚未満では立直不可
        if self.wall.len() < 4 {
            return false;
        }
        if self.uso_riichi_enabled {
            // 嘘リーチ有効: 門前 + 未リーチのみチェック
            let p = &self.players[player_idx];
            if !p.hand.get_melds().is_empty() {
                return false;
            }
            if p.is_riichi {
                return false;
            }
            true
        } else {
            if !self.players[player_idx].can_riichi() {
                return false;
            }
            true
        }
    }

    /// 立直を宣言する (#49 / #91)。
    ///
    /// `Game::can_riichi` を満たす場合のみ `Player::declare_riichi` を呼ぶ。
    /// 加えて第一巡 (鳴きなし + 河 0 or 1) かつ全員の打牌が初手以下なら
    /// ダブル立直として `double_riichi=true` をセットする。
    ///
    /// 第一巡判定の簡易ルール:
    /// - 当該プレイヤーの discards が 0 (まだ捨ててない、ツモ直後の宣言)
    /// - 他家からの鳴きが全く発生していない (`Hand::get_melds().is_empty()` を全員でチェック)
    /// - 本局でまだ誰の打牌回数も 1 を超えていない
    pub fn declare_riichi(&mut self, player_idx: usize) -> bool {
        if !self.can_riichi(player_idx) {
            return false;
        }
        // ダブル立直判定: 鳴き無し + 全員 1 巡目以内
        let is_first_round = self
            .players
            .iter()
            .all(|p| p.hand.get_melds().is_empty() && p.discards.len() == 0);
        // #89: 嘘リーチ判定 — 宣言時点で非テンパイ（または 1000 点未満）なら uso_riichi=true
        let is_uso = self.uso_riichi_enabled && !self.players[player_idx].can_riichi();
        let turn = self.round as usize;
        if is_uso {
            // 嘘リーチ: Player::declare_riichi は can_riichi() をガードするので直接セット
            let p = &mut self.players[player_idx];
            p.is_riichi = true;
            p.riichi_turn = Some(turn);
            p.ippatsu = true;
            p.subtract_score(1000);
            p.uso_riichi = true;
        } else if !self.players[player_idx].declare_riichi(turn) {
            // Game::can_riichi で通った後に Player::declare_riichi が false を返すのは
            // 想定外 (両者は同じ Player::can_riichi を経由するため)。防御的に false 返却。
            return false;
        }
        if is_first_round {
            self.players[player_idx].double_riichi = true;
        }
        // 立直棒を供託に積む
        self.riichi_sticks += 1;
        true
    }

    /// #60 オープンリーチを宣言する。通常立直と同じ条件で、手牌公開 + 和了時 +1 飜。
    /// 通常立直の処理を行ったうえで `open_riichi=true` を立てる。
    pub fn declare_open_riichi(&mut self, player_idx: usize) -> bool {
        if !self.declare_riichi(player_idx) {
            return false;
        }
        if player_idx < self.players.len() {
            self.players[player_idx].open_riichi = true;
        }
        true
    }

    /// 場風 (#53)。
    ///
    /// round 1-4 → 東 (Ton)、round 5-8 → 南 (Nan)。半荘戦想定。
    /// (西入・北入の対応は未実装。round 9+ は南扱いでクランプ)
    pub fn round_wind(&self) -> Honor {
        if self.round <= 4 { Honor::Ton } else { Honor::Nan }
    }

    /// 指定プレイヤーの自風 (#53)。
    ///
    /// dealer からの相対位置で割り当てる:
    /// - dealer 本人 = 東 (Ton)
    /// - dealer + 1 = 南 (Nan)
    /// - dealer + 2 = 西 (Shaa)
    /// - dealer + 3 = 北 (Pei)
    pub fn seat_wind(&self, player_idx: usize) -> Honor {
        let rel = (player_idx + 4 - self.dealer) % 4;
        match rel {
            0 => Honor::Ton,
            1 => Honor::Nan,
            2 => Honor::Shaa,
            _ => Honor::Pei,
        }
    }

    /// 指定プレイヤー視点の `ScoringContext` を構築する (#49 / #50 / #53 / #54)。
    ///
    /// 立直 / 一発 / ダブル立直は `Player` の各フラグから引く。
    /// 状況役 (海底 / 河底 / 嶺上 / 槍槓) は `Game` の追跡フラグから組み立てる。
    /// 場風 / 自風 / ドラ表示牌は `Game` の対局状態から取得する。
    ///
    /// `is_tsumo` フラグの解釈:
    /// - true: ツモ和了経路 → 海底 (`is_last_draw`) / 嶺上 (`last_was_rinshan`) を有効化
    /// - false: ロン和了経路 → 河底 (`is_last_discard`) / 槍槓 (`pending_chankan`) を有効化
    ///
    /// `winning_tile`: ロン経路では `is_chankan` の判定に使用。
    ///   `pending_chankan == Some(winning_tile)` のときだけ `is_chankan=true` を立てる (#75)。
    ///   `None` を渡した場合は従来互換で `pending_chankan.is_some()` で判定する。
    pub fn build_scoring_context(&self, player_idx: usize, is_tsumo: bool) -> crate::scoring::ScoringContext {
        self.build_scoring_context_with_tile(player_idx, is_tsumo, None)
    }

    /// `winning_tile` を指定する版。`build_scoring_context` の内部実装。
    pub fn build_scoring_context_with_tile(
        &self,
        player_idx: usize,
        is_tsumo: bool,
        winning_tile: Option<&crate::tile::Tile>,
    ) -> crate::scoring::ScoringContext {
        use crate::scoring::ScoringContext;
        if player_idx >= self.players.len() {
            return ScoringContext::default();
        }
        let p = &self.players[player_idx];
        let is_dealer = player_idx == self.dealer;

        // #51: 天和 / 地和
        // 共通条件: ツモ和了 + 当該プレイヤーの discards 0 + 鳴き未発生
        // 天和: 上記 + 親 + 全員 discard 0 + 山牌から誰もツモっていない
        //       (= 親の配牌時 14 枚のままで和了 = `draws_this_round == 0`)
        // 地和: 上記 + 子 + 「自分の第 1 ツモ」 + 全員 discard 0
        //       (= 子i 番目の第 1 ツモ index = i = `draws_this_round == winner_seat_offset`)
        let no_calls_yet = !self.any_call_made_this_round;
        let winner_no_discards = p.discards.is_empty();
        let all_no_discards = self.players.iter().all(|pl| pl.discards.is_empty());
        let is_tenhou = is_tsumo
            && is_dealer
            && no_calls_yet
            && winner_no_discards
            && all_no_discards
            && self.draws_this_round == 0;
        // 子の seat offset (親=0 から時計回りで 1..=3)
        let seat_offset = ((player_idx + 4 - self.dealer) % 4) as u32;
        let is_chiihou = is_tsumo
            && !is_dealer
            && no_calls_yet
            && winner_no_discards
            && all_no_discards
            && self.draws_this_round == seat_offset;

        ScoringContext {
            is_tsumo,
            is_dealer,
            is_riichi: p.is_riichi && !p.double_riichi,
            is_double_riichi: p.double_riichi,
            is_ippatsu: p.ippatsu,
            is_haitei: is_tsumo && self.is_last_draw,
            is_houtei: !is_tsumo && self.is_last_discard,
            is_rinshan: is_tsumo && self.last_was_rinshan,
            is_chankan: !is_tsumo && match winning_tile {
                // #75: winning_tile 指定あり → pending_chankan と一致するときだけ is_chankan=true
                Some(wt) => self.pending_chankan == Some(*wt),
                // winning_tile 未指定 → 後方互換で pending_chankan.is_some() で判定
                None => self.pending_chankan.is_some(),
            },
            round_wind: self.round_wind(),
            seat_wind: self.seat_wind(player_idx),
            dora_indicators: self.dora_indicators.clone(),
            // 裏ドラは現状未実装 (#54 fast path)。立直成立時のみ集計対象なので空でも実害なし。
            uradora_indicators: Vec::new(),
            is_tenhou,
            is_chiihou,
            allow_open_tanyao: self.allow_open_tanyao,
            allow_local_yakuman: self.allow_local_yakuman,
            // #58 人和: 子 + ロン + 当該プレイヤー discards 0 + 無鳴き (第一巡ロン)。
            is_renhou: !is_tsumo
                && !is_dealer
                && no_calls_yet
                && winner_no_discards,
            // #60 オープンリーチ
            is_open_riichi: p.open_riichi,
        }
    }

    /// 東西戦: 和了者のチームに役を 1 件登録する。
    ///
    /// - `GameMode::EastWest` 以外のモードでは安全に no-op（`team_progress` は変更されない）。
    /// - 既に同じ役を登録済みなら no-op（`HashSet` の性質）。
    /// - `winner_seat` の範囲チェックはしない（`team_of` が常に East/West を返すため安全）。
    pub fn record_team_yaku(&mut self, winner_seat: usize, yaku: Yaku) {
        if self.mode != GameMode::EastWest {
            return;
        }
        let team = team_of(winner_seat);
        self.team_progress.entry(team).or_insert_with(HashSet::new).insert(yaku);
    }

    /// 東西戦: 指定チームのクリア進捗を返す。
    ///
    /// `east_west_target_yaku()` の並び順でソートされた `Vec<Yaku>` を返す。
    /// （クリア対象 5 種のうち登録済みのものだけ含める。順序が安定なので CLI 表示と
    /// テスト assert で揺れない。）
    pub fn team_clear_progress(&self, team: Team) -> Vec<Yaku> {
        let empty = HashSet::new();
        let set = self.team_progress.get(&team).unwrap_or(&empty);
        east_west_target_yaku()
            .iter()
            .filter(|y| set.contains(y))
            .cloned()
            .collect()
    }

    /// 東西戦: 指定チームがクリア対象 5 役を全て揃えたか。
    pub fn is_team_cleared(&self, team: Team) -> bool {
        let empty = HashSet::new();
        let set = self.team_progress.get(&team).unwrap_or(&empty);
        east_west_target_yaku().iter().all(|y| set.contains(y))
    }

    /// 東西戦の勝者チーム。両方 cleared の同時成立はゲーム性質上ほぼ起きないが、
    /// 起きた場合は East を優先して返す（決定論的）。
    pub fn east_west_winner(&self) -> Option<Team> {
        if self.is_team_cleared(Team::East) {
            Some(Team::East)
        } else if self.is_team_cleared(Team::West) {
            Some(Team::West)
        } else {
            None
        }
    }

    // ========================================
    // RealTime（リアルタイム麻雀）API
    // ========================================

    /// 全プレイヤーのタイマーを `delta_ms` 進める。
    ///
    /// `GameMode::RealTime` 以外でも安全に呼べる（フィールドは常に保持しているため）が、
    /// 他モードでは意味を持たない。
    pub fn tick_timers(&mut self, delta_ms: u64) {
        for timer in self.player_timers.iter_mut() {
            timer.tick(delta_ms);
        }
    }

    /// 現在タイムアウト中のプレイヤー idx 一覧。
    ///
    /// `elapsed_ms >= limit_ms` を満たすプレイヤーを昇順で返す。
    pub fn timed_out_players(&self) -> Vec<usize> {
        self.player_timers
            .iter()
            .enumerate()
            .filter_map(|(i, t)| if t.is_timeout() { Some(i) } else { None })
            .collect()
    }

    /// 指定プレイヤーが手牌から自動ツモ切り（手牌末尾＝最新ツモ）。
    ///
    /// - 範囲外 idx なら None
    /// - 手牌が空なら None
    /// - 成功時: 手牌から末尾牌を除去 + 河に discard として追加 + `last_discard` 更新 +
    ///   `last_discard_hidden=false` + 当該プレイヤーのタイマーリセット
    ///
    /// **注意**: `current_player` は変更しない。RealTime ではターンの概念が無いため、
    /// この関数は「タイムアウトしたプレイヤーの自動打牌」だけを行い、誰が次に打つかは
    /// 呼び出し側のスケジューラに委ねる。
    pub fn auto_discard_for(&mut self, player_idx: usize) -> Option<Tile> {
        if player_idx >= self.players.len() {
            return None;
        }
        let tile = {
            let tiles = self.players[player_idx].hand.get_tiles();
            realtime::pick_auto_discard_tile(tiles)?
        };
        // Player::discard_tile は「手牌から remove + 河に追加」を 1 つの API でこなす。
        if !self.players[player_idx].discard_tile(tile) {
            return None;
        }
        self.last_discard = Some(tile);
        self.last_discard_hidden = false;
        self.last_discarder = Some(player_idx);
        self.is_last_discard = self.wall.is_empty();
        self.last_was_rinshan = false;
        self.player_timers[player_idx].reset();
        Some(tile)
    }

    /// 指定プレイヤーのタイマー（経過時間）を 0 に戻す。`limit_ms` は維持。
    ///
    /// `GameMode::RealTime` で `discard_tile` / `auto_discard_for` を**自前の経路で**
    /// 通したあとに呼び出し側が呼ぶ必要がある（`discard_tile` パス自体はタイマーに
    /// 触れないため）。`auto_discard_for` 内では自動的にリセットされる。
    ///
    /// **戻り値ポリシー**: 範囲外 `player_idx` は silently ignore（戻り値なし）。
    /// これは `tick_timers` / `timed_out_players` と揃えた API 形状で、呼び出し側に
    /// プレイヤー数の検査責任を持たせない（4 人席という前提が崩れたとき bool/Result
    /// で返してもハンドリングできることが少ないため）。範囲外の呼び出しは noop。
    pub fn reset_player_timer(&mut self, player_idx: usize) {
        if player_idx < self.player_timers.len() {
            self.player_timers[player_idx].reset();
        }
    }

    /// 同フレームに集まった鳴き宣言から優先順位通りに 1 件採用する。
    ///
    /// `realtime::resolve_calls` のラッパー。優先順位は Ron > Pon > Kan > Chi、
    /// 同優先は入力順で先勝ち。
    pub fn resolve_pending_calls(
        &self,
        calls: &[realtime::Call],
    ) -> Option<realtime::Call> {
        realtime::resolve_calls(calls)
    }

    /// 対局終了判定。
    ///
    /// 以下のいずれかで true:
    /// - `game_over` フラグが立っている（`next_round` が終了したと判断したケース）
    /// - 東西戦でいずれかチームが 5 役クリア
    /// - 飛び: いずれかのプレイヤーのスコアが 0 未満
    ///   （`Player::subtract_score` は 0 でクランプするが、`pay_yakuman_tip` /
    ///   `pay_unclamped`（`resolve_win` / `resolve_draw` の徴収経路）はクランプを
    ///   回避するため負値になりうる。ここはそれを検知する）
    /// - Tonpuusen で round > 4 かつ親流れ
    /// - Hanchan で round > 8 かつ親流れ
    ///
    /// 「親流れ」とは `dealer_won_last == false` のこと。連荘中（true）は最終局を
    /// 超えても続行する（オーラス連荘）。
    pub fn is_game_over(&self) -> bool {
        if self.game_over {
            return true;
        }
        if self.mode == GameMode::EastWest && self.east_west_winner().is_some() {
            return true;
        }
        if self.players.iter().any(|p| p.score < 0) {
            return true;
        }
        let last_round = match self.length {
            Length::Tonpuusen => 4,
            Length::Hanchan => 8,
        };
        if self.round > last_round && !self.dealer_won_last {
            return true;
        }
        false
    }

    /// 和了の点数移動を実行する内部ヘルパー。
    ///
    /// `kind` / `is_dealer` から徴収先と分担額を決め、徴収マップを作って
    /// 一括で `pay_unclamped` で引く。winner には徴収額の合計を `add_score` で渡す。
    ///
    /// 仕様:
    /// - **ロン**: 放銃者から `total + honba_bonus` を一括徴収（本場全額放銃者持ち）
    /// - **親ツモ**: 子全員から `total / 3` ずつ + 本場 `honba_bonus / 3` ずつ
    /// - **子ツモ**: 親から `total / 2` + 他子 2 人から `total / 4` ずつ、本場は他家
    ///   全員から `honba_bonus / 3` ずつ均等
    ///
    /// **本場の二重加算は起きない**: 徴収マップを集計して winner に一度だけ add する。
    /// 戻り値は winner が受け取る合計点数（徴収マップの値の和）。
    fn apply_payment(
        &mut self,
        winner: usize,
        kind: WinKind,
        total_points: i32,
        honba_bonus: i32,
        is_dealer: bool,
    ) -> i32 {
        // 100 点単位切り上げ（伝統的な麻雀の点数計算ルール）。
        // 各支払者の負担額を 100 点単位に切り上げ、winner は実測合計を受領する
        // （元の `total_points` ではなく切り上げ後の合計 = ゼロサム保持）。
        fn ceil_to_hundred(n: i32) -> i32 {
            if n <= 0 {
                return n;
            }
            ((n + 99) / 100) * 100
        }

        // 徴収マップ: player_idx → 徴収額（正値、100 点単位切り上げ済み）
        let mut payments: Vec<(usize, i32)> = Vec::new();

        match kind {
            WinKind::Ron { from } => {
                if from < self.players.len() && from != winner {
                    let raw = total_points + honba_bonus;
                    payments.push((from, ceil_to_hundred(raw)));
                }
            }
            WinKind::Tsumo => {
                let honba_per_raw = if honba_bonus > 0 { honba_bonus / 3 } else { 0 };
                if is_dealer {
                    // 親ツモ: 子 3 人から total/3 ずつ（100 点単位切り上げ）
                    let per_raw = total_points / 3;
                    for i in 0..self.players.len() {
                        if i != winner {
                            payments.push((i, ceil_to_hundred(per_raw + honba_per_raw)));
                        }
                    }
                } else {
                    // 子ツモ: 親から total/2、他子 2 人から total/4 ずつ（各 100 点単位切り上げ）
                    let dealer_pay_raw = total_points / 2;
                    let ko_pay_raw = total_points / 4;
                    for i in 0..self.players.len() {
                        if i == winner {
                            continue;
                        }
                        let base = if i == self.dealer { dealer_pay_raw } else { ko_pay_raw };
                        payments.push((i, ceil_to_hundred(base + honba_per_raw)));
                    }
                }
            }
        }

        // 徴収を実行
        let mut total_received = 0i32;
        for (idx, amount) in &payments {
            // #118 割れ目: 払う側 or 受け取る側 (winner) が割れ目なら 2 倍。
            let amt = if self.warime_player == Some(*idx) || self.warime_player == Some(winner) {
                *amount * 2
            } else {
                *amount
            };
            self.players[*idx].pay_unclamped(amt);
            total_received += amt;
        }
        // winner に合計を渡す（本場ボーナス含む。重複加算しない）
        self.players[winner].add_score(total_received);
        total_received
    }

    /// #57 winner が和了した役の中に、winner を beneficiary とする包の対象役満が
    /// 含まれていれば、その責任者 index を返す。
    fn find_pao_responsible(&self, winner: usize, yaku: &[Yaku]) -> Option<usize> {
        if !self.enforce_pao {
            return None;
        }
        self.pao_liabilities
            .iter()
            .find(|p| p.beneficiary == winner && yaku.contains(&p.yaku))
            .map(|p| p.responsible)
    }

    /// #57 包の点数移動。責任者が総得点を負担する。
    /// - ツモ: 責任者が全額（他家は払わない）
    /// - ロン: 放銃者と責任者で折半（放銃者が本場ボーナスを負担）。
    ///   放銃者 == 責任者なら責任者が全額。
    fn apply_pao_payment(
        &mut self,
        winner: usize,
        kind: WinKind,
        total_points: i32,
        honba_bonus: i32,
        responsible: usize,
    ) {
        fn ceil_to_hundred(n: i32) -> i32 {
            if n <= 0 {
                return n;
            }
            ((n + 99) / 100) * 100
        }
        let mut payments: Vec<(usize, i32)> = Vec::new();
        match kind {
            WinKind::Tsumo => {
                payments.push((responsible, ceil_to_hundred(total_points + honba_bonus)));
            }
            WinKind::Ron { from } => {
                if from == responsible {
                    payments.push((responsible, ceil_to_hundred(total_points + honba_bonus)));
                } else {
                    let half = total_points / 2;
                    payments.push((responsible, ceil_to_hundred(half)));
                    payments.push((from, ceil_to_hundred(total_points - half + honba_bonus)));
                }
            }
        }
        let mut total_received = 0i32;
        for (idx, amount) in &payments {
            if *idx == winner {
                continue;
            }
            self.players[*idx].pay_unclamped(*amount);
            total_received += *amount;
        }
        self.players[winner].add_score(total_received);
    }

    /// 1 局の和了を確定させ、点数を移動して連荘フラグを更新する。
    ///
    /// - `ScoringResult.total_points`（親なら満貫=12000、子なら満貫=8000 等の合計値）と
    ///   本場ボーナス（`HONBA_BONUS * honba`）を `apply_payment` で適切に分担徴収する
    /// - 供託リーチ棒（`1000 * riichi_sticks`）を winner に渡す
    /// - 誠京 pot を winner に渡す（`winner_takes_pot`）
    /// - 親和了なら `dealer_won_last = true`（連荘）、子和了なら false（親流れ）
    /// - `last_outcome` に Win を記録、供託リーチ棒は 0 にリセット
    ///
    /// **役満ご祝儀**（誠京モードのみ）: `SEIKYO_YAKUMAN_TIP` を放銃者 (ロン) or
    /// 他家全員 (ツモ) から winner に追加で授受する。`Player::pay_yakuman_tip` /
    /// `receive_yakuman_tip` を経由するため 0 クランプせず、ゼロサムが保たれる。
    /// #57 包の確定判定。`player_idx` が `from` の打牌を鳴いて副露を増やした直後に呼ぶ。
    /// 大三元 (三元 3 種刻子/槓子) / 大四喜 (風 4 種) / 四槓子 (槓子 4) が
    /// この鳴きで確定したら `pao_liabilities` に責任関係を積む。
    fn check_pao_after_call(&mut self, player_idx: usize, from: Option<usize>) {
        if !self.enforce_pao {
            return;
        }
        let from = match from {
            Some(f) => f,
            None => return, // 暗槓など打牌者不在の鳴きは包なし
        };
        let melds = self.players[player_idx].hand.get_melds();
        // 三元牌の刻子/槓子の種類数
        let mut sangen = std::collections::HashSet::new();
        let mut winds = std::collections::HashSet::new();
        let mut kan_count = 0;
        for m in melds {
            if matches!(m.meld_type, crate::hand::MeldType::Pon | crate::hand::MeldType::Kan) {
                if let Some(t) = m.tiles.first() {
                    if let TileType::Honor(h) = t.tile_type {
                        match h {
                            Honor::Haku | Honor::Hatsu | Honor::Chun => {
                                sangen.insert(h);
                            }
                            Honor::Ton | Honor::Nan | Honor::Shaa | Honor::Pei => {
                                winds.insert(h);
                            }
                        }
                    }
                }
            }
            if matches!(m.meld_type, crate::hand::MeldType::Kan) {
                kan_count += 1;
            }
        }
        let mut push = |yaku: Yaku, this: &mut Self| {
            // 同一 beneficiary/yaku の重複は積まない
            if !this
                .pao_liabilities
                .iter()
                .any(|p| p.beneficiary == player_idx && p.yaku == yaku)
            {
                this.pao_liabilities.push(PaoLiability {
                    beneficiary: player_idx,
                    responsible: from,
                    yaku,
                });
            }
        };
        if sangen.len() == 3 {
            push(Yaku::Daisangen, self);
        }
        if winds.len() == 4 {
            push(Yaku::Daisuushii, self);
        }
        if kan_count == 4 {
            push(Yaku::Suukantsu, self);
        }
    }

    /// #61 本場縛り: 現在の honba と縛りルールに照らして、この和了結果が
    /// 最低点数縛りを満たすか。満たさない和了は無効（呼び出し側で和了拒否する）。
    ///
    /// - 2 飜縛り: 役（ドラ除く）が 2 飜以上、または役満。
    /// - 満貫縛り: base_points >= 2000（満貫以上）、または役満。
    /// - 役満縛り: 役満のみ。
    pub fn meets_shibari(&self, result: &crate::scoring::ScoringResult) -> bool {
        match self.shibari_rule {
            ShibariRule::Standard => true,
            ShibariRule::TwoHanFromFiveHonba => {
                if self.honba < 5 {
                    return true;
                }
                let yaku_han = result
                    .han
                    .saturating_sub(result.dora + result.uradora + result.akadora);
                yaku_han >= 2 || result.yakuman_count > 0
            }
            ShibariRule::ManganFromFiveHonba => {
                if self.honba < 5 {
                    return true;
                }
                result.base_points >= 2000 || result.yakuman_count > 0
            }
            ShibariRule::YakumanFromSevenHonba => {
                if self.honba < 7 {
                    return true;
                }
                result.yakuman_count > 0
            }
        }
    }

    pub fn resolve_win(
        &mut self,
        winner: usize,
        kind: WinKind,
        result: crate::scoring::ScoringResult,
    ) {
        if winner >= self.players.len() {
            return;
        }

        let total = result.total_points as i32;
        let honba_bonus = HONBA_BONUS * self.honba as i32;
        let riichi_bonus = 1000 * self.riichi_sticks as i32;
        let is_dealer_win = winner == self.dealer;
        // #42 #52: `ScoringResult.yakuman_count` を優先して使う (倍役満を含めて正確)。
        // 古い経路で `yakuman_count` が 0 のままセットされている場合は yaku 列から数える
        // (旧 dummy_result / yakuman_result テストとの後方互換)。
        let yakuman_count = if result.yakuman_count > 0 {
            result.yakuman_count
        } else {
            count_yakuman(&result.yaku)
        };

        // #57 包: 役満確定の責任払い。winner が責任払い対象の役満で和了したか判定。
        let pao_responsible = self.find_pao_responsible(winner, &result.yaku);
        if let Some(responsible) = pao_responsible {
            // 包成立: 責任者が総得点を負担する (ツモ=全額、ロン=放銃者と折半)。
            self.apply_pao_payment(winner, kind, total, honba_bonus, responsible);
        } else {
            // 点数移動（本場ボーナス込みで一括）
            self.apply_payment(winner, kind, total, honba_bonus, is_dealer_win);
        }

        // 供託リーチ棒を winner に渡す
        if riichi_bonus > 0 {
            self.players[winner].add_score(riichi_bonus);
            self.riichi_sticks = 0;
        }

        // 誠京モード: pot を winner に渡す
        self.winner_takes_pot(winner);

        // 誠京モード: 役満ご祝儀の授受
        if self.mode == GameMode::Seikyo && yakuman_count > 0 {
            self.pay_yakuman_tip(winner, kind, yakuman_count);
        }

        // 東西戦モード: 和了者のチームに役を全て記録 (5 役クリア判定用)
        if self.mode == GameMode::EastWest {
            for y in &result.yaku {
                self.record_team_yaku(winner, y.clone());
            }
        }

        // 連荘フラグ更新
        self.dealer_won_last = is_dealer_win;

        // 結果を保持
        self.last_outcome = Some(RoundOutcome::Win {
            winner,
            kind,
            result,
        });
    }

    /// 誠京モードの役満ご祝儀を授受する。
    /// - ロン: 放銃者から winner へ `SEIKYO_YAKUMAN_TIP * yakuman_count` の一括移動
    /// - ツモ: 他家全員から winner へ各 `SEIKYO_YAKUMAN_TIP * yakuman_count` ずつ移動
    ///   (winner の合計受取は `3 * tip`)
    /// 倍役満等で `yakuman_count >= 2` の場合は単純に倍率を乗じる。
    fn pay_yakuman_tip(&mut self, winner: usize, kind: WinKind, yakuman_count: u32) {
        let tip = SEIKYO_YAKUMAN_TIP * yakuman_count as i32;
        match kind {
            WinKind::Ron { from } => {
                if from >= self.players.len() || from == winner {
                    return;
                }
                self.players[from].pay_yakuman_tip(tip);
                self.players[winner].receive_yakuman_tip(tip);
            }
            WinKind::Tsumo => {
                for i in 0..self.players.len() {
                    if i == winner {
                        continue;
                    }
                    self.players[i].pay_yakuman_tip(tip);
                    self.players[winner].receive_yakuman_tip(tip);
                }
            }
        }
    }

    /// 流局（山牌 0 で誰も和了せず）を確定させ、聴牌料を計算する。
    ///
    /// 聴牌料テーブル（合計 ±3000 点の伝統ルール）:
    /// - 0 / 4 テンパイ: 移動なし
    /// - 1 テンパイ: テンパイ +3000、各ノーテン -1000
    /// - 2 テンパイ: 各テンパイ +1500、各ノーテン -1500
    /// - 3 テンパイ: 各テンパイ +1000、ノーテン -3000
    ///
    /// - 親テンパイ → `dealer_won_last = true`（連荘）
    /// - 親ノーテン → `dealer_won_last = false`（親流れ）
    /// - 供託リーチ棒は持ち越し（誰も取らない）
    /// - `last_outcome` に Draw を記録
    /// 各プレイヤーの `Player::is_tenpai()` を呼び、テンパイしている座席 index を集める。
    /// 流局時に `resolve_draw` へ渡すノーテン罰符徴収用の補助 API。
    ///
    /// 副露ありの聴牌判定は `Player::is_tenpai` / `Hand::is_tenpai` 経由で
    /// `Hand::shanten()` を呼び、`melds_needed = 4 - melds.len()` として
    /// `shanten_normal` を実行する（src/hand.rs:417-427）。
    /// つまり副露ありでも shanten 計算自体は動くが、現状の `shanten_normal` は
    /// 簡易実装のため精度が低く、副露ありや複雑な待ち形では誤判定し得る。
    /// 精度改善は `#33` / `#34` で対応予定。
    pub fn compute_tenpai_players(&self) -> Vec<usize> {
        self.players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_tenpai())
            .map(|(i, _)| i)
            .collect()
    }

    // ==================== #55 特殊（途中）流局 ====================

    /// 四風連打: 第一巡（各自 1 打のみ・無鳴き）で全員が同じ風牌を打牌したか。
    pub fn check_suufon_renda(&self) -> bool {
        if !self.allow_abortive_draws || self.any_call_made_this_round {
            return false;
        }
        // 全員ちょうど 1 打
        if !self.players.iter().all(|p| p.discards.len() == 1) {
            return false;
        }
        let first = self.players[0].discards[0].tile;
        let wind = matches!(
            first.tile_type,
            TileType::Honor(Honor::Ton | Honor::Nan | Honor::Shaa | Honor::Pei)
        );
        if !wind {
            return false;
        }
        self.players
            .iter()
            .all(|p| p.discards[0].tile.tile_type == first.tile_type)
    }

    /// 四家立直: 4 人全員が立直しているか。
    pub fn check_suucha_riichi(&self) -> bool {
        self.allow_abortive_draws && self.players.iter().all(|p| p.is_riichi)
    }

    /// 四槓散了: 槓が合計 4 つあり、かつ 2 人以上で打たれているか（1 人 4 槓は継続）。
    pub fn check_suukan_sanra(&self) -> bool {
        if !self.allow_abortive_draws {
            return false;
        }
        let mut total_kan = 0;
        let mut kan_owners = std::collections::HashSet::new();
        for (idx, p) in self.players.iter().enumerate() {
            for m in p.hand.get_melds() {
                if matches!(m.meld_type, crate::hand::MeldType::Kan) {
                    total_kan += 1;
                    kan_owners.insert(idx);
                }
            }
        }
        total_kan == 4 && kan_owners.len() >= 2
    }

    /// 九種九牌を宣言できるか。第一ツモ・無鳴き・当該プレイヤー無打牌で、
    /// 手牌（14 枚）に么九牌が 9 種類以上あること。
    pub fn can_declare_kyuushu(&self, player_idx: usize) -> bool {
        if !self.allow_abortive_draws || self.any_call_made_this_round {
            return false;
        }
        if player_idx >= self.players.len() {
            return false;
        }
        let p = &self.players[player_idx];
        if !p.discards.is_empty() {
            return false;
        }
        let mut yaochu_types = std::collections::HashSet::new();
        for t in p.hand.get_tiles() {
            let is_yaochu = match t.tile_type {
                TileType::Number { value, .. } => value == 1 || value == 9,
                TileType::Honor(_) => true,
            };
            if is_yaochu {
                yaochu_types.insert(t.tile_type);
            }
        }
        yaochu_types.len() >= 9
    }

    /// 特殊流局を確定させる。親はそのまま連荘、聴牌料は発生しない、
    /// 供託リーチ棒は持ち越し。本場は `next_round` 側で +1 される。
    pub fn apply_abortive_draw(&mut self, kind: AbortiveDrawKind) {
        // 親流れせず連荘扱い（dealer_won_last=true で next_round が本場を積む）
        self.dealer_won_last = true;
        self.last_outcome = Some(RoundOutcome::AbortiveDraw { kind });
    }

    /// #55 流し満貫の判定: 河がすべて么九牌で、かつ自分の打牌が一度も鳴かれていない
    /// プレイヤーの座席 index 一覧を返す。
    pub fn nagashi_mangan_players(&self) -> Vec<usize> {
        let mut result = Vec::new();
        for (idx, p) in self.players.iter().enumerate() {
            if p.discards.is_empty() {
                continue;
            }
            if idx < self.discard_taken_from.len() && self.discard_taken_from[idx] {
                continue; // 鳴かれていたら不成立
            }
            let all_yaochu = p.discards.iter().all(|d| match d.tile.tile_type {
                TileType::Number { value, .. } => value == 1 || value == 9,
                TileType::Honor(_) => true,
            });
            if all_yaochu {
                result.push(idx);
            }
        }
        result
    }

    pub fn resolve_draw(&mut self, tenpai_players: Vec<usize>) {
        // #55 流し満貫: 河が全て么九 + 無鳴きのプレイヤーがいれば満貫和了扱い。
        // 通常のテンパイ料は発生させず、流し満貫の支払いのみ行う。
        if self.allow_abortive_draws {
            let nagashi = self.nagashi_mangan_players();
            if !nagashi.is_empty() {
                for winner in &nagashi {
                    // 親流し満貫 = 12000 (子 4000 ずつ)、子流し満貫 = 8000 (親 4000 + 子 2000)。
                    // ツモ満貫と同じ分配で apply_payment を使う (base_points=2000)。
                    let is_dealer_win = *winner == self.dealer;
                    let total = if is_dealer_win { 12000 } else { 8000 };
                    self.apply_payment(*winner, WinKind::Tsumo, total, 0, is_dealer_win);
                }
                // 親が流し満貫なら連荘
                self.dealer_won_last = nagashi.contains(&self.dealer);
                self.last_outcome = Some(RoundOutcome::Draw { tenpai_players });
                return;
            }
        }
        let tenpai_count = tenpai_players.len();
        let dealer_tenpai = tenpai_players.contains(&self.dealer);

        let (per_tenpai, per_noten): (i32, i32) = match tenpai_count {
            1 => (3000, -1000),
            2 => (1500, -1500),
            3 => (1000, -3000),
            _ => (0, 0),
        };

        if per_tenpai != 0 {
            for i in 0..self.players.len() {
                if tenpai_players.contains(&i) {
                    self.players[i].add_score(per_tenpai);
                } else {
                    // per_noten は負の値。pay_unclamped で 0 クランプせず徴収
                    // （飛び検知のためゼロサム維持が必要）
                    self.players[i].pay_unclamped(-per_noten);
                }
            }
        }

        // 連荘フラグ更新（親テンパイで連荘）
        //
        // 仕様（意図的挙動）:
        // - 0 人テンパイ: dealer_tenpai == false → 親もノーテン扱い → 親流れ
        // - 4 人テンパイ: dealer_tenpai == true  → 親も聴牌扱い → 連荘
        // - 1〜3 人テンパイ: 親が tenpai_players に含まれるか否かで分岐
        //   罰符の支払い有無（per_tenpai == 0 のケース）とは独立に連荘判定する。
        self.dealer_won_last = dealer_tenpai;

        // #89: 嘘リーチに「追加の罰符」は無い。嘘リーチとは「テンパイしていないのに
        // リーチを宣言してしまい、和了できず流局で露見した状態」であり、その損失は
        // 普通の不成立リーチと同じく
        //   (1) 宣言時に供託したリーチ棒 1000 点の没収（declare_riichi で riichi_sticks へ
        //       積み済み。次局の和了者が回収する）
        //   (2) テンパイしていないので上の per_noten によるノーテン罰符の支払い
        // の 2 つで完結する。両方とも標準処理 (riichi_sticks / per_noten) でゼロサムが
        // 保たれており、ここで別途 pay_unclamped する必要はない（旧実装は二重徴収かつ
        // 徴収先が無く点棒が消滅していた）。uso_riichi フラグは can_riichi の緩和と、
        // 流局時の手牌公開判定 (#89 要件 3: 和了者がいれば非公開) のためだけに保持する。

        // TODO: 流し満貫 / 9 種 9 牌 / 四風連打 / 三家和 / 四開槓 / リーチ後チョンボ等の
        // 特殊流局は未対応。`RoundOutcome::Draw` に種別フィールド (enum) を追加して
        // 別 Issue で扱う。現状は通常流局（荒牌平局）のみ。
        self.last_outcome = Some(RoundOutcome::Draw { tenpai_players });
    }

    /// 次の局へ進む。`resolve_win` / `resolve_draw` 直後に呼ぶ。
    ///
    /// - 連荘 (`dealer_won_last == true`): dealer・round 据え置き、honba += 1
    /// - 親流れ: dealer = (dealer + 1) % 4、round += 1、honba = 0
    /// - 山牌・手牌・河・ドラ表示牌をリセット（`initialize_wall` + `deal_initial_tiles` 再呼出）
    /// - 局スコープ状態（リーチ・一発・ダブル立直）も `Player::reset_for_next_round` で初期化
    /// - 誠京モードなら場代を再徴収（`collect_seat_fee(SEIKYO_SEAT_FEE)`）
    /// - **ゲーム継続時のみ** `last_outcome` を None にクリアする。
    ///   ゲーム終了時（`is_game_over() == true`）は UI が直前局の結果を読み続けられるよう
    ///   `last_outcome` を保持したまま `game_over` フラグだけ立てて return する。
    /// - 戻り値: true = 続行、false = 対局終了（`game_over` フラグも立てる）
    pub fn next_round(&mut self) -> bool {
        // 終了判定（is_game_over は self を変更しないので先に呼べる）
        // ゲーム終了時は last_outcome を保持（UI が直前局の結果を読むため）
        if self.is_game_over() {
            self.game_over = true;
            return false;
        }

        if self.dealer_won_last {
            // 連荘: dealer 据え置き、honba +1
            self.honba += 1;
        } else {
            // 親流れ
            self.dealer = (self.dealer + 1) % 4;
            self.round += 1;
            self.honba = 0;
            // is_dealer フラグを書き直す
            for (i, p) in self.players.iter_mut().enumerate() {
                p.is_dealer = i == self.dealer;
            }
        }

        // 終了判定（round 進行後に再チェック）
        // ここでも last_outcome は保持したまま return（オーラス確定で UI が結果画面表示中）
        if self.is_game_over() {
            self.game_over = true;
            // 表示用に round を最終局にクランプ（UI は round をそのまま表示できる）
            let max_round = match self.length {
                Length::Tonpuusen => 4,
                Length::Hanchan => 8,
            };
            if self.round > max_round {
                self.round = max_round;
            }
            return false;
        }

        // 局スコープ状態を一括リセット（手牌・河・リーチ・一発等）
        for p in self.players.iter_mut() {
            p.reset_for_next_round();
        }
        self.dora_indicators.clear();
        self.current_player = self.dealer;
        self.last_discard = None;
        self.last_discard_hidden = false;
        // #50 状況役の局スコープ状態をリセット
        self.is_last_draw = false;
        self.is_last_discard = false;
        self.last_was_rinshan = false;
        self.pending_chankan = None;
        self.last_discarder = None;
        // #59 食い替え禁止牌をリセット
        self.kuikae_forbidden.clear();
        // #57 包の責任関係をリセット
        self.pao_liabilities.clear();
        // #55 流し満貫の打牌鳴かれフラグをリセット
        for f in self.discard_taken_from.iter_mut() {
            *f = false;
        }
        // #51 鳴き発生フラグをリセット
        self.any_call_made_this_round = false;
        self.draws_this_round = 0;
        // ゲーム継続が確定したのでここで初めて last_outcome をクリア
        self.last_outcome = None;

        // 山牌再構築 + 配牌
        self.initialize_wall();
        self.deal_initial_tiles();

        // 誠京: 場代再徴収
        if self.mode == GameMode::Seikyo {
            self.collect_seat_fee(SEIKYO_SEAT_FEE);
        }

        true
    }

    pub fn get_wall_count(&self) -> usize {
        self.wall.len()
    }

    pub fn get_dora_indicators(&self) -> &Vec<Tile> {
        &self.dora_indicators
    }

    pub fn get_game_state_string(&self) -> String {
        let mut result = String::new();
        
        result.push_str(&format!("Round: {} | Wall: {} tiles\n", self.round, self.wall.len()));
        result.push_str(&format!("Dora indicators: {}\n", 
            self.dora_indicators.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" ")));
        
        for (i, player) in self.players.iter().enumerate() {
            let marker = if i == self.current_player { ">" } else { " " };
            let dealer_mark = if player.is_dealer { "親" } else { " " };
            result.push_str(&format!("{}{} {} ({}点): {}\n",
                marker, dealer_mark, player.name, player.score, player.get_hand_string()));

            // 鷲巣麻雀: 自分以外（player id 0 = 「あなた」視点）の glass 牌を追加表示
            // CLI クライアントは player id 0 を観測者として想定。
            // 自分自身 (i == 0) はすでに手牌全表示済みなのでスキップ
            if self.mode == GameMode::Washizu && i != 0 {
                let glass_tiles = self.get_visible_tiles_of_opponent(0, i);
                let glass_str = if glass_tiles.is_empty() {
                    "（透明牌なし）".to_string()
                } else {
                    glass_tiles
                        .iter()
                        .map(|t| t.to_string())
                        .collect::<Vec<_>>()
                        .join(" ")
                };
                result.push_str(&format!("  [{} の透明牌: {}]\n", player.name, glass_str));
            }

            if !player.discards.is_empty() {
                result.push_str(&format!("  河: {}\n", player.get_discards_string()));
            }
        }
        
        if let Some(tile) = self.last_discard {
            result.push_str(&format!("Last discard: {}\n", tile.to_string()));
        }

        // 東西戦のクリア進捗表示
        if self.mode == GameMode::EastWest {
            result.push_str(&format!(
                "東チーム: {}\n",
                format_team_progress_line(self, Team::East)
            ));
            result.push_str(&format!(
                "西チーム: {}\n",
                format_team_progress_line(self, Team::West)
            ));
            if let Some(winner) = self.east_west_winner() {
                let label = match winner {
                    Team::East => "東",
                    Team::West => "西",
                };
                result.push_str(&format!("{}チーム勝利！\n", label));
            }
        }

        result
    }
}

/// 東西戦進捗の 1 行表示。
/// 例: `[✓三色同順, _一気通貫, _対々和, _全帯么, _混老頭]`
fn format_team_progress_line(game: &Game, team: Team) -> String {
    let set: HashSet<Yaku> = game
        .team_clear_progress(team)
        .into_iter()
        .collect();
    let parts: Vec<String> = east_west_target_yaku()
        .iter()
        .map(|y| {
            let mark = if set.contains(y) { "✓" } else { "_" };
            format!("{}{}", mark, yaku_label_ja(y))
        })
        .collect();
    format!("[{}]", parts.join(", "))
}

/// 東西戦クリア対象役の日本語ラベル。
fn yaku_label_ja(y: &Yaku) -> &'static str {
    match y {
        Yaku::SanshokuDoujun => "三色同順",
        Yaku::Ittsu => "一気通貫",
        Yaku::Toitoi => "対々和",
        Yaku::Chanta => "全帯么",
        Yaku::Honroutou => "混老頭",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_game_creation() {
        let names = vec!["Player1".to_string(), "Player2".to_string(), "Player3".to_string(), "Player4".to_string()];
        let game = Game::new(names);
        
        assert_eq!(game.players.len(), 4);
        assert!(game.players[0].is_dealer);
        assert_eq!(game.players[0].tile_count(), 14); // 親は14枚
        assert_eq!(game.players[1].tile_count(), 13); // 子は13枚
        assert_eq!(game.dora_indicators.len(), 1);
    }

    #[test]
    fn test_tile_draw_and_discard() {
        let names = vec!["P1".to_string(), "P2".to_string(), "P3".to_string(), "P4".to_string()];
        let mut game = Game::new(names);

        let initial_wall_count = game.get_wall_count();
        assert!(game.current_player_draw());
        assert_eq!(game.get_wall_count(), initial_wall_count - 1);

        let _player_tiles = game.get_current_player().get_hand_string();
        let first_tile = game.get_current_player().hand.get_tiles()[0];

        assert!(game.discard_tile(first_tile));
        assert_eq!(game.current_player, 1); // Next player
    }

    fn seikyo_names() -> Vec<String> {
        vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ]
    }

    #[test]
    fn test_standard_mode_pot_is_noop() {
        let mut game = Game::new(seikyo_names());
        assert_eq!(game.mode, GameMode::Standard);
        assert_eq!(game.pot, 0);

        let initial_scores: Vec<i32> = game.players.iter().map(|p| p.score).collect();
        game.collect_seat_fee(SEIKYO_SEAT_FEE);
        let after_scores: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        assert_eq!(game.pot, 0, "Standard モードでは pot が増えない");
        assert_eq!(initial_scores, after_scores, "Standard モードでは点数も動かない");
    }

    #[test]
    fn test_seikyo_collect_seat_fee() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        let initial_scores: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        game.collect_seat_fee(SEIKYO_SEAT_FEE);

        assert_eq!(game.pot, SEIKYO_SEAT_FEE * 4);
        for (i, p) in game.players.iter().enumerate() {
            assert_eq!(p.score, initial_scores[i] - SEIKYO_SEAT_FEE);
        }
    }

    #[test]
    fn test_seikyo_winner_takes_pot() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        game.collect_seat_fee(SEIKYO_SEAT_FEE);
        let pot_before = game.pot;
        let winner_score_before = game.players[1].score;

        let moved = game.winner_takes_pot(1);

        assert_eq!(moved, pot_before);
        assert_eq!(game.pot, 0);
        assert_eq!(game.players[1].score, winner_score_before + pot_before);
    }

    #[test]
    fn test_seikyo_dealer_double_draw() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        // 連荘していなければ二度ヅモは発動しない
        assert!(game.dealer_double_draw().is_none());

        game.dealer_won_last = true;
        game.current_player = game.dealer;
        let hand_before = game.players[game.dealer].tile_count();
        let wall_before = game.wall.len();

        let result = game.dealer_double_draw();
        assert!(result.is_some(), "Seikyo + 連荘 + 親手番なら二度ヅモが成立する");
        let hand_after = game.players[game.dealer].tile_count();
        let wall_after = game.wall.len();
        assert_eq!(hand_after, hand_before + 2, "親の手牌が 2 枚増える");
        assert_eq!(wall_after, wall_before - 2, "山牌が 2 枚減る");
    }

    #[test]
    fn test_seikyo_dealer_double_draw_not_dealer() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        game.dealer_won_last = true;
        game.current_player = (game.dealer + 1) % 4; // 子の手番

        assert!(game.dealer_double_draw().is_none(), "親の手番でなければ発動しない");
    }

    /// pot が 0 のときに `winner_takes_pot` を呼んでも winner の点数は不変
    #[test]
    fn test_winner_takes_empty_pot() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        // collect_seat_fee は呼ばない → pot は 0
        assert_eq!(game.pot, 0);

        let winner_score_before = game.players[2].score;
        let moved = game.winner_takes_pot(2);

        assert_eq!(moved, 0, "pot が空なら 0 が返る");
        assert_eq!(game.players[2].score, winner_score_before, "winner の点数は不変");
        assert_eq!(game.pot, 0);
    }

    /// `collect_seat_fee` を複数回呼ぶと pot が累積する（流局持ち越しのシミュレーション）
    #[test]
    fn test_pot_carries_over_multiple_collections() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);

        for _ in 0..3 {
            game.collect_seat_fee(SEIKYO_SEAT_FEE);
        }

        // 1000 × 4 人 × 3 回 = 12000
        assert_eq!(game.pot, SEIKYO_SEAT_FEE * 4 * 3);
        assert_eq!(game.pot, 12000);

        // 各プレイヤーは初期 25000 - 3000 = 22000
        for p in &game.players {
            assert_eq!(p.score, 25000 - SEIKYO_SEAT_FEE * 3);
        }
    }

    fn washizu_names() -> Vec<String> {
        vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ]
    }

    /// Washizu モードでは wall + 配牌 + dora indicator の合計のうち
    /// **3/4 が glass** になっていること。
    /// シャッフル後に先頭 75% を塗る決定的な整数演算実装なので、毎回ぴったり 102 枚 glass。
    /// exact 比較で固定。将来確率的実装（サンプリング等）に変えたら許容幅を緩める。
    #[test]
    fn test_washizu_wall_has_3_4_glass_tiles() {
        let game = Game::new_with_mode(washizu_names(), GameMode::Washizu);

        let mut total = 0usize;
        let mut glass = 0usize;

        // 山牌
        for t in &game.wall {
            total += 1;
            if t.is_glass {
                glass += 1;
            }
        }
        // 各プレイヤー手牌
        for p in &game.players {
            for t in p.hand.get_tiles() {
                total += 1;
                if t.is_glass {
                    glass += 1;
                }
            }
        }
        // ドラ表示牌
        for t in &game.dora_indicators {
            total += 1;
            if t.is_glass {
                glass += 1;
            }
        }

        assert_eq!(total, 136, "麻雀牌は全 136 枚");
        let expected = 136 * 3 / 4; // 102
        let diff = (glass as i32 - expected as i32).abs();
        assert!(
            diff <= 1,
            "glass 牌は 102 (3/4 of 136) 枚であるべき。実測 {} / 期待 {} (exact 比較)",
            glass,
            expected
        );
    }

    #[test]
    fn test_standard_wall_no_glass_tiles() {
        let game = Game::new(washizu_names());

        for t in &game.wall {
            assert!(!t.is_glass, "Standard モードでは glass が立たない");
        }
        for p in &game.players {
            for t in p.hand.get_tiles() {
                assert!(!t.is_glass, "Standard モードでは手牌にも glass が無い");
            }
        }
        for t in &game.dora_indicators {
            assert!(!t.is_glass, "Standard モードではドラ表示牌にも glass が無い");
        }
    }

    #[test]
    fn test_visible_tiles_self_returns_all() {
        let game = Game::new_with_mode(washizu_names(), GameMode::Washizu);

        for idx in 0..4 {
            let visible = game.get_visible_tiles_of_opponent(idx, idx);
            let own = game.players[idx].hand.get_tiles();
            assert_eq!(
                visible.len(),
                own.len(),
                "自分の手牌は glass/opaque 問わず全部見える (player {})",
                idx
            );
        }
    }

    #[test]
    fn test_visible_tiles_opponent_returns_glass_only() {
        let game = Game::new_with_mode(washizu_names(), GameMode::Washizu);

        // 観測者 0、対象 1
        let visible = game.get_visible_tiles_of_opponent(0, 1);
        let target_all = game.players[1].hand.get_tiles();
        let target_glass_count = target_all.iter().filter(|t| t.is_glass).count();

        assert_eq!(
            visible.len(),
            target_glass_count,
            "他家の glass 牌のみ見える"
        );
        for t in &visible {
            assert!(t.is_glass, "他家からの見えは必ず is_glass=true");
        }
    }

    /// Standard モードでは `get_visible_tiles_of_opponent` は他家に対して空を返す。
    #[test]
    fn test_visible_tiles_standard_mode_opponent_empty() {
        let game = Game::new(washizu_names());
        let visible = game.get_visible_tiles_of_opponent(0, 1);
        assert!(visible.is_empty(), "Standard では他家手牌は不可視");

        // ただし自分自身なら全表示
        let own = game.get_visible_tiles_of_opponent(0, 0);
        assert_eq!(own.len(), game.players[0].hand.get_tiles().len());
    }

    /// 5 枚麻雀の配牌: 子 5 枚 / 親 6 枚
    #[test]
    fn test_five_tile_initial_deal() {
        let names = vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ];
        let game = Game::new_with_mode(names, GameMode::FiveTile);

        assert_eq!(game.players[0].tile_count(), 6, "親（P1）は 6 枚");
        for i in 1..4 {
            assert_eq!(game.players[i].tile_count(), 5, "子（P{}）は 5 枚", i + 1);
        }
        // ドラ表示牌 1 枚は配られている
        assert_eq!(game.dora_indicators.len(), 1);
    }

    /// 二度ヅモ直後は親手牌が 15 枚になり、(1枚目, 2枚目) を返す（打牌は呼び出し側責務）。
    ///
    /// 実機の局ループでは「親が前局打牌した直後 = 手牌 13 枚」状態で次局が始まる想定。
    /// 本テストは Game::new 直後（親 14 枚）から 1 枚捨てて 13 枚にしてから二度ヅモする。
    #[test]
    fn test_dealer_double_draw_returns_two_tiles_and_hand_size_15() {
        let mut game = Game::new_with_mode(seikyo_names(), GameMode::Seikyo);
        game.current_player = game.dealer;

        // 親手牌を 14 → 13 にする（1 枚捨てる）
        let first_tile = game.players[game.dealer].hand.get_tiles()[0];
        assert!(game.players[game.dealer].discard_tile(first_tile));
        // discard_tile は current_player を回さないので手動でリセット
        game.current_player = game.dealer;
        assert_eq!(game.players[game.dealer].tile_count(), 13, "親手牌 13 枚に揃える");

        game.dealer_won_last = true;
        let result = game.dealer_double_draw();
        assert!(result.is_some(), "二度ヅモ成立");

        let (_t1, _t2) = result.unwrap();
        let hand_after = game.players[game.dealer].tile_count();
        assert_eq!(hand_after, 15, "親 13 + 2 ツモ = 15 枚（呼び出し側で 1 枚捨てて 14 枚に戻す）");
    }

    // ========================================
    // 東西戦（クリア麻雀）テスト
    // ========================================

    fn east_west_names() -> Vec<String> {
        vec![
            "東家".to_string(),
            "南家".to_string(),
            "西家".to_string(),
            "北家".to_string(),
        ]
    }

    /// 座席 0/2 → East、座席 1/3 → West の対応
    #[test]
    fn test_team_of_seats() {
        assert_eq!(team_of(0), Team::East, "東家(座席0)は East チーム");
        assert_eq!(team_of(2), Team::East, "西家(座席2)は East チーム");
        assert_eq!(team_of(1), Team::West, "南家(座席1)は West チーム");
        assert_eq!(team_of(3), Team::West, "北家(座席3)は West チーム");
    }

    /// 起動直後は両チームとも進捗 0 件
    #[test]
    fn test_east_west_no_yaku_recorded_initially() {
        let game = Game::new_with_mode(east_west_names(), GameMode::EastWest);
        assert!(game.team_clear_progress(Team::East).is_empty());
        assert!(game.team_clear_progress(Team::West).is_empty());
        assert!(!game.is_team_cleared(Team::East));
        assert!(!game.is_team_cleared(Team::West));
        assert_eq!(game.east_west_winner(), None);
    }

    /// 和了者の所属チームに役が登録される
    #[test]
    fn test_record_team_yaku_for_winner() {
        let mut game = Game::new_with_mode(east_west_names(), GameMode::EastWest);

        // 座席 0 (東家) が三色同順で和了 → East チームに追加
        game.record_team_yaku(0, Yaku::SanshokuDoujun);
        assert_eq!(game.team_clear_progress(Team::East), vec![Yaku::SanshokuDoujun]);
        assert!(game.team_clear_progress(Team::West).is_empty());

        // 座席 3 (北家) が一気通貫で和了 → West チームに追加
        game.record_team_yaku(3, Yaku::Ittsu);
        assert_eq!(game.team_clear_progress(Team::West), vec![Yaku::Ittsu]);
    }

    /// 同じ役を 2 回登録しても重複しない（HashSet）
    #[test]
    fn test_team_clear_progress_unique() {
        let mut game = Game::new_with_mode(east_west_names(), GameMode::EastWest);
        game.record_team_yaku(0, Yaku::Toitoi);
        game.record_team_yaku(0, Yaku::Toitoi);
        // 別の East 座席（2 = 西家）で再登録しても同じ
        game.record_team_yaku(2, Yaku::Toitoi);
        assert_eq!(game.team_clear_progress(Team::East), vec![Yaku::Toitoi]);
    }

    /// 5 役全て登録すると is_team_cleared が true
    #[test]
    fn test_is_team_cleared_after_five() {
        let mut game = Game::new_with_mode(east_west_names(), GameMode::EastWest);
        for y in east_west_target_yaku().iter() {
            game.record_team_yaku(0, y.clone());
        }
        assert!(game.is_team_cleared(Team::East));
        assert!(!game.is_team_cleared(Team::West));
    }

    /// どちらか cleared なら east_west_winner = Some
    #[test]
    fn test_east_west_winner_returns_first_cleared() {
        let mut game = Game::new_with_mode(east_west_names(), GameMode::EastWest);
        assert_eq!(game.east_west_winner(), None);

        // West チーム（座席 1 = 南家）が先に揃える
        for y in east_west_target_yaku().iter() {
            game.record_team_yaku(1, y.clone());
        }
        assert_eq!(game.east_west_winner(), Some(Team::West));
        // ゲーム終了判定もトリガーされる
        assert!(game.is_game_over());
    }

    /// EastWest 以外のモードでは record_team_yaku は no-op
    #[test]
    fn test_record_yaku_no_op_in_other_modes() {
        let mut game = Game::new(east_west_names());
        assert_eq!(game.mode, GameMode::Standard);

        for y in east_west_target_yaku().iter() {
            game.record_team_yaku(0, y.clone());
        }
        // Standard モードでは team_progress に書き込まれない
        assert!(game.team_clear_progress(Team::East).is_empty());
        assert!(!game.is_team_cleared(Team::East));
        assert_eq!(game.east_west_winner(), None);
    }

    /// 東西戦のターゲット役は 5 種ちょうど
    #[test]
    fn test_east_west_target_yaku_count() {
        let targets = east_west_target_yaku();
        assert_eq!(targets.len(), 5);
        assert!(targets.contains(&Yaku::SanshokuDoujun));
        assert!(targets.contains(&Yaku::Ittsu));
        assert!(targets.contains(&Yaku::Toitoi));
        assert!(targets.contains(&Yaku::Chanta));
        assert!(targets.contains(&Yaku::Honroutou));
    }

    // ========================================
    // Yamima（闇麻）テスト
    // ========================================

    fn yamima_names() -> Vec<String> {
        vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ]
    }

    /// Yamima モードで闇牌打牌すると last_discard_hidden=true、点数が 1000 減る。
    #[test]
    fn test_discard_hidden_tile_in_yamima() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];
        let score_before = game.players[current].score;

        assert!(game.discard_hidden_tile(tile));
        assert_eq!(game.players[current].score, score_before - YAMIMA_HIDDEN_COST);
        assert!(game.last_discard_hidden, "闇牌打牌後は last_discard_hidden=true");
        assert_eq!(game.last_discard, Some(tile));
        // 次プレイヤーへ進んでいる
        assert_eq!(game.current_player, (current + 1) % 4);
    }

    /// Yamima 以外のモードでは discard_hidden_tile は no-op で false。
    #[test]
    fn test_discard_hidden_tile_no_op_in_other_modes() {
        let mut game = Game::new(yamima_names());
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];

        assert!(!game.discard_hidden_tile(tile), "Standard では闇牌打牌不可");
        assert!(!game.last_discard_hidden);
    }

    /// 通常 discard_tile を呼ぶと last_discard_hidden が false に戻る。
    #[test]
    fn test_discard_tile_clears_hidden_flag() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        // 直接フラグを true にしてから通常打牌
        game.last_discard_hidden = true;
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];

        assert!(game.discard_tile(tile));
        assert!(!game.last_discard_hidden, "通常打牌で false に戻る");
    }

    /// 闇牌に対する can_pon は false（鳴き不可）。
    #[test]
    fn test_can_pon_returns_false_on_hidden_discard() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        // 強制的に「他家が手番、闇牌打牌直後、相手にポンできる牌がある」状態を作る
        let tile = Tile::new_number(Suit::Man, 5, false);
        // player 1 の手牌に同じ牌 2 枚を入れる
        game.players[1].hand = crate::hand::Hand::new();
        game.players[1].hand.add_tile(tile);
        game.players[1].hand.add_tile(tile);
        game.last_discard = Some(tile);
        game.last_discard_hidden = true;
        game.current_player = 0;

        assert!(!game.can_pon(1), "闇牌はポン不可");

        // フラグを下ろせばポン可能になる（リグレッション防止）
        game.last_discard_hidden = false;
        assert!(game.can_pon(1), "通常打牌ならポン可能");
    }

    /// 闇牌に対する can_kan / can_chi も false。
    #[test]
    fn test_can_kan_and_chi_return_false_on_hidden_discard() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let tile = Tile::new_number(Suit::Pin, 5, false);
        game.players[1].hand = crate::hand::Hand::new();
        for _ in 0..3 {
            game.players[1].hand.add_tile(tile);
        }
        // チー対象も仕込む（下家パターン: 4p, 6p を持つ）
        game.players[3].hand = crate::hand::Hand::new();
        game.players[3].hand.add_tile(Tile::new_number(Suit::Pin, 4, false));
        game.players[3].hand.add_tile(Tile::new_number(Suit::Pin, 6, false));

        game.last_discard = Some(tile);
        game.last_discard_hidden = true;
        game.current_player = 0;

        assert!(!game.can_kan(1), "闇牌は明槓不可");
        // current_player=0 の下家 = 3
        assert!(!game.can_chi(3), "闇牌はチー不可");
    }

    /// 闇牌に対する can_someone_win は常に空ベクタ（ロン不可）。
    #[test]
    fn test_can_someone_win_returns_empty_on_hidden_discard() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let tile = Tile::new_number(Suit::Sou, 1, false);
        game.last_discard = Some(tile);
        game.last_discard_hidden = true;
        game.current_player = 0;

        assert!(
            game.can_someone_win(&tile).is_empty(),
            "闇牌に対してロン宣言は不可（先に照射が必要）"
        );
    }

    /// 照射で 500 点支払い、対象の闇牌が公開されて tile が返る。
    #[test]
    fn test_light_up_reveals_and_costs_500() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];
        assert!(game.discard_hidden_tile(tile)); // current が闇牌打牌
        // current は元の current_player、闇牌は players[current].discards[0]
        let observer = (current + 1) % 4; // 次プレイヤー（current_player になっている）
        let score_before = game.players[observer].score;

        let revealed = game.light_up(observer, current, 0);
        assert_eq!(revealed, Some(tile));
        assert_eq!(
            game.players[observer].score,
            score_before - YAMIMA_LIGHT_UP_COST,
            "観測者から 500 点徴収"
        );
        assert!(
            !game.players[current].discards[0].is_hidden,
            "対象の河が公開される"
        );
    }

    /// 既に公開済の河を照射しても None（無効）。点数も引かない。
    #[test]
    fn test_light_up_fails_on_already_revealed() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];
        assert!(game.discard_tile(tile)); // 通常打牌（公開状態）
        let observer = (current + 1) % 4;
        let score_before = game.players[observer].score;

        assert_eq!(
            game.light_up(observer, current, 0),
            None,
            "公開済は照射対象外"
        );
        assert_eq!(
            game.players[observer].score, score_before,
            "失敗時は点数を引かない"
        );
    }

    /// 観測者の点数が 500 未満なら照射不可（None）。
    #[test]
    fn test_light_up_fails_when_observer_score_below_500() {
        let mut game = Game::new_with_mode(yamima_names(), GameMode::Yamima);
        let current = game.current_player;
        let tile = game.players[current].hand.get_tiles()[0];
        assert!(game.discard_hidden_tile(tile));
        let observer = (current + 1) % 4;
        game.players[observer].score = 499;

        assert_eq!(game.light_up(observer, current, 0), None);
        assert_eq!(game.players[observer].score, 499, "点数据え置き");
        assert!(
            game.players[current].discards[0].is_hidden,
            "対象は闇牌のまま"
        );
    }

    /// Yamima 以外では light_up は no-op で None。
    #[test]
    fn test_light_up_no_op_in_other_modes() {
        let mut game = Game::new(yamima_names());
        // 強引に Discard を仕込む（is_hidden=true のものを置く）
        use crate::player::Discard;
        let tile = Tile::new_number(Suit::Man, 1, false);
        game.players[0].discards.push(Discard { tile, is_hidden: true });

        assert_eq!(game.light_up(1, 0, 0), None, "Standard では照射不可");
    }

    // ========================================
    // RealTime（リアルタイム麻雀）統合 API のテスト
    // realtime モジュール単体ではなく Game レベルでの統合動作を検証する。
    // ========================================

    fn realtime_names() -> Vec<String> {
        vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ]
    }

    /// `auto_discard_for` に範囲外 idx (>= 4) を渡すと None で no-op。
    #[test]
    fn test_realtime_auto_discard_returns_none_for_out_of_range_idx() {
        let mut game = Game::new_with_mode(realtime_names(), GameMode::RealTime);
        let last_discard_before = game.last_discard;

        assert_eq!(
            game.auto_discard_for(4),
            None,
            "idx=4 は範囲外で None"
        );
        assert_eq!(
            game.auto_discard_for(99),
            None,
            "idx=99 も範囲外で None"
        );
        assert_eq!(
            game.last_discard, last_discard_before,
            "範囲外呼び出しでは last_discard も変化しない"
        );
    }

    /// 手牌を空にしてから `auto_discard_for` を呼ぶと None。
    #[test]
    fn test_realtime_auto_discard_returns_none_when_hand_empty() {
        let mut game = Game::new_with_mode(realtime_names(), GameMode::RealTime);
        // player 1 の手牌をすべて消す（remove で空に）
        while !game.players[1].hand.get_tiles().is_empty() {
            let tile = game.players[1].hand.get_tiles()[0];
            assert!(game.players[1].hand.remove_tile(&tile));
        }
        assert_eq!(game.players[1].hand.get_tiles().len(), 0);

        let last_discard_before = game.last_discard;
        assert_eq!(
            game.auto_discard_for(1),
            None,
            "手牌が空なら None"
        );
        assert_eq!(
            game.last_discard, last_discard_before,
            "空手牌時は last_discard が変化しない"
        );
    }

    /// `auto_discard_for` 成功時も `current_player` は変化しない（RealTime はターン制ではない）。
    #[test]
    fn test_realtime_auto_discard_does_not_change_current_player() {
        let mut game = Game::new_with_mode(realtime_names(), GameMode::RealTime);
        let before = game.current_player;

        // player 2 を強制 auto_discard（手牌は配牌で 13 枚あるはず）
        assert!(
            game.auto_discard_for(2).is_some(),
            "通常の配牌状態なら auto_discard_for は成功する"
        );
        assert_eq!(
            game.current_player, before,
            "auto_discard_for は current_player を進めない"
        );
    }

    /// `auto_discard_for(player_idx)` は当該プレイヤーのタイマーだけを 0 に戻し、
    /// 他プレイヤーの elapsed_ms は維持する。
    #[test]
    fn test_realtime_auto_discard_resets_only_that_players_timer() {
        let mut game = Game::new_with_mode(realtime_names(), GameMode::RealTime);
        // 全員のタイマーを 1234ms 進める
        game.tick_timers(1234);
        for t in &game.player_timers {
            assert_eq!(t.elapsed_ms, 1234);
        }

        // player 0 だけ auto_discard
        assert!(game.auto_discard_for(0).is_some());

        assert_eq!(
            game.player_timers[0].elapsed_ms, 0,
            "auto_discard した player 0 のタイマーは 0"
        );
        for i in 1..4 {
            assert_eq!(
                game.player_timers[i].elapsed_ms, 1234,
                "他プレイヤー (idx={i}) のタイマーは tick された値のまま"
            );
        }
    }

    /// `tick_timers(500)` は 4 人全員の elapsed_ms を一括で進める。
    #[test]
    fn test_realtime_tick_timers_advances_all_four() {
        let mut game = Game::new_with_mode(realtime_names(), GameMode::RealTime);
        for t in &game.player_timers {
            assert_eq!(t.elapsed_ms, 0, "初期値は 0");
        }

        game.tick_timers(500);

        for (i, t) in game.player_timers.iter().enumerate() {
            assert_eq!(
                t.elapsed_ms, 500,
                "player {i} の elapsed_ms は 500"
            );
        }

        // もう一度 tick すると加算される
        game.tick_timers(250);
        for t in &game.player_timers {
            assert_eq!(t.elapsed_ms, 750);
        }
    }

    /// `Game::resolve_pending_calls` は `realtime::resolve_calls` の単純委譲。
    /// 同じ入力に対して同じ結果が返ることを確認する。
    #[test]
    fn test_realtime_resolve_pending_calls_delegates_to_realtime_module() {
        let game = Game::new_with_mode(realtime_names(), GameMode::RealTime);
        use crate::realtime::{resolve_calls, Call, CallKind};

        // ケース 1: 複数の宣言から Ron が勝つ
        let calls = vec![
            Call { player_idx: 3, kind: CallKind::Chi },
            Call { player_idx: 1, kind: CallKind::Pon },
            Call { player_idx: 0, kind: CallKind::Ron },
        ];
        assert_eq!(
            game.resolve_pending_calls(&calls),
            resolve_calls(&calls),
            "Game::resolve_pending_calls は realtime::resolve_calls の委譲",
        );
        assert_eq!(
            game.resolve_pending_calls(&calls).unwrap().kind,
            CallKind::Ron,
        );

        // ケース 2: 空ベクタは None
        let empty: Vec<Call> = Vec::new();
        assert_eq!(game.resolve_pending_calls(&empty), None);
        assert_eq!(game.resolve_pending_calls(&empty), resolve_calls(&empty));

        // ケース 3: 同優先は先勝ち
        let same = vec![
            Call { player_idx: 2, kind: CallKind::Pon },
            Call { player_idx: 3, kind: CallKind::Pon },
        ];
        assert_eq!(
            game.resolve_pending_calls(&same),
            resolve_calls(&same),
        );
        assert_eq!(game.resolve_pending_calls(&same).unwrap().player_idx, 2);
    }

    // ========================================
    // 局終了→次局ループ（round-loop）テスト
    // ========================================

    use crate::scoring::ScoringResult;

    fn round_loop_names() -> Vec<String> {
        vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ]
    }

    fn dummy_result(total: u32) -> ScoringResult {
        ScoringResult {
            han: 1,
            fu: 30,
            yaku: Vec::new(),
            base_points: 0,
            total_points: total,
            ..Default::default()
        }
    }

    /// 親和了 → 連荘: dealer 据え置き、round 据え置き、honba +1
    #[test]
    fn test_renchan_keeps_dealer_and_increments_honba() {
        let mut game = Game::new(round_loop_names());
        let dealer_before = game.dealer;
        let round_before = game.round;
        let honba_before = game.honba;

        game.resolve_win(dealer_before, WinKind::Tsumo, dummy_result(8000));
        assert!(game.dealer_won_last, "親和了で連荘フラグ true");

        assert!(game.next_round(), "対局はまだ続く");
        assert_eq!(game.dealer, dealer_before, "連荘で dealer 据え置き");
        assert_eq!(game.round, round_before, "連荘で round 据え置き");
        assert_eq!(game.honba, honba_before + 1, "連荘で honba +1");
    }

    /// 子和了 → 親流れ: dealer +1、round +1、honba = 0
    #[test]
    fn test_dealer_rotation_advances_round_and_resets_honba() {
        let mut game = Game::new(round_loop_names());
        let dealer_before = game.dealer;
        let round_before = game.round;
        game.honba = 3;

        // 子（dealer ではない座席）が和了
        let non_dealer = (dealer_before + 1) % 4;
        game.resolve_win(non_dealer, WinKind::Tsumo, dummy_result(4000));
        assert!(!game.dealer_won_last, "子和了で連荘フラグ false");

        assert!(game.next_round());
        assert_eq!(game.dealer, (dealer_before + 1) % 4, "dealer +1");
        assert_eq!(game.round, round_before + 1, "round +1");
        assert_eq!(game.honba, 0, "親流れで honba リセット");
        // is_dealer フラグの整合性
        for (i, p) in game.players.iter().enumerate() {
            assert_eq!(p.is_dealer, i == game.dealer);
        }
    }

    /// 流局かつ親テンパイ → 連荘
    #[test]
    fn test_draw_with_dealer_tenpai_is_renchan() {
        let mut game = Game::new(round_loop_names());
        let dealer_before = game.dealer;
        let round_before = game.round;

        // 親だけテンパイ
        game.resolve_draw(vec![dealer_before]);
        assert!(game.dealer_won_last);

        assert!(game.next_round());
        assert_eq!(game.dealer, dealer_before);
        assert_eq!(game.round, round_before);
        assert_eq!(game.honba, 1);
    }

    /// 流局かつ親ノーテン → 親流れ
    #[test]
    fn test_draw_with_dealer_noten_rotates_dealer() {
        let mut game = Game::new(round_loop_names());
        let dealer_before = game.dealer;
        let round_before = game.round;

        // 子だけテンパイ
        game.resolve_draw(vec![(dealer_before + 1) % 4]);
        assert!(!game.dealer_won_last);

        assert!(game.next_round());
        assert_eq!(game.dealer, (dealer_before + 1) % 4);
        assert_eq!(game.round, round_before + 1);
    }

    /// 飛び（誰かのスコアが負）で対局終了
    #[test]
    fn test_tobi_triggers_game_over() {
        let mut game = Game::new(round_loop_names());
        // subtract_score は 0 でクランプするので直接代入で負値にする
        game.players[2].score = -100;

        assert!(game.is_game_over(), "負スコアで game over");
        assert!(!game.next_round(), "next_round は false を返す");
        assert!(game.game_over, "game_over フラグが立つ");
    }

    /// 東風戦: 東 4 局で親流れ → 対局終了
    #[test]
    fn test_tonpuusen_ends_at_east4_when_dealer_not_renchan() {
        let mut game = Game::new_with_mode_and_length(
            round_loop_names(),
            GameMode::Standard,
            Length::Tonpuusen,
        );
        // round=4 から始めて、子が和了 → 親流れ → round=5、終了
        game.round = 4;
        let dealer_before = game.dealer;
        let non_dealer = (dealer_before + 1) % 4;
        game.resolve_win(non_dealer, WinKind::Tsumo, dummy_result(4000));

        assert!(!game.next_round(), "東 4 局親流れで終了");
        assert!(game.game_over);
    }

    /// 半荘戦: 南 4 局（round=8）で親流れ → 対局終了
    #[test]
    fn test_hanchan_ends_at_south4_when_dealer_not_renchan() {
        let mut game = Game::new_with_mode_and_length(
            round_loop_names(),
            GameMode::Standard,
            Length::Hanchan,
        );
        game.round = 8;
        let dealer_before = game.dealer;
        let non_dealer = (dealer_before + 1) % 4;
        game.resolve_win(non_dealer, WinKind::Tsumo, dummy_result(4000));

        assert!(!game.next_round(), "南 4 局親流れで終了");
        assert!(game.game_over);
    }

    /// 流局 2 テンパイ 2 ノーテン → 各 ±1500
    #[test]
    fn test_tenpai_payments_2_2() {
        let mut game = Game::new(round_loop_names());
        let scores_before: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        // 0, 1 がテンパイ、2, 3 がノーテン
        game.resolve_draw(vec![0, 1]);

        assert_eq!(game.players[0].score, scores_before[0] + 1500);
        assert_eq!(game.players[1].score, scores_before[1] + 1500);
        assert_eq!(game.players[2].score, scores_before[2] - 1500);
        assert_eq!(game.players[3].score, scores_before[3] - 1500);
    }

    /// 流局 1 テンパイ → 聴牌者 +3000、ノーテン各 -1000
    #[test]
    fn test_tenpai_payments_1_3() {
        let mut game = Game::new(round_loop_names());
        let before = game.clone();
        let scores_before: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        game.resolve_draw(vec![2]);

        assert_eq!(game.players[0].score, scores_before[0] - 1000);
        assert_eq!(game.players[1].score, scores_before[1] - 1000);
        assert_eq!(game.players[2].score, scores_before[2] + 3000);
        assert_eq!(game.players[3].score, scores_before[3] - 1000);
        assert_score_conservation(&before, &game);
    }

    /// 流局 3 テンパイ → 聴牌者各 +1000、ノーテン -3000
    #[test]
    fn test_tenpai_payments_3_1() {
        let mut game = Game::new(round_loop_names());
        let before = game.clone();
        let scores_before: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        game.resolve_draw(vec![0, 1, 2]);

        assert_eq!(game.players[0].score, scores_before[0] + 1000);
        assert_eq!(game.players[1].score, scores_before[1] + 1000);
        assert_eq!(game.players[2].score, scores_before[2] + 1000);
        assert_eq!(game.players[3].score, scores_before[3] - 3000);
        assert_score_conservation(&before, &game);
        // 親 (idx=0) が聴牌側に含まれるので連荘
        // 罰符配分と連荘判定が独立して保たれることを固定
        assert!(
            game.dealer_won_last,
            "親 (idx=0) が聴牌のとき dealer_won_last == true（連荘）"
        );
    }

    /// 流局 0 テンパイ → 罰符無し（スコア不変）
    #[test]
    fn test_tenpai_payments_0_4() {
        let mut game = Game::new(round_loop_names());
        let scores_before: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        game.resolve_draw(vec![]);

        for i in 0..4 {
            assert_eq!(
                game.players[i].score, scores_before[i],
                "0 テンパイは罰符無し (player {i})"
            );
        }
        // 0 人テンパイ → 親もノーテン → 親流れ
        assert!(
            !game.dealer_won_last,
            "0 人テンパイは親流れ (dealer_won_last == false)"
        );
    }

    /// 流局 4 テンパイ → 罰符無し（スコア不変）
    #[test]
    fn test_tenpai_payments_4_0() {
        let mut game = Game::new(round_loop_names());
        let scores_before: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        game.resolve_draw(vec![0, 1, 2, 3]);

        for i in 0..4 {
            assert_eq!(
                game.players[i].score, scores_before[i],
                "4 テンパイは罰符無し (player {i})"
            );
        }
        // 4 人テンパイ → 親も聴牌 → 連荘
        assert!(
            game.dealer_won_last,
            "4 人テンパイは連荘 (dealer_won_last == true)"
        );
    }

    /// `Game::compute_tenpai_players` が `Player::is_tenpai` を 4 人分まわした
    /// 結果と一致する（WASM bridge は本関数を委譲する）。
    #[test]
    fn test_compute_tenpai_players_matches_is_tenpai() {
        let game = Game::new(round_loop_names());
        let expected: Vec<usize> = game
            .players
            .iter()
            .enumerate()
            .filter(|(_, p)| p.is_tenpai())
            .map(|(i, _)| i)
            .collect();
        assert_eq!(game.compute_tenpai_players(), expected);
    }

    /// 1 本場で和了 → 和了者に +300 追加で乗る
    #[test]
    fn test_honba_bonus_added_to_win() {
        let mut game = Game::new(round_loop_names());
        game.honba = 1;
        let winner = game.dealer;
        let winner_before = game.players[winner].score;
        // ロン: 放銃者を winner と別にする
        let from = (winner + 1) % 4;

        game.resolve_win(winner, WinKind::Ron { from }, dummy_result(8000));

        // winner: total_points (8000) + honba_bonus (300) を受け取る
        assert_eq!(
            game.players[winner].score,
            winner_before + 8000 + 300,
            "和了点 8000 + 本場ボーナス 300 = +8300"
        );
    }

    /// 供託リーチ棒は和了者に渡る
    #[test]
    fn test_riichi_sticks_go_to_winner() {
        let mut game = Game::new(round_loop_names());
        game.riichi_sticks = 2; // 2000 点ぶん
        let winner = 1;
        let winner_before = game.players[winner].score;
        let from = (winner + 1) % 4;

        game.resolve_win(winner, WinKind::Ron { from }, dummy_result(1000));

        // winner: 1000 (total) + 2000 (riichi_sticks)
        assert_eq!(game.players[winner].score, winner_before + 1000 + 2000);
        assert_eq!(game.riichi_sticks, 0, "供託は和了者が回収してリセット");
    }

    // ========================================
    // セルフレビュー指摘修正に伴う追加テスト (T1-T6)
    // ========================================

    /// ゼロサム保証ヘルパー: 局前後で
    /// 全プレイヤースコア + riichi_sticks*1000 + pot が一致する
    fn assert_score_conservation(before: &Game, after: &Game) {
        let total_before: i64 = before.players.iter().map(|p| p.score as i64).sum::<i64>()
            + (before.riichi_sticks as i64) * 1000
            + before.pot as i64;
        let total_after: i64 = after.players.iter().map(|p| p.score as i64).sum::<i64>()
            + (after.riichi_sticks as i64) * 1000
            + after.pot as i64;
        assert_eq!(
            total_before, total_after,
            "ゼロサム違反: before={total_before}, after={total_after}"
        );
    }

    /// T1. 親ツモ満貫: 子全員から 4000 ずつ徴収、親 +12000
    #[test]
    fn test_dealer_tsumo_splits_among_three_ko() {
        let mut game = Game::new(round_loop_names());
        let before = game.clone();
        let dealer = game.dealer; // 0
        let scores_before: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        // 親満貫ツモ total=12000
        game.resolve_win(dealer, WinKind::Tsumo, dummy_result(12000));

        // 親 +12000
        assert_eq!(
            game.players[dealer].score,
            scores_before[dealer] + 12000,
            "親は +12000 受領"
        );
        // 子 3 人は -4000
        for i in 0..4 {
            if i == dealer {
                continue;
            }
            assert_eq!(
                game.players[i].score,
                scores_before[i] - 4000,
                "子 {i} は -4000"
            );
        }
        assert_score_conservation(&before, &game);
    }

    /// T2. 子ツモ満貫: 親から 4000、他子 2 人から 2000 ずつ
    #[test]
    fn test_ko_tsumo_splits_dealer_half_ko_quarter() {
        let mut game = Game::new(round_loop_names());
        let before = game.clone();
        let dealer = game.dealer; // 0
        let winner = 1; // 子
        let scores_before: Vec<i32> = game.players.iter().map(|p| p.score).collect();

        // 子満貫ツモ total=8000
        game.resolve_win(winner, WinKind::Tsumo, dummy_result(8000));

        // 子（winner）+8000
        assert_eq!(
            game.players[winner].score,
            scores_before[winner] + 8000,
            "子 winner は +8000 受領"
        );
        // 親 -4000
        assert_eq!(
            game.players[dealer].score,
            scores_before[dealer] - 4000,
            "親は -4000"
        );
        // 他の子 (idx=2,3) -2000
        for i in [2, 3] {
            assert_eq!(
                game.players[i].score,
                scores_before[i] - 2000,
                "他子 {i} は -2000"
            );
        }
        assert_score_conservation(&before, &game);
    }

    /// T3. 流局のゼロサム保証
    #[test]
    fn test_draw_payment_conserves_score() {
        let mut game = Game::new(round_loop_names());
        let before = game.clone();

        game.resolve_draw(vec![0, 1]);
        assert_score_conservation(&before, &game);
    }

    /// T3. ロン和了のゼロサム保証（本場・リーチ棒含む）
    #[test]
    fn test_ron_payment_conserves_score_with_honba_and_riichi() {
        let mut game = Game::new(round_loop_names());
        game.honba = 2;
        game.riichi_sticks = 1;
        let before = game.clone();

        let winner = 0;
        let from = 2;
        game.resolve_win(winner, WinKind::Ron { from }, dummy_result(8000));
        assert_score_conservation(&before, &game);
    }

    /// T4. 飛び発火（実フロー）: 100 点プレイヤーが満貫ロン放銃 → game over
    #[test]
    fn test_tobi_via_real_payment_flow() {
        let mut game = Game::new(round_loop_names());
        // 放銃者の点数を 100 に絞る（pay_unclamped 経由で負スコアになるはず）
        game.players[2].score = 100;

        let winner = 0;
        let from = 2;
        // 満貫ロン 8000 → 100 - 8000 = -7900
        game.resolve_win(winner, WinKind::Ron { from }, dummy_result(8000));

        assert!(
            game.players[2].score < 0,
            "pay_unclamped 経由なので負スコアになる: actual={}",
            game.players[2].score
        );
        assert!(game.is_game_over(), "負スコア検知で game over");
    }

    /// T5. 本場 3 のロンで二重加算が起きない
    #[test]
    fn test_honba_no_double_count_on_ron() {
        let mut game = Game::new(round_loop_names());
        game.honba = 3; // 本場ボーナス = 300*3 = 900
        let winner = 1;
        let from = 2;
        let winner_before = game.players[winner].score;
        let from_before = game.players[from].score;

        // total=8000, honba_bonus=900
        game.resolve_win(winner, WinKind::Ron { from }, dummy_result(8000));

        assert_eq!(
            game.players[winner].score,
            winner_before + 8000 + 900,
            "winner +8900 (本場の二重加算なし)"
        );
        assert_eq!(
            game.players[from].score,
            from_before - 8000 - 900,
            "放銃者 -8900 (一括徴収)"
        );
    }

    /// T6. 局スコープ状態（リーチ・一発等）が next_round でリセットされる
    #[test]
    fn test_local_state_resets_on_next_round() {
        let mut game = Game::new(round_loop_names());
        // 局 1: player[0] にリーチ系フラグを立てる
        game.players[0].is_riichi = true;
        game.players[0].riichi_turn = Some(5);
        game.players[0].ippatsu = true;
        game.players[0].double_riichi = true;

        // 流局 → 親流れ
        let dealer = game.dealer;
        let non_dealer = (dealer + 1) % 4;
        game.resolve_draw(vec![non_dealer]);
        assert!(game.next_round());

        // 局 2: 全フラグがリセットされている
        assert!(!game.players[0].is_riichi, "is_riichi リセット");
        assert_eq!(game.players[0].riichi_turn, None, "riichi_turn リセット");
        assert!(!game.players[0].ippatsu, "ippatsu リセット");
        assert!(!game.players[0].double_riichi, "double_riichi リセット");
    }

    /// M4. ゲーム終了時に last_outcome が保持される
    #[test]
    fn test_last_outcome_preserved_on_game_over() {
        let mut game = Game::new_with_mode_and_length(
            round_loop_names(),
            GameMode::Standard,
            Length::Tonpuusen,
        );
        game.round = 4;
        let dealer = game.dealer;
        let non_dealer = (dealer + 1) % 4;
        // 子和了 → 親流れ → 東4局終了
        game.resolve_win(non_dealer, WinKind::Tsumo, dummy_result(4000));
        assert!(game.last_outcome.is_some(), "resolve_win 直後は当然ある");

        assert!(!game.next_round(), "東 4 局親流れで終了");
        assert!(game.game_over);
        assert!(
            game.last_outcome.is_some(),
            "ゲーム終了時は last_outcome が保持される（UI が結果画面を表示するため）"
        );
    }

    /// Length::default は Hanchan
    #[test]
    fn test_length_default_is_hanchan() {
        assert_eq!(Length::default(), Length::Hanchan);
    }

    /// S1. 100 点単位切り上げ
    /// - 7700 点子ロン: 放銃者 -7700、winner +7700（100 単位なので変化なし）
    /// - 7700 点親ツモ: 各子 7700/3 = 2566 → 切り上げ 2600 → winner +2600*3 = +7800
    #[test]
    fn test_apply_payment_ceils_to_hundred() {
        // ケース 1: 7700 点子ロン
        let mut game = Game::new(round_loop_names());
        let winner = 1; // 子
        let from = 2;
        let winner_before = game.players[winner].score;
        let from_before = game.players[from].score;
        game.resolve_win(winner, WinKind::Ron { from }, dummy_result(7700));
        assert_eq!(
            game.players[from].score,
            from_before - 7700,
            "放銃者 -7700（既に 100 単位なので変化なし）"
        );
        assert_eq!(
            game.players[winner].score,
            winner_before + 7700,
            "winner +7700"
        );

        // ケース 2: 7700 点親ツモ
        let mut game = Game::new(round_loop_names());
        let dealer = game.dealer;
        let scores_before: Vec<i32> = game.players.iter().map(|p| p.score).collect();
        game.resolve_win(dealer, WinKind::Tsumo, dummy_result(7700));
        // 子 3 人それぞれ 2600 ずつ徴収（7700/3 = 2566 → 切り上げ 2600）
        for i in 0..4 {
            if i == dealer {
                continue;
            }
            assert_eq!(
                game.players[i].score,
                scores_before[i] - 2600,
                "子 {i} は 2600 徴収（切り上げ）"
            );
        }
        // winner は 2600 × 3 = 7800 受領
        assert_eq!(
            game.players[dealer].score,
            scores_before[dealer] + 7800,
            "親は実測合計 +7800 受領"
        );
    }

    /// S1. ゼロサム保持: 7700 親ツモ、5200 子ツモ、5200 子ロンの各 odd ケース
    #[test]
    fn test_score_conservation_on_odd_point_values() {
        // 7700 親ツモ
        let mut game = Game::new(round_loop_names());
        let before = game.clone();
        let dealer = game.dealer;
        game.resolve_win(dealer, WinKind::Tsumo, dummy_result(7700));
        assert_score_conservation(&before, &game);

        // 5200 子ツモ（5200/2=2600, 5200/4=1300）
        let mut game = Game::new(round_loop_names());
        let before = game.clone();
        game.resolve_win(1, WinKind::Tsumo, dummy_result(5200));
        assert_score_conservation(&before, &game);

        // 5200 子ロン
        let mut game = Game::new(round_loop_names());
        let before = game.clone();
        game.resolve_win(1, WinKind::Ron { from: 2 }, dummy_result(5200));
        assert_score_conservation(&before, &game);

        // 本場 1 込みの 7700 親ツモ（本場 100/3 → 切り上げ 100）
        let mut game = Game::new(round_loop_names());
        game.honba = 1;
        let before = game.clone();
        let dealer = game.dealer;
        game.resolve_win(dealer, WinKind::Tsumo, dummy_result(7700));
        assert_score_conservation(&before, &game);
    }

    /// N2. ゲーム終了時に round が最終局を越えない（表示用クランプ）
    #[test]
    fn test_round_clamped_to_last_round_on_game_over_tonpuusen() {
        let mut game = Game::new_with_mode_and_length(
            round_loop_names(),
            GameMode::Standard,
            Length::Tonpuusen,
        );
        game.round = 4;
        let dealer = game.dealer;
        let non_dealer = (dealer + 1) % 4;
        game.resolve_win(non_dealer, WinKind::Tsumo, dummy_result(4000));
        assert!(!game.next_round(), "東 4 局親流れで終了");
        assert!(game.game_over);
        assert_eq!(game.round, 4, "round は最終局 4 にクランプされる（5 ではない）");
    }

    /// N2. 半荘戦でも同様にクランプされる
    #[test]
    fn test_round_clamped_to_last_round_on_game_over_hanchan() {
        let mut game = Game::new_with_mode_and_length(
            round_loop_names(),
            GameMode::Standard,
            Length::Hanchan,
        );
        game.round = 8;
        let dealer = game.dealer;
        let non_dealer = (dealer + 1) % 4;
        game.resolve_win(non_dealer, WinKind::Tsumo, dummy_result(4000));
        assert!(!game.next_round(), "南 4 局親流れで終了");
        assert!(game.game_over);
        assert_eq!(game.round, 8, "round は最終局 8 にクランプされる（9 ではない）");
    }

    // ========================================================================
    // Issue #28: 誠京モードの役満ご祝儀
    // ========================================================================

    fn yakuman_result(total: u32, yakuman: Yaku) -> ScoringResult {
        ScoringResult {
            han: 13,
            fu: 0,
            yaku: vec![yakuman],
            base_points: 0,
            total_points: total,
            ..Default::default()
        }
    }

    /// `count_yakuman` が役満バリアントを 1 つカウントし、非役満は 0
    #[test]
    fn test_count_yakuman_identifies_yakuman_variants() {
        use Yaku::*;
        assert_eq!(count_yakuman(&[Kokushi]), 1);
        assert_eq!(count_yakuman(&[Daisangen, Suuankou]), 2);
        assert_eq!(count_yakuman(&[Riichi, Tanyao, Pinfu]), 0);
        // Chuuren / Tenhou 等も役満として認識される
        assert_eq!(count_yakuman(&[Chuuren]), 1);
        assert_eq!(count_yakuman(&[Tenhou, Suuankou]), 2);
    }

    /// 誠京モードで役満ロン: 放銃者から winner へ祝儀 8000 が移動、ゼロサム保持
    #[test]
    fn test_seikyo_yakuman_ron_pays_tip_from_loser() {
        let mut game = Game::new_with_mode(round_loop_names(), GameMode::Seikyo);
        // collect_seat_fee は手動で呼ぶ (本テストは pot を整える)
        game.collect_seat_fee(SEIKYO_SEAT_FEE);
        let before = game.clone();
        let winner = 1;
        let from = 2;
        let result = yakuman_result(32000, Yaku::Kokushi);
        game.resolve_win(winner, WinKind::Ron { from }, result);

        // 放銃者は祝儀 8000 を追加で支払う
        let from_diff = game.players[from].score - before.players[from].score;
        let winner_diff = game.players[winner].score - before.players[winner].score;
        // 通常点 + 祝儀 8000 + pot の合計が反映されている
        assert!(from_diff <= -32000 - SEIKYO_YAKUMAN_TIP, "from 差分: {}", from_diff);
        assert!(winner_diff >= 32000 + SEIKYO_YAKUMAN_TIP, "winner 差分: {}", winner_diff);
        // ゼロサム保証
        assert_score_conservation(&before, &game);
    }

    /// 誠京モードで役満ツモ: 他家全員から winner へ各 8000 移動、合計 24000、ゼロサム保持
    #[test]
    fn test_seikyo_yakuman_tsumo_pays_tip_from_all_others() {
        let mut game = Game::new_with_mode(round_loop_names(), GameMode::Seikyo);
        game.collect_seat_fee(SEIKYO_SEAT_FEE);
        let before = game.clone();
        let winner = game.dealer; // 親役満ツモ
        let result = yakuman_result(48000, Yaku::Daisangen);
        game.resolve_win(winner, WinKind::Tsumo, result);

        // 他家 3 人全員から祝儀 8000 ずつ徴収されている
        for i in 0..4 {
            if i == winner {
                continue;
            }
            let diff = game.players[i].score - before.players[i].score;
            // 親ツモなら通常 16000 + 祝儀 8000 = -24000 以下
            assert!(diff <= -16000 - SEIKYO_YAKUMAN_TIP, "player {} 差分: {}", i, diff);
        }
        // ゼロサム保証 (pot + riichi_sticks 含む)
        assert_score_conservation(&before, &game);
    }

    /// Standard モードでは役満でも祝儀は移動しない
    #[test]
    fn test_standard_mode_no_yakuman_tip_even_if_yakuman() {
        let mut game = Game::new(round_loop_names());
        let before_winner_score = game.players[1].score;
        let before_from_score = game.players[2].score;
        let result = yakuman_result(8000, Yaku::Suuankou);
        game.resolve_win(1, WinKind::Ron { from: 2 }, result);

        let winner_diff = game.players[1].score - before_winner_score;
        let from_diff = before_from_score - game.players[2].score;
        // 通常点数のみ、祝儀 8000 は乗らない
        assert_eq!(winner_diff, 8000, "Standard では祝儀なし");
        assert_eq!(from_diff, 8000, "Standard では祝儀なし");
    }

    // ========================================================================
    // Issue #29: 東西戦の record_team_yaku 自動配線
    // ========================================================================

    /// 東西戦モードで和了 → 和了者のチームに役が自動記録される
    #[test]
    fn test_eastwest_resolve_win_records_team_yaku() {
        let mut game = Game::new_with_mode(round_loop_names(), GameMode::EastWest);
        let winner = 0; // East 家 (Team::East)
        let mut result = dummy_result(8000);
        result.yaku = vec![Yaku::SanshokuDoujun, Yaku::Toitoi];

        game.resolve_win(winner, WinKind::Tsumo, result);

        let east_progress = game.team_progress.get(&Team::East).expect("East 進捗");
        assert!(east_progress.contains(&Yaku::SanshokuDoujun), "三色同順が記録されている");
        assert!(east_progress.contains(&Yaku::Toitoi), "対々和が記録されている");
        // 反対チームには記録されない
        let west_progress = game.team_progress.get(&Team::West).expect("West 進捗");
        assert!(west_progress.is_empty(), "West 側は空のまま");
    }

    /// EastWest 以外のモードでは team_progress は変更されない
    #[test]
    fn test_standard_mode_does_not_record_team_yaku() {
        let mut game = Game::new(round_loop_names());
        let mut result = dummy_result(8000);
        result.yaku = vec![Yaku::SanshokuDoujun];

        game.resolve_win(0, WinKind::Tsumo, result);

        let east_progress = game.team_progress.get(&Team::East).expect("East 進捗");
        assert!(east_progress.is_empty(), "Standard モードでは team_progress 変更なし");
    }

    /// 東西戦で 5 役クリア → east_west_winner が返り、is_game_over が true
    #[test]
    fn test_eastwest_team_clears_5_yaku_triggers_game_over() {
        let mut game = Game::new_with_mode(round_loop_names(), GameMode::EastWest);
        let east_seat = 0; // Team::East の代表
        let targets = east_west_target_yaku();

        // 5 役を 1 度ずつ和了で積む
        for y in &targets {
            let mut result = dummy_result(2000);
            result.yaku = vec![y.clone()];
            game.resolve_win(east_seat, WinKind::Tsumo, result);
        }

        assert_eq!(game.east_west_winner(), Some(Team::East));
        assert!(game.is_game_over(), "5 役クリアで game over");
    }

    /// ダブル役満 (役満 2 個同時) はご祝儀も 2 倍 (8000 × 2 = 16000)
    #[test]
    fn test_seikyo_double_yakuman_doubles_tip() {
        let mut game = Game::new_with_mode(round_loop_names(), GameMode::Seikyo);
        game.collect_seat_fee(SEIKYO_SEAT_FEE);
        let before = game.clone();
        let winner = 1;
        let from = 2;
        let mut result = yakuman_result(64000, Yaku::Daisuushii);
        result.yaku.push(Yaku::Suuankou); // ダブル役満

        game.resolve_win(winner, WinKind::Ron { from }, result);

        let from_diff = before.players[from].score - game.players[from].score;
        // 通常 64000 + 祝儀 16000 (8000 × 2) = 80000 以上引かれている
        assert!(from_diff >= 64000 + SEIKYO_YAKUMAN_TIP * 2, "ダブル役満で祝儀 2 倍: {}", from_diff);
        assert_score_conservation(&before, &game);
    }

    /// Issue #48: `do_kan` (明槓) は嶺上ツモ 1 枚 + 槓ドラ 1 枚追加を実行する。
    ///
    /// - 鳴き対象の打牌 3 枚を手牌から削除し副露へ移すので、手牌枚数は -3 になる
    /// - 嶺上から 1 枚ツモするので最終的に手牌枚数は -3 + 1 = -2
    /// - dora_indicators が +1 (槓ドラ)
    /// - current_player が宣言者に移る
    #[test]
    fn test_do_kan_draws_rinshan_tile_and_adds_kan_dora() {
        let names = vec![
            "P1".to_string(),
            "P2".to_string(),
            "P3".to_string(),
            "P4".to_string(),
        ];
        let mut game = Game::new(names);

        // 強制的に「player 0 が打牌、player 1 が同じ牌を 3 枚持っていて明槓可能」状態を作る。
        let tile = Tile::new_number(Suit::Man, 5, false);
        game.players[1].hand = crate::hand::Hand::new();
        for _ in 0..3 {
            game.players[1].hand.add_tile(tile);
        }
        // 副露区別がついて見えるよう、関係無い牌も 1 枚混ぜておく (kan 後に残るはず)
        let other = Tile::new_number(Suit::Pin, 2, false);
        game.players[1].hand.add_tile(other);

        game.last_discard = Some(tile);
        game.last_discard_hidden = false;
        game.current_player = 0;

        // 山末尾を決定論的に固定する。do_kan は wall.pop() で
        // 「槓ドラ表示牌 → 嶺上牌」の順に引くので、最後に push したものが槓ドラ表示、
        // その手前が嶺上牌になる。嶺上牌に 5m が来ると「手牌から 5m が消えている」
        // アサーションが偶発的に落ちる (フラキー) ため、5m 以外を明示的に積む。
        let rinshan = Tile::new_number(Suit::Sou, 9, false);
        let kan_dora_indicator = Tile::new_number(Suit::Sou, 8, false);
        game.wall.push(rinshan); // 2 番目に pop → 嶺上牌
        game.wall.push(kan_dora_indicator); // 最初に pop → 槓ドラ表示牌

        let raw_hand_len_before = game.players[1].hand.get_tiles().len();
        let dora_count_before = game.dora_indicators.len();
        let wall_count_before = game.get_wall_count();

        assert!(game.can_kan(1), "前提: player 1 は明槓可能");
        let ok = game.do_kan(1);
        assert!(ok, "do_kan は成功する");

        // 嶺上牌 1 枚 + 槓ドラ表示牌 1 枚で wall が 2 枚減る
        assert_eq!(
            game.get_wall_count(),
            wall_count_before - 2,
            "嶺上ツモ + 槓ドラ表示で山が 2 枚減る"
        );

        // 槓ドラが追加されている
        assert_eq!(
            game.dora_indicators.len(),
            dora_count_before + 1,
            "do_kan で dora_indicators が +1 される (槓ドラ)"
        );

        // 副露の中身: 5m 4 枚の明槓 1 個
        let melds = game.players[1].hand.get_melds();
        assert_eq!(melds.len(), 1, "明槓 1 個が副露に追加される");
        assert!(matches!(melds[0].meld_type, crate::hand::MeldType::Kan));
        assert!(melds[0].is_open, "他家打牌のカンは明槓 (is_open=true)");
        assert_eq!(melds[0].tiles.len(), 4);
        assert!(melds[0].tiles.iter().all(|&t| t == tile));

        // 生 hand.tiles (副露を除く) は -3 (副露へ) + 1 (嶺上ツモ) = -2 枚になる
        let remaining = game.players[1].hand.get_tiles();
        assert_eq!(
            remaining.len(),
            raw_hand_len_before - 3 + 1,
            "生の手牌 (副露除く) は -3 + 1 で正味 -2 枚"
        );
        // 副露へ移ったので手牌 (raw) に 5m は残っていない
        assert!(
            !remaining.iter().any(|&t| t == tile),
            "手牌からは明槓した牌 (5m) が消えている"
        );
        // 関係無い牌は残っている
        assert!(remaining.iter().any(|&t| t == other), "他の手牌 (2p) は残る");

        // 手番は宣言者 (player 1) に移っている
        assert_eq!(game.current_player, 1, "do_kan 後の手番は宣言者");
        // last_discard はクリアされる
        assert!(game.last_discard.is_none(), "明槓で last_discard はクリアされる");
    }

    // ==================== #55 特殊（途中）流局 ====================

    fn abortive_names() -> Vec<String> {
        vec!["A".into(), "B".into(), "C".into(), "D".into()]
    }

    fn push_discard(game: &mut Game, player: usize, tile: Tile) {
        game.players[player]
            .discards
            .push(crate::player::Discard { tile, is_hidden: false });
    }

    #[test]
    fn test_suufon_renda() {
        let mut game = Game::new(abortive_names());
        for i in 0..4 {
            push_discard(&mut game, i, Tile::new_honor(Honor::Ton));
        }
        assert!(game.check_suufon_renda(), "全員東打ちで四風連打");
        // 1 人だけ違う風 → 不成立
        game.players[3].discards.clear();
        push_discard(&mut game, 3, Tile::new_honor(Honor::Nan));
        assert!(!game.check_suufon_renda(), "風が揃わなければ不成立");
    }

    #[test]
    fn test_suucha_riichi() {
        let mut game = Game::new(abortive_names());
        for p in game.players.iter_mut() {
            p.is_riichi = true;
        }
        assert!(game.check_suucha_riichi(), "全員立直で四家立直");
        game.players[2].is_riichi = false;
        assert!(!game.check_suucha_riichi(), "1 人未立直なら不成立");
    }

    #[test]
    fn test_can_declare_kyuushu() {
        let mut game = Game::new(abortive_names());
        game.players[0].hand = crate::hand::Hand::new();
        // 么九 9 種 + 適当な 5 枚
        for t in [
            Tile::new_number(Suit::Man, 1, false),
            Tile::new_number(Suit::Man, 9, false),
            Tile::new_number(Suit::Pin, 1, false),
            Tile::new_number(Suit::Pin, 9, false),
            Tile::new_number(Suit::Sou, 1, false),
            Tile::new_number(Suit::Sou, 9, false),
            Tile::new_honor(Honor::Ton),
            Tile::new_honor(Honor::Nan),
            Tile::new_honor(Honor::Shaa),
            Tile::new_number(Suit::Man, 3, false),
            Tile::new_number(Suit::Man, 4, false),
            Tile::new_number(Suit::Pin, 5, false),
            Tile::new_number(Suit::Pin, 6, false),
            Tile::new_number(Suit::Sou, 5, false),
        ] {
            game.players[0].hand.add_tile(t);
        }
        game.current_player = 0;
        assert!(game.can_declare_kyuushu(0), "么九 9 種で九種九牌宣言可");
        // 打牌済みなら不可
        push_discard(&mut game, 0, Tile::new_number(Suit::Man, 3, false));
        assert!(!game.can_declare_kyuushu(0), "打牌後は不可");
    }

    #[test]
    fn test_nagashi_mangan() {
        let mut game = Game::new(abortive_names());
        // player 0 の河が全て么九・鳴かれていない → 流し満貫
        push_discard(&mut game, 0, Tile::new_number(Suit::Man, 1, false));
        push_discard(&mut game, 0, Tile::new_honor(Honor::Chun));
        push_discard(&mut game, 0, Tile::new_number(Suit::Sou, 9, false));
        // player 1 は中張を含む
        push_discard(&mut game, 1, Tile::new_number(Suit::Pin, 5, false));
        let nagashi = game.nagashi_mangan_players();
        assert_eq!(nagashi, vec![0], "player 0 のみ流し満貫");

        // 鳴かれていたら不成立
        game.discard_taken_from[0] = true;
        assert!(game.nagashi_mangan_players().is_empty(), "鳴かれたら不成立");
    }

    #[test]
    fn test_nagashi_mangan_payment_in_resolve_draw() {
        let mut game = Game::new(abortive_names());
        // player 1 (子) が流し満貫
        push_discard(&mut game, 1, Tile::new_number(Suit::Man, 1, false));
        let before: Vec<i32> = game.players.iter().map(|p| p.score).collect();
        game.resolve_draw(vec![]);
        // 子流し満貫 8000: 親(0) 4000 + 子(2,3) 2000 ずつ
        assert_eq!(game.players[1].score - before[1], 8000, "流し満貫 +8000");
        assert_eq!(before[0] - game.players[0].score, 4000, "親 -4000");
    }

    // ==================== #118 割れ目 ====================

    fn warime_result(total: u32) -> crate::scoring::ScoringResult {
        crate::scoring::ScoringResult {
            han: 4,
            fu: 30,
            yaku: vec![Yaku::Tanyao],
            base_points: 2000,
            total_points: total,
            dora: 0,
            uradora: 0,
            akadora: 0,
            kandora: 0,
            yakuman_count: 0,
        }
    }

    /// 割れ目プレイヤーが放銃すると支払いが 2 倍。
    #[test]
    fn test_warime_discarder_pays_double() {
        let mut game = Game::new(vec!["A".into(), "B".into(), "C".into(), "D".into()]);
        game.warime_player = Some(0); // player 0 が割れ目
        let before: Vec<i32> = game.players.iter().map(|p| p.score).collect();
        // player 1 が player 0 からロン (子満貫 8000)
        game.resolve_win(1, WinKind::Ron { from: 0 }, warime_result(8000));
        assert_eq!(before[0] - game.players[0].score, 16000, "割れ目放銃で 2 倍 (16000)");
        assert_eq!(game.players[1].score - before[1], 16000, "winner も 2 倍受領");
    }

    /// 割れ目プレイヤーが和了すると受け取りが 2 倍。
    #[test]
    fn test_warime_winner_receives_double() {
        let mut game = Game::new(vec!["A".into(), "B".into(), "C".into(), "D".into()]);
        game.warime_player = Some(1); // player 1 (winner) が割れ目
        let before: Vec<i32> = game.players.iter().map(|p| p.score).collect();
        game.resolve_win(1, WinKind::Ron { from: 2 }, warime_result(8000));
        assert_eq!(before[2] - game.players[2].score, 16000, "割れ目和了で放銃者 2 倍");
        assert_eq!(game.players[1].score - before[1], 16000, "割れ目 winner 2 倍受領");
    }

    /// 割れ目無効 (None) なら通常通り。
    #[test]
    fn test_warime_disabled_normal() {
        let mut game = Game::new(vec!["A".into(), "B".into(), "C".into(), "D".into()]);
        let before: Vec<i32> = game.players.iter().map(|p| p.score).collect();
        game.resolve_win(1, WinKind::Ron { from: 0 }, warime_result(8000));
        assert_eq!(before[0] - game.players[0].score, 8000, "割れ目なしは通常 8000");
    }

    // ==================== #57 包 (責任払い) ====================

    fn pao_names() -> Vec<String> {
        vec!["A".into(), "B".into(), "C".into(), "D".into()]
    }

    fn sangen_pon(tile: Tile, from: usize) -> crate::hand::Meld {
        crate::hand::Meld {
            meld_type: crate::hand::MeldType::Pon,
            tiles: vec![tile, tile, tile],
            is_open: true,
            from_player: Some(from),
            is_kakan: false,
            claimed_index: Some(0),
        }
    }

    /// 白・發 をポン済みの player 1 が、player 0 の打牌 中 をポンして大三元確定
    /// → pao_liabilities に責任者 0 が積まれる。
    #[test]
    fn test_pao_daisangen_detected_on_pon() {
        let mut game = Game::new(pao_names());
        let chun = Tile::new_honor(Honor::Chun);
        game.players[1].hand = crate::hand::Hand::new();
        game.players[1].hand.add_tile(chun);
        game.players[1].hand.add_tile(chun);
        game.players[1].hand.add_meld(sangen_pon(Tile::new_honor(Honor::Haku), 0));
        game.players[1].hand.add_meld(sangen_pon(Tile::new_honor(Honor::Hatsu), 0));
        game.last_discard = Some(chun);
        game.last_discard_hidden = false;
        game.last_discarder = Some(0);
        game.current_player = 0;

        assert!(game.do_pon(1), "中をポンで大三元確定");
        assert_eq!(game.pao_liabilities.len(), 1, "包が 1 件積まれる");
        let p = &game.pao_liabilities[0];
        assert_eq!(p.beneficiary, 1);
        assert_eq!(p.responsible, 0);
        assert_eq!(p.yaku, Yaku::Daisangen);
    }

    /// 包成立時のロン支払い: 放銃者と責任者で折半。
    #[test]
    fn test_pao_ron_split_payment() {
        let mut game = Game::new(pao_names());
        game.pao_liabilities.push(PaoLiability {
            beneficiary: 1,
            responsible: 0,
            yaku: Yaku::Daisangen,
        });
        let before: Vec<i32> = game.players.iter().map(|p| p.score).collect();
        let result = crate::scoring::ScoringResult {
            han: 13,
            fu: 20,
            yaku: vec![Yaku::Daisangen],
            base_points: 8000,
            total_points: 32000,
            dora: 0,
            uradora: 0,
            akadora: 0,
            kandora: 0,
            yakuman_count: 1,
        };
        game.resolve_win(1, WinKind::Ron { from: 2 }, result);
        assert_eq!(game.players[1].score - before[1], 32000, "winner +32000");
        assert_eq!(before[0] - game.players[0].score, 16000, "責任者 -16000");
        assert_eq!(before[2] - game.players[2].score, 16000, "放銃者 -16000");
        assert_eq!(game.players[3].score, before[3], "無関係の 3 は変動なし");
    }

    /// 包成立時のツモ支払い: 責任者が全額負担。
    #[test]
    fn test_pao_tsumo_full_payment() {
        let mut game = Game::new(pao_names());
        game.pao_liabilities.push(PaoLiability {
            beneficiary: 1,
            responsible: 0,
            yaku: Yaku::Daisangen,
        });
        let before: Vec<i32> = game.players.iter().map(|p| p.score).collect();
        let result = crate::scoring::ScoringResult {
            han: 13,
            fu: 20,
            yaku: vec![Yaku::Daisangen],
            base_points: 8000,
            total_points: 32000,
            dora: 0,
            uradora: 0,
            akadora: 0,
            kandora: 0,
            yakuman_count: 1,
        };
        game.resolve_win(1, WinKind::Tsumo, result);
        assert_eq!(game.players[1].score - before[1], 32000, "winner +32000");
        assert_eq!(before[0] - game.players[0].score, 32000, "責任者が全額 -32000");
        assert_eq!(game.players[2].score, before[2], "他家 2 は変動なし");
        assert_eq!(game.players[3].score, before[3], "他家 3 は変動なし");
    }

    // ==================== #61 本場縛り ====================

    fn result_with(han: u32, dora: u32, base: u32, yakuman: u32) -> crate::scoring::ScoringResult {
        crate::scoring::ScoringResult {
            han,
            fu: 30,
            yaku: Vec::new(),
            base_points: base,
            total_points: 0,
            dora,
            uradora: 0,
            akadora: 0,
            kandora: 0,
            yakuman_count: yakuman,
        }
    }

    #[test]
    fn test_shibari_two_han_from_five_honba() {
        let mut game = Game::new(vec!["A".into(), "B".into(), "C".into(), "D".into()]);
        game.shibari_rule = ShibariRule::TwoHanFromFiveHonba;
        // 4 本場まではどんな手でも OK
        game.honba = 4;
        assert!(game.meets_shibari(&result_with(1, 0, 480, 0)), "4本場は1飜でも可");
        // 5 本場以降は役 2 飜以上が必要 (ドラは数えない)
        game.honba = 5;
        // 役1飜 + ドラ1 = han 2 だが役は1飜 → 不可
        assert!(!game.meets_shibari(&result_with(2, 1, 0, 0)), "役1飜+ドラ1は2飜縛り不可");
        // 役2飜 → 可
        assert!(game.meets_shibari(&result_with(2, 0, 0, 0)), "役2飜は可");
        // 役満は常に可
        assert!(game.meets_shibari(&result_with(13, 0, 8000, 1)), "役満は可");
    }

    #[test]
    fn test_shibari_mangan_from_five_honba() {
        let mut game = Game::new(vec!["A".into(), "B".into(), "C".into(), "D".into()]);
        game.shibari_rule = ShibariRule::ManganFromFiveHonba;
        game.honba = 5;
        assert!(!game.meets_shibari(&result_with(3, 0, 1920, 0)), "満貫未満は不可");
        assert!(game.meets_shibari(&result_with(5, 0, 2000, 0)), "満貫は可");
    }

    #[test]
    fn test_shibari_standard_always_ok() {
        let mut game = Game::new(vec!["A".into(), "B".into(), "C".into(), "D".into()]);
        game.honba = 9;
        assert!(game.meets_shibari(&result_with(1, 0, 480, 0)), "標準は常時可");
    }

    // ==================== #132 add_meld 二重除去回避 ====================

    /// 5m を 3 枚持った状態でポンすると、副露に 3 枚移り手牌に 1 枚残る。
    /// 旧実装 (明示 remove 2 + add_meld remove 3) では 3 枚全部消えていた。
    #[test]
    fn test_pon_with_triple_retains_extra() {
        let mut game = Game::new(vec!["A".into(), "B".into(), "C".into(), "D".into()]);
        let five = Tile::new_number(Suit::Man, 5, false);
        game.players[1].hand = crate::hand::Hand::new();
        for _ in 0..3 {
            game.players[1].hand.add_tile(five);
        }
        game.players[1].hand.add_tile(Tile::new_number(Suit::Pin, 1, false));
        game.last_discard = Some(five);
        game.last_discard_hidden = false;
        game.last_discarder = Some(0);
        game.current_player = 0;

        assert!(game.do_pon(1), "ポン成立");
        let melds = game.players[1].hand.get_melds();
        assert_eq!(melds.len(), 1);
        assert_eq!(melds[0].tiles.len(), 3, "副露に 5m が 3 枚");
        let remaining = game.players[1].hand.get_tiles();
        let fives = remaining.iter().filter(|t| **t == five).count();
        assert_eq!(fives, 1, "3 枚持ちポンで 1 枚残る (二重除去されない)");
    }

    // ==================== #59 食い替え禁止 ====================

    fn kuikae_names() -> Vec<String> {
        vec!["A".into(), "B".into(), "C".into(), "D".into()]
    }

    /// ポン直後は現物 (鳴いた牌と同種) を切れない。
    #[test]
    fn test_kuikae_pon_genbutsu_forbidden() {
        let mut game = Game::new(kuikae_names());
        let five = Tile::new_number(Suit::Man, 5, false);
        // player 1 に 5m 3 枚 + 別の牌を持たせる (ポンで 2 枚消費しても 1 枚残る)
        game.players[1].hand = crate::hand::Hand::new();
        for _ in 0..3 {
            game.players[1].hand.add_tile(five);
        }
        game.players[1].hand.add_tile(Tile::new_number(Suit::Pin, 1, false));
        game.last_discard = Some(five);
        game.last_discard_hidden = false;
        game.last_discarder = Some(0);
        game.current_player = 0;

        assert!(game.do_pon(1), "ポン成立");
        assert_eq!(game.current_player, 1);
        assert_eq!(game.kuikae_forbidden, vec![five], "現物 5m が禁止");
        // 現物 5m は切れない
        assert!(!game.discard_tile(five), "ポン直後に現物 5m は打てない");
        // 別の牌は切れる
        assert!(
            game.discard_tile(Tile::new_number(Suit::Pin, 1, false)),
            "現物以外は打てる"
        );
        // 打牌後は禁止が解除される
        assert!(game.kuikae_forbidden.is_empty(), "打牌で食い替え禁止解除");
    }

    /// enforce_kuikae=false なら食い替え (筋 7m) を許可する (toggle)。
    /// チー (456m を 5m6m で鳴き) 後、手牌に残る筋牌 7m を切れることを確認する。
    #[test]
    fn test_kuikae_toggle_off_allows_suji() {
        let mut game = Game::new(kuikae_names());
        game.enforce_kuikae = false;
        let four = Tile::new_number(Suit::Man, 4, false);
        let seven = Tile::new_number(Suit::Man, 7, false);
        game.players[3].hand = crate::hand::Hand::new();
        game.players[3].hand.add_tile(Tile::new_number(Suit::Man, 5, false));
        game.players[3].hand.add_tile(Tile::new_number(Suit::Man, 6, false));
        game.players[3].hand.add_tile(seven);
        game.players[3].hand.add_tile(Tile::new_number(Suit::Pin, 1, false));
        game.last_discard = Some(four);
        game.last_discard_hidden = false;
        game.last_discarder = Some(0);
        game.current_player = 0;

        assert!(game.do_chi(3, 2), "456m チー成立");
        // toggle off でも禁止牌自体は計算される
        assert!(game.kuikae_forbidden.contains(&seven));
        // が、enforce_kuikae=false なので筋 7m を打てる
        assert!(game.discard_tile(seven), "toggle off なら筋 7m も打てる");
    }

    /// チー (456m を 5m6m で鳴き) 直後は現物 4m と筋 7m を切れない。
    #[test]
    fn test_kuikae_chi_genbutsu_and_suji_forbidden() {
        let mut game = Game::new(kuikae_names());
        let four = Tile::new_number(Suit::Man, 4, false);
        let seven = Tile::new_number(Suit::Man, 7, false);
        // チーは下家のみ: current_player=0 の下家 = (0+3)%4 = 3。player 3 にチー牌を仕込む。
        game.players[3].hand = crate::hand::Hand::new();
        game.players[3].hand.add_tile(Tile::new_number(Suit::Man, 5, false));
        game.players[3].hand.add_tile(Tile::new_number(Suit::Man, 6, false));
        game.players[3].hand.add_tile(four); // 切る用 (現物テスト)
        game.players[3].hand.add_tile(seven); // 切る用 (筋テスト)
        game.players[3].hand.add_tile(Tile::new_number(Suit::Pin, 1, false)); // 合法打牌用
        game.last_discard = Some(four);
        game.last_discard_hidden = false;
        game.last_discarder = Some(0);
        game.current_player = 0;

        // pattern 2: n,n+1,n+2 (= 4m,5m,6m)。手牌 5m6m を使う。
        assert!(game.do_chi(3, 2), "456m チー成立");
        assert_eq!(game.current_player, 3);
        assert!(game.kuikae_forbidden.contains(&four), "現物 4m 禁止");
        assert!(game.kuikae_forbidden.contains(&seven), "筋 7m 禁止");
        assert!(!game.discard_tile(four), "現物 4m は打てない");
        assert!(!game.discard_tile(seven), "筋 7m は打てない");
        assert!(
            game.discard_tile(Tile::new_number(Suit::Pin, 1, false)),
            "無関係の牌は打てる"
        );
    }

    /// 嵌張チー (3m5m で 4m を鳴き) は筋食い替えが無く、現物 4m のみ禁止。
    #[test]
    fn test_kuikae_chi_kanchan_only_genbutsu() {
        let mut game = Game::new(kuikae_names());
        let four = Tile::new_number(Suit::Man, 4, false);
        game.players[3].hand = crate::hand::Hand::new();
        game.players[3].hand.add_tile(Tile::new_number(Suit::Man, 3, false));
        game.players[3].hand.add_tile(Tile::new_number(Suit::Man, 5, false));
        game.players[3].hand.add_tile(four);
        game.players[3].hand.add_tile(Tile::new_number(Suit::Pin, 1, false));
        game.last_discard = Some(four);
        game.last_discard_hidden = false;
        game.last_discarder = Some(0);
        game.current_player = 0;

        // pattern 1: n-1,n,n+1 (= 3m,4m,5m) 嵌張。
        assert!(game.do_chi(3, 1), "345m 嵌張チー成立");
        assert_eq!(game.kuikae_forbidden, vec![four], "嵌張は現物 4m のみ禁止 (筋なし)");
        assert!(!game.discard_tile(four), "現物 4m は打てない");
    }
}
