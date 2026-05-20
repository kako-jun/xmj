# DESIGN.md

xmj (Xtreme Mahjong) — Design System

## 1. Visual Theme & Atmosphere

The current Web battle view is a PixiJS table scene with a cinematic, pressure-heavy look: dark felt, brass-like trims, and a crimson vignette around the table. DOM overlays such as loading or future menus may still use gradients and glass cards, but the main match surface itself is rendered inside the canvas.

Dual interfaces remain: Web (PixiJS + WASM) and CUI (Rust terminal output). The important rule is unchanged: no bitmap tile assets. Tiles are rendered from vector rectangles plus text glyphs so the look stays lightweight and programmable.

## 2. Color Palette & Roles

### Main Battle (PixiJS table) — Muted Felt + Soft Vignette

長時間プレイで目が疲れない明度・彩度に揃える。原色の緑・赤・青は避け、
くすませた中間トーンに統一する。Crimson vignette は alpha 0.07 程度の
かすかな滲み (画面外周のテンション暗示) として使う。

| Color     | Hex       | Usage                     |
| --------- | --------- | ------------------------- |
| Backdrop  | `#0a0a0a` | Outer stage background    |
| Felt      | `#1f3a2a` | Main table surface (muted green) |
| Inner Felt| `#1a3024` | Center information area   |
| Brass     | `#7a6038` / `#b39a6e` | Frames, accents (muted gold) |
| Crimson   | `#4a1f24` | Tension glow / vignette (alpha 0.07) |
| Ivory     | `#f3ead2` | Tile face (warm off-white) |
| Danger    | `#b84a4a` | Riichi / tension text     |
| Tile Sou  | `#2f6b3a` | 索子 (muted forest green) |
| Tile Pin  | `#365a85` | 筒子 (muted slate blue)   |
| Tile Back | `#33445e` | 裏向き (muted indigo)     |
| Tile Red Dora | `#a83a3a` | 赤ドラ                |

### Hybrid / Legacy DOM screens — Gradient + Glass

| Color     | Hex       | Usage                     |
| --------- | --------- | ------------------------- |
| Gradient  | `#1a1a2e → #16213e` | Background for legacy/overlay screens |
| Sky Blue  | `#4facfe` | Primary elements, CPU log |
| Cyan      | `#00f2fe` | Active highlights         |
| Title     | `#f093fb → #f5576c` | Title text gradient |
| Green     | `#4cd137` | Success, player log       |
| Red/Pink  | `#f5576c` | Riichi state              |
| Orange    | `#ffa502` | System log messages       |

### Debug Mode (debug.html) — Retro Terminal

| Color     | Hex       | Usage                     |
| --------- | --------- | ------------------------- |
| Black     | `#1a1a1a` | Background                |
| Lime      | `#0f0`    | Primary text, success     |
| Cyan      | `#0ff`    | Secondary text, info      |
| Red       | `#f00`    | Error                     |
| Yellow    | `#ff0`    | Warning                   |

Text shadow glow: `0 0 10px #0f0` on primary text.

## 3. Typography Rules

### Font Families

| Context          | Family                                             |
| ---------------- | -------------------------------------------------- |
| UI text          | `"Segoe UI", Tahoma, Geneva, Verdana, sans-serif`  |
| Tiles & data     | Sans-serif glyph text inside vector tiles          |

### Type Scale

| Element           | Size    | Weight | Notes                    |
| ----------------- | ------- | ------ | ------------------------ |
| Page title (h1)   | 2–2.5rem | 400  | Text shadow for depth    |
| Section header    | 1.2–1.3em | 400 |                          |
| Subsection        | 1.1–1.2em | 400 |                          |
| Body/card text    | 1rem    | 400    |                          |
| Small labels      | 0.9rem  | 400    |                          |
| Tile display      | 1.1–1.3rem | 700 | Vector tile + text glyph |

### Text Effects

- Depth shadow: `2px 2px 4px rgba(0,0,0,0.5)` on headers
- Terminal glow: `0 0 10px #0f0` (debug page)
- Gradient clip text: `-webkit-background-clip: text` for title

## 4. Component Stylings

### Glass Cards

Use these for DOM overlays, loading states, or future title/result panels. Do not force the match table itself into glassmorphism; the current canvas battle scene is intentionally heavier and more analog.

```css
background: rgba(255,255,255,0.1);
backdrop-filter: blur(10px);
border-radius: 12px;
padding: 2rem;
box-shadow: 0 8px 32px rgba(0,0,0,0.3);
border: 2px solid rgba(255,255,255,0.2);
```

### Buttons

- Background: solid or gradient color
- Hover: `translateY(-2px)` + enhanced shadow
- Active: return to default position
- Disabled: `opacity: 0.5`
- Transition: `all 0.3s`

### Input Fields

```css
border: 2px solid rgba(255,255,255,0.3);
background: rgba(255,255,255,0.1);
color: white;
border-radius: 6px;
padding: 0.8rem;
```

Focus: border-color becomes primary, glow effect added.

### Tile Display

- No bitmap images for tiles
- Use PixiJS Graphics for the face/back silhouette
- Place text glyphs on top for number / honor / suit marks
- Keep colors semantic: man/honor black, pin blue, sou green, red dora red

### Status Indicators

- Green dot with `animation: pulse 2s infinite` for active states
- Box-shadow glows on active elements

## 5. Layout Principles

### Container

- Max width: `1400px`
- Padding: `1.5–2rem`

### Canvas Composition

- Main table uses a fixed 16:9 composition (`1280x720`) centered in the viewport
- Four seats wrap the center information plate rather than using DOM grid cards
- DOM grids remain acceptable for setup, result, and debug screens

### Mahjong Table Stacking (Human Player, Bottom Edge)

下から上に向かって以下の順で重ねる。各帯は y 範囲を排他にして牌の重なりを禁ずる。

| Layer | y range | Notes |
| ----- | ------- | ----- |
| Footer shadow strip | 702-720 | 視覚的底辺、操作物は置かない |
| Hand (自家手牌)     | 612-682 | 13 牌 × handSpacing 54px (重なり禁止) |
| Self discards (河)  | 450-595 | 6 列 × 36px × 3 行 × 50px |
| Center info panel   | 253-443 | 局・山残・ドラ・直前打牌 |

**Tile Spacing Rule (重要)** — 手牌の隣接牌中心間ピッチは必ず `TILE.width` 以上にする。
xmj の `TILE.handSpacing` は 54px (width 50 + gap 4)。河 (捨牌) の列ピッチは
`TILE.discardColPitch = 36px` (scale 0.62 → 実効 width 31)、行ピッチ
`TILE.discardRowPitch = 50px` (scale 0.62 → 実効 height 43.4) で `+ gap` を確保する。
**牌の重なりはバグとして扱う。**

### Frame Minimalism (枠は必要なところだけ)

「卓に札を置いた」リアリティを優先し、情報表示パネルは枠を極力持たない。

| 要素 | 枠の扱い |
| ---- | -------- |
| スコアバッジ (各家の名前・点数) | **枠なし**。風牌 + 名前 + 点数のテキストだけステージ四隅に置く |
| 手番マーカー | **枠なし**。`TURN_GLOW_COLOR` の小さな丸印 + テキスト |
| 中央情報帯 (東1局・山牌枚数・ドラ表示) | 影だけ (`SHADOW_COLOR` alpha 0.22 の薄いラウンド矩形)、ストロークなし |
| 対局ログ | 影だけ (`SHADOW_COLOR` alpha 0.32)、ストロークなし |
| 操作 UI (行動エリア) 外周 | 影だけ (`SHADOW_COLOR` alpha 0.34)。**個別のボタン**は枠ありで押せると分かる見た目を維持 |
| 牌 | 縁取りあり (`TILE.edgeColor`) — 牌そのものは物理オブジェクトとして強調 |

**Do**: 卓画面 (game-table) の情報表示は卓に直書きされた札のように見せる。操作可能要素 (ボタン) だけ明確な枠で区別する。
**Don't**: 卓画面の表示要素 (点数・ログ・情報帯) に明るい枠を引いて UI チップ感を出さない。

**例外**: タイトル / モード選択 / 場決め等のオーバーレイ系シーン (titleScene, modeSelectScene, diceRollScene) は卓ではなくダイアログ的な性格を持つので、`PANEL_BORDER_COLOR` の枠を持つフレームを許容する。Frame Minimalism のルールは「卓上に重ねた情報表示は枠を持たない」という範囲で適用する。

### Mobile Touch Targets (右下集約)

スマホ片手操作を想定し、能動的な操作 UI は画面右下に集める。

- **Action area** (打牌・立直など): `x = STAGE_WIDTH - 220 - 24`, `y = 底辺 - 24 - height`
- **Turn marker** (あなたの手番): action area の真上 (`y = actionY - 44`)
- **Event log** (受動情報): 左下、`x = 24`, `y = 底辺 - 104 - 24` で操作 UI と分離
- **Buttons**: 最小高さ 52px (Apple HIG / Material タッチターゲット推奨 44px+)
- ボタンは横並びではなく**縦積み**にする (親指で当てやすい)
- ラベルは 18px 太字

### Spacing

Standard gap: `10–20px`. Generous padding throughout.

## 6. Depth & Elevation

### Layers

- Stage backdrop / vignette (lowest)
- Felt table and discard areas (middle)
- Score badges / center info / turn markers (top UI)
- DOM overlays such as loading and modal panels may sit above canvas when needed

### Shadows

- Cards: `0 8px 32px rgba(0,0,0,0.3)`
- Hover buttons: enhanced shadow + translateY

### Border Radius

| Component    | Radius |
| ------------ | ------ |
| Glass cards  | 12px   |
| Inputs       | 6px    |
| Buttons      | varies |

## 7. Do's and Don'ts

### Do

- Use vector tile rendering with PixiJS Graphics + text; no bitmap tile images
- Reserve gradients and glass cards for DOM overlays or non-table pages
- Keep the battle table dark, tense, and legible from all four sides
- Use `translateY(-2px)` hover effect on buttons
- Use consistent transition: `all 0.3s`
- Color-code log messages: green (player), blue (CPU), orange (system)
- Use emoji labels for buttons and status indicators

### Don't

- Reintroduce bitmap tile sheets or photoreal tile textures
- Force the PixiJS battle scene into a bright purple gradient theme
- Apply debug-page terminal styling (green glow) to the battle screen
- Flatten everything into generic material-style cards

### Animations

| Animation  | Duration | Timing  | Usage              |
| ---------- | -------- | ------- | ------------------ |
| `spin`     | 1s       | linear  | Loading spinner    |
| `pulse`    | 2s       | —       | Active indicator   |
| Hover      | 0.3s     | —       | Button effects     |

## 8. Responsive Behavior

### Current Web Rule

- Keep the table composition intact on desktop and mobile; scale the canvas rather than reflowing the four-seat layout into unrelated cards
- Preserve `width=device-width, initial-scale=1.0`
- If auxiliary controls need mobile treatment, move those controls around the canvas instead of rewriting the table geometry

## 9. Agent Prompt Guide

### Tile Notation Reference

```
Number tiles: [value][suit]     e.g. 1m, 5p, 9s
Red dora:     [value][suit]r    e.g. 5mr, 5pr, 5sr
Honors:       to(East) na(South) sa(West) pe(North)
              hk(White) ht(Green) cn(Red)
```

### Page Color Identities

```
Main battle:  dark felt + brass + crimson vignette
Hybrid mode:  #1a1a2e → #16213e  (legacy / overlay gradient)
Debug:        #1a1a1a             (terminal black)
```

### When generating UI for this project

- Glass card pattern is for DOM overlays, not the battle table itself
- Tile rendering is vector-based: Graphics body + text glyphs, no bitmap assets
- The battle table uses felt, brass, and vignette rather than bright page gradients
- Log messages are color-coded by source (green/blue/orange)
- Buttons lift on hover (`translateY(-2px)`) with shadow enhancement
- Transition everything at `0.3s`
- Emoji are used liberally for UI labels and status

### Color Emotion Reference

- **Purple (#667eea):** Strategic, focused, competitive
- **Emerald (#10b981):** Action, go, positive outcome
- **Cyan (#00f2fe):** Information, clarity, data
- **Red (#ef4444):** Danger, riichi declaration, critical action
- **Lime (#0f0):** Retro, developer mode, raw data

## 10. Rule Specifications (Game Modes)

### Standard

- 標準的なリーチ麻雀。初期点数 25000、全員から場代徴収なし、祝儀なし。
- API: `Game::new(names)` または `Game::new_with_mode(names, GameMode::Standard)`。

### Seikyo（誠京麻雀 / 『天』『アカギ』）

| 項目     | 値         | 内容                                                              |
| -------- | ---------- | ----------------------------------------------------------------- |
| 場代     | **1000点** | 各局開始時に全員から徴収。和了者が `pot` を回収。流局時は持ち越し |
| 役満祝儀 | **8000点** | 役満和了時、放銃者（または振り込み相手）から追加で授受            |
| 二度ヅモ | 親限定     | 前局親和了（連荘）時、親は 2 枚ツモして 1 枚捨てる                |

#### 現状の実装ステータス

PR #21 時点では **API 提供レベル**まで。本番ゲームフローへの自動配線は follow-up Issue。

| 機能         | 実装レベル                                                       | follow-up                              |
| ------------ | ---------------------------------------------------------------- | -------------------------------------- |
| 局ループ     | **API 実装済** (`resolve_win` / `resolve_draw` / `next_round`)   | WASM bridge への公開と Web UI 連携     |
| 場代         | API + `next_round` で各局自動再徴収 (Seikyo モード)              | UI 表示                                |
| 二度ヅモ     | API + CLI 即捨て UX、`resolve_win` で `dealer_won_last` 自動更新 | （配線完了）                           |
| 役満祝儀     | API（ゼロサム保証）                                              | 役満和了→放銃者特定→自動授受の配線     |
| pot 持ち越し | API + `next_round` で連荘・流局時に保持                          | （配線完了）                           |
| 本場         | API (`Game.honba`、和了で `HONBA_BONUS * honba` 加算)            | UI 表示                                |
| 供託リーチ棒 | API (`Game.riichi_sticks`)、和了者が取得・流局で持ち越し         | リーチ宣言 → `riichi_sticks++` の配線  |

#### API（`src/game.rs`）

- `GameMode::Seikyo` — モード識別子
- `Game.pot: i32` — 供託の合計
- `Game.dealer_won_last: bool` — 前局親和了フラグ（**外部から win-resolve 時に手動更新**）
- `Game::new_with_mode(names, GameMode::Seikyo)` — 構築
- `Game::collect_seat_fee(amount)` — 全員から `amount` 徴収して pot へ。標準は `SEIKYO_SEAT_FEE`
- `Game::winner_takes_pot(winner_idx) -> i32` — pot を winner に渡してリセット
- `Game::dealer_double_draw() -> Option<(Tile, Tile)>` — 親二度ヅモ（**打牌は呼び出し側責務**）

#### API（`src/player.rs`）

- `Player::pay_yakuman_tip(amount)` — 役満祝儀の支払い（放銃者）。**ゼロサム保証のため 0 クランプしない**（マイナス許容）
- `Player::receive_yakuman_tip(amount)` — 役満祝儀の受け取り（和了者）

#### CLI

```bash
cargo run -- --mode seikyo
```

起動時に「場代 1000 点ずつ供託しました（pot: 4000 点）」が表示される。
親かつ連荘中のターンでは「親二度ヅモ可能」と案内され、2 枚ツモ後に即捨てる牌を選ぶ（デフォルトは 1 枚目を即捨て）。

未知のモード値や `--mode` の値なしは warning 出力後に Standard へフォールバックする。

### Washizu（鷲巣麻雀 / 『アカギ』）

| 項目     | 値          | 内容                                                                  |
| -------- | ----------- | --------------------------------------------------------------------- |
| 透明牌   | **3/4 (102/136)** | 山牌初期化時にシャッフル後の先頭 3/4 を `is_glass=true` にする  |
| 可視性   | 自家全可視 + 他家 glass のみ | 不透明な 1/4 は他家から見えない                                |
| ドラ表示牌 | 常に opaque | wall 末尾から取るため、シャッフル後の先頭 3/4 塗り実装上ドラ表示牌は必ず非 glass になる |

#### 同値性の扱い

`is_glass` は**表示属性**であり、`Tile` の `PartialEq` / `Eq` / `Hash` には含めない。
理由: 「9m の透明牌」と「9m の不透明牌」は同じ牌として和了判定・鳴き判定で扱う必要があるため、
`is_glass` を比較に入れると既存の `Hand::can_win` / `can_pon` / `can_kan` が壊れる。
`Tile` は `derive(PartialEq/Eq/Hash)` をやめて手動実装している。

#### 現状の実装ステータス

| 機能            | 実装レベル                            | follow-up                     |
| --------------- | ------------------------------------- | ----------------------------- |
| 透明牌の生成    | API + CLI 起動時に 3/4 自動 glass 化 | -                             |
| 他家の glass 可視 | API + CLI 表示                      | Web (PixiJS) 側の glass 描画 |
| 血液ポイント    | 未実装                                | 別 Issue                      |
| 牌の使用回数制限 | 未実装                                | 別 Issue                      |

#### API（`src/tile.rs`）

- `Tile.is_glass: bool` — 透明牌フラグ（pub フィールド、同値比較に含まれない）
- `Tile::with_glass(bool) -> Tile` — ビルダー（`true` で透明化、`false` は試験用）

#### API（`src/game.rs`）

- `GameMode::Washizu` — モード識別子
- `Game::new_with_mode(names, GameMode::Washizu)` — 構築（自動で 3/4 glass 化）
- `Game::get_visible_tiles_of_opponent(observer_idx, target_idx) -> Vec<Tile>` — 観測者から見た対象手牌
  - 自分自身（observer == target）: 全手牌
  - Washizu × 他家: glass 牌のみ
  - 非 Washizu × 他家: 空ベクタ

#### CLI

```bash
cargo run -- --mode washizu
```

起動時に「ルール: 鷲巣麻雀（3/4 透明牌、他家の glass 牌が見える）」が表示される。
各他家の行の下に `[CPUx の透明牌: 9m 7p 9p to na]` のように glass 牌のリストが追記される。
自分の手牌は従来通り全表示（自家は全可視のため）。

### FiveTile（5枚麻雀 / クライマックスだけ麻雀）

| 項目     | 値          | 内容                                                           |
| -------- | ----------- | -------------------------------------------------------------- |
| 配牌     | 子 5 / 親 6 | 親はツモ番が回った状態でスタート                               |
| 和了形   | 雀頭 + 面子1組 | 順子 or 刻子 1 組 + 雀頭 1 組 = 5 枚                        |
| 役       | タンヤオのみ | 手牌 + アガリ牌すべて 2-8 の数牌で +1000 点                  |
| 基礎点   | 1000        | 和了で必ず加算（タンヤオ込みで最大 2000）                      |

「麻雀のクライマックスだけゲーム」。押し引き・読み合いに特化した短時間モード。

#### 和了形の定義（重要）

5 枚麻雀の和了形は「雀頭(2) + 面子(3) = 5 枚を使い切る」形。
通常麻雀と異なり、**アガリ牌は必ず和了形の構成牌の 1 つ**として手牌 5 枚に含まれている必要がある。
これにより「既に和了形を含む手牌に対して、関係ない捨て牌でロン成立」というバグを防ぐ。

具体的には:

- 手牌 5 枚 = ツモ後 / ロン後の状態（取り込み済み）
- 手牌 4 枚 = 打牌後の状態（テンパイ判定の対象）
- 待ち牌 = 手牌 5 枚から 1 枚捨てた残り 4 枚に加えると完成形になる 1 枚

#### 現状の実装ステータス

| 機能              | 実装レベル                | follow-up                                |
| ----------------- | ------------------------- | ---------------------------------------- |
| 5 枚配牌          | API + CLI 起動時の自動配牌 | -                                        |
| 和了判定          | API（`can_win_five_tile`） | -                                        |
| テンパイ判定      | API（`is_tenpai_five_tile`） | -                                       |
| 待ち牌計算        | API（`five_tile_waits`）   | -                                        |
| タンヤオ点数      | API（`score_five_tile`） | -                                        |
| ツモ宣言 UI       | CLI で `tile_count == 5` 時に分岐 | Web UI は follow-up               |
| ロン宣言自動配線  | 未実装（API 単体は動作）  | `can_someone_win` の本格運用は別 Issue   |
| 字牌特殊効果      | 未実装                    | カード化として別 Issue                   |
| ドラエスカレート  | 未実装                    | 別 Issue                                 |
| ボスステージ      | 未実装                    | 別 Issue                                 |
| イカサマ要素      | 未実装                    | 別 Issue                                 |

#### API（`src/hand.rs`）

- `Hand::can_win_five_tile(&self, winning_tile: &Tile) -> bool` — 5 枚麻雀の和了判定。
  手牌 5 枚に `winning_tile` が含まれ、かつ 5 枚が「雀頭 + 面子」で完成形のとき true。
- `Hand::is_tenpai_five_tile(&self) -> bool` — 5 枚麻雀のテンパイ判定（打牌後相当）。
  手牌 5 枚から 1 枚捨てた残り 4 枚に対し、待ち牌が 1 つ以上存在するとき true。
- `Hand::five_tile_waits(&self) -> Vec<Tile>` — 5 枚麻雀の待ち牌候補リスト。
  鳴きなし・手牌 5 枚前提。

#### API（`src/scoring.rs`）

- `scoring::score_five_tile(hand: &Hand, win_tile: &Tile) -> i32` — 簡易点数計算（基礎点 + タンヤオ）

#### API（`src/player.rs`）

- `Player::can_win_with_mode(&self, tile: &Tile, mode: GameMode) -> bool` — モード別和了判定ディスパッチ

#### CLI

```bash
cargo run -- --mode five-tile
cargo run -- --mode five_tile
cargo run -- --mode fivetile
```

起動時に「ルール: 5枚麻雀（クライマックスだけ麻雀）」が表示される。
親は 6 枚、子は 5 枚で配牌される（既存の打牌・ツモループはそのまま動作）。

#### Limitations

5 枚麻雀の現状は「最低限の動線（配牌 + 和了判定 + タンヤオ点数 + CLI ツモ宣言）」までで、
以下は follow-up Issue で対応する:

- ロン宣言の自動配線（`Game::can_someone_win` は `can_win_five_tile` を呼ぶが、
  捨て牌から相手手牌に取り込んで判定する UI フローは未配線）
- ツモ宣言フローの本格対応（現状は最後にツモった 1 枚を構成牌として扱う簡易判定）
- 字牌の特殊効果（カード化）
- ドラエスカレート（複数局でドラが累積）
- ボスステージ（特殊 CPU 戦）
- イカサマ要素（積み込み・牌のすり替え）
- Web UI

### EastWest（東西戦 / クリア麻雀、『天』チーム戦）

| 項目         | 値                  | 内容                                                                 |
| ------------ | ------------------- | -------------------------------------------------------------------- |
| チーム構成   | 東/西 の 2 チーム   | 東家(座席0) + 西家(座席2) = East / 南家(座席1) + 北家(座席3) = West |
| クリア対象役 | 指定二翻役 5 種     | 三色同順 / 一気通貫 / 対々和 / 全帯么 / 混老頭                       |
| 勝利条件     | チームとしての先取  | 5 種を先に全て揃えたチームの勝利。点数計算は副次的                   |

『天』『アカギ』に登場するチーム戦ルール。座席名（東家=ton, 南家=nan, 西家=shaa, 北家=pei）と
チーム名（East/West）は別概念であることに注意。座席→チームの対応は `team_of()` に集約。

#### 現状の実装ステータス

| 機能                    | 実装レベル                          | follow-up                                  |
| ----------------------- | ----------------------------------- | ------------------------------------------ |
| Team enum / 座席マッピング | API（`team_of`）                  | -                                          |
| 進捗 HashMap            | `Game.team_progress`                | -                                          |
| 役登録 / クリア判定     | API（`record_team_yaku` 等）        | -                                          |
| CLI 表示                | 各局のゲーム状態に進捗 1 行 + 勝敗  | Web UI は follow-up                        |
| 役判定の自動配線        | 未実装（API 単体は動作）            | 和了 → 役検出 → `record_team_yaku` の本配線 |

#### API（`src/game.rs`）

- `GameMode::EastWest` — モード識別子
- `Team::{East, West}` — チーム enum
- `team_of(seat_idx: usize) -> Team` — 座席 → チーム変換ヘルパー
- `east_west_target_yaku() -> [Yaku; 5]` — クリア対象 5 役（並び順は CLI 表示・テスト assert で安定）
- `Game.team_progress: HashMap<Team, HashSet<Yaku>>` — チーム別クリア進捗
- `Game::record_team_yaku(winner_seat, yaku)` — 和了者のチームに役を 1 件登録（EastWest 以外は no-op、重複は HashSet で吸収）
- `Game::team_clear_progress(team) -> Vec<Yaku>` — `east_west_target_yaku()` の並びでソートされた進捗
- `Game::is_team_cleared(team) -> bool` — 5 役全て揃ったか
- `Game::east_west_winner() -> Option<Team>` — どちらかが cleared なら Some。両方同時成立は East 優先（決定論的）
- `Game::is_game_over()` は EastWest モードでは winner 確定時に true を返す

#### Yaku enum 拡張（`src/scoring.rs`）

- `Yaku::Honroutou` を二飜役に追加（既存の SanshokuDoujun / Ittsu / Toitoi / Chanta と合わせて 5 役分が揃った）
- `Yaku` には `Hash` を追加（`HashSet<Yaku>` で進捗管理するため）

#### CLI

```bash
cargo run -- --mode east-west
cargo run -- --mode east_west
cargo run -- --mode eastwest
```

起動時に「ルール: 東西戦（クリア麻雀）」と「クリア対象役: 三色同順 / 一気通貫 / 対々和 / 全帯么 / 混老頭」が表示される。
ゲーム状態には毎ターン以下のような進捗行が追加される:

```
東チーム: [✓三色同順, _一気通貫, _対々和, _全帯么, _混老頭]
西チーム: [_三色同順, _一気通貫, _対々和, _全帯么, _混老頭]
```

どちらかのチームが 5 役を揃えると `east_west_winner()` が Some を返し、メインループが終了して
「東チーム勝利！（東家+西家）」または「西チーム勝利！（南家+北家）」が表示される。

#### Limitations

PR #19 時点では「Team / Yaku enum + API + CLI 表示 + テスト」までの最低限の動線。以下は follow-up:

- **役判定の自動配線**: 実際に三色同順・一気通貫・対々和・全帯么・混老頭 の各役を和了から検出して
  `record_team_yaku` を呼び出す配線は未実装。現状は API 単体で動作（テスト・外部呼び出し前提）。
  通常モードの scoring 経路に EastWest 用フックを差し込む必要がある
- **チーム間の協力 UX**: 「自チームの未取得役を狙う」誘導表示や、味方が和了した直後に役一覧を見せる演出は未実装
- **得点ルール**: 通常の点数計算（誰が何点取ったか）は EastWest でも動作するが、勝敗はクリア進捗のみで決まる。
  「点数で勝ったがクリアで負け」のような副次表示は CLI には現状無い
- **Web UI**: WASM 側は follow-up

### Yamima（闇麻 / 闇牌・照射）

| 項目     | 値          | 内容                                                                  |
| -------- | ----------- | --------------------------------------------------------------------- |
| 闇牌打牌 | **1000点**  | 打牌を裏向き（種類非公開）で河に置く。他家からは `??` として見える    |
| 照射     | **500点**   | 観測者が支払って他家の闇牌を 1 枚公開する（必ず公開、空振りなし）     |
| 鳴き・ロン | **闇牌は全て不可** | 照射で公開してからでないと判定できない（仕様）                  |
| 和了判定 | 実体牌として扱う | フリテン判定で河を走査する際は `Player::discards_tiles()` を使う |

闇牌は「自分の打牌が他家から読まれることを 1000 点で買い切る」防御的アクション、
照射は「相手の不審な打牌に 500 点を払って確証を得る」攻撃的アクション。
点数の出入りが牌譜に対称に乗るので、初心者でも「相手が闇牌を切ったらこちらは照射した方が得か？」の
シンプルなトレードオフで戦える設計。

#### 河の構造変更（破壊的変更）

`Player.discards` の型を **`Vec<Tile>` → `Vec<Discard>`** に変更。
`Discard { tile: Tile, is_hidden: bool }` で構成され、照射成立時に
`is_hidden` だけが false に書き換わる。`tile` は常に実体牌として保存される。

互換性のため `Player::discards_tiles() -> Vec<Tile>` を追加し、
既存の「河を Tile のリストとして見たい」読み出し（フリテン判定など）はこのラッパー経由で吸収する。
`get_discards_string()` は闇牌を `??` でマスクして表示する。

#### 現状の実装ステータス

| 機能              | 実装レベル                          | follow-up                                  |
| ----------------- | ----------------------------------- | ------------------------------------------ |
| Discard 構造体    | API（`Vec<Discard>`）               | -                                          |
| 闇牌打牌          | API + CLI `?` プレフィックス         | -                                          |
| 照射              | API（`Game::light_up`）             | CLI コマンド配線                           |
| 鳴き・ロンのゲート | `last_discard_hidden` で 4 関数を制御 | -                                          |
| 河表示            | `??` マスク表示                      | Web UI（PixiJS 側の裏向き描画）             |

#### API（`src/player.rs`）

- `Discard { tile, is_hidden }` — 河の 1 要素
- `Player.discards: Vec<Discard>` — 河（破壊的変更）
- `Player::discard_hidden(tile) -> bool` — 1000 点支払って闇牌で河に追加
- `Player::reveal_discard(idx) -> Option<Tile>` — 該当河を公開（既公開なら None）
- `Player::discards_tiles() -> Vec<Tile>` — Tile のみ抽出（互換ラッパー）
- `Player::get_discards_string()` — 闇牌を `??` で表示

#### API（`src/game.rs`）

- `GameMode::Yamima` — モード識別子
- `YAMIMA_HIDDEN_COST = 1000` / `YAMIMA_LIGHT_UP_COST = 500`
- `Game.last_discard_hidden: bool` — 直近打牌が闇牌か（鳴き・ロンの可否判定で参照）
- `Game::discard_hidden_tile(tile) -> bool` — 闇牌打牌 + `next_player`
- `Game::light_up(observer, target, idx) -> Option<Tile>` — 照射成立で 500 点支払い + 公開
- `Game::can_someone_win` / `can_pon` / `can_chi` / `can_kan` — `last_discard_hidden==true` なら必ず false / 空

#### CLI

```bash
cargo run -- --mode yamima
```

起動時に「ルール: 闇麻（闇牌 1000 / 照射 500）」と説明が表示される。
打牌入力で `?` プレフィックスを付けると闇牌打牌:

```
打牌する牌を入力してください (例: 1m / 闇牌は ?1m): ?6m
[闇麻] 闇牌打牌（1000 点支払い）
 親 あなた (24000点): 3m 9m 1p ...
  河: ??
```

#### Limitations

PR #18 時点では「Discard 構造 + 闇牌・照射 API + CLI 闇牌打牌 + 鳴きゲート」までの最低限の動線。以下は follow-up:

- **照射 CLI コマンド未実装**: 照射は API のみ提供。CLI からの UX（誰のどの河を照射するかの対話）は未配線
- **闇牌対象の鳴き・ロンは仕様上不可**: 「先に照射してから判定」が運用ルール。照射後に
  鳴き再開する仕様（`last_discard_hidden` を後追いで下ろす）は将来検討
- **Web UI**: PixiJS 側で `??` 裏向き牌の描画は未対応
- **AI 配線**: CPU は闇牌打牌・照射を判断しない（常に通常打牌）


### RealTime（リアルタイム麻雀）

『天』『アカギ』の極限緊張感を「同時打牌 + 5 秒タイムアウト」で再現する非ターン制モード。
本実装は Rust core のロジック層のみで、完全な同時打牌入力ループは web/wasm follow-up。

#### ルール

| 項目         | 値             | 意味                                                                    |
| ------------ | -------------- | ----------------------------------------------------------------------- |
| ターン制     | **廃止**       | 全プレイヤーが独立して「ツモ→打牌」を繰り返す                          |
| 制限時間     | **5000ms**     | 各プレイヤーごと。`PlayerTimer::default_limit()` 定数                  |
| タイムアウト | **自動ツモ切り** | `auto_discard_for(idx)` で手牌末尾を河へ                                |
| 鳴き優先順位 | **Ron > Pon > Kan > Chi** | 同優先は先勝ち。`resolve_calls(&[Call])` で 1 件採用                    |

#### 主要 API

- `realtime::CallKind { Ron, Pon, Kan, Chi }` — `Ord` 実装で優先順位（数値小ほど強い）
- `realtime::Call { player_idx, kind }` — 1 件の鳴き宣言
- `realtime::resolve_calls(&[Call]) -> Option<Call>` — 同フレームの宣言から 1 件採用
- `realtime::PlayerTimer { elapsed_ms, limit_ms }` — タイマー状態。`tick` / `is_timeout` / `reset`
- `realtime::should_auto_discard(elapsed_ms, limit_ms) -> bool`
- `realtime::pick_auto_discard_tile(tiles) -> Option<Tile>` — 自動打牌対象（手牌末尾）
- `Game::tick_timers(delta_ms)` — 全プレイヤータイマーを進める
- `Game::timed_out_players() -> Vec<usize>` — タイムアウト中の idx 一覧
- `Game::auto_discard_for(idx) -> Option<Tile>` — タイムアウトプレイヤーの自動ツモ切り
- `Game::reset_player_timer(idx)` — 打牌成功後に呼ぶ
- `Game::resolve_pending_calls(&[Call]) -> Option<Call>` — `realtime::resolve_calls` ラッパー

#### CLI

```bash
cargo run -- --mode realtime
# or --mode real-time / --mode real_time
```

起動時に「ルール: リアルタイム麻雀（全員同時打牌、5 秒タイムアウト）」+ 鳴き優先順位 +
「CLI 版は同期入力のため完全な同時打牌は web/wasm follow-up」が表示される。

#### Limitations

PR #20 時点では Rust core ロジック層 + CLI 起動メッセージまで。以下は follow-up:

- **CLI 同時打牌不可**: Rust 標準の同期 I/O ではタイムアウト付き stdin 読み取りが不可能。
  実時間で進めるには `crossterm` の poll や tokio の `select!` が必要
- **タイマーの実時間進行**: 呼び出し側 (web `requestAnimationFrame` / wasm async / CLI ループ) が
  周期的に `tick_timers(delta_ms)` を呼ぶ責務。本 PR は提供しない
- **WebRTC 鳴きシグナリング**: P2P 経路で鳴き宣言を即時伝搬する配線は別 Issue
- **AI 配線**: CPU は通常モードの逐次打牌のみ。RealTime での非同期思考は未実装
