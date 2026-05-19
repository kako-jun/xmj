# DESIGN.md

xmj (Xtreme Mahjong) — Design System

## 1. Visual Theme & Atmosphere

The current Web battle view is a PixiJS table scene with a cinematic, pressure-heavy look: dark felt, brass-like trims, and a crimson vignette around the table. DOM overlays such as loading or future menus may still use gradients and glass cards, but the main match surface itself is rendered inside the canvas.

Dual interfaces remain: Web (PixiJS + WASM) and CUI (Rust terminal output). The important rule is unchanged: no bitmap tile assets. Tiles are rendered from vector rectangles plus text glyphs so the look stays lightweight and programmable.

## 2. Color Palette & Roles

### Main Battle (PixiJS table) — Dark Felt + Crimson Vignette

| Color     | Hex       | Usage                     |
| --------- | --------- | ------------------------- |
| Backdrop  | `#050505` | Outer stage background    |
| Felt      | `#003300` | Main table surface        |
| Inner Felt| `#0d2f1d` | Center information area   |
| Brass     | `#8f6a2f` / `#d4b06a` | Frames, accents |
| Crimson   | `#7a0f16` | Tension glow / vignette   |
| Ivory     | `#faf3e0` | Tile face                 |
| Danger    | `#c93a3a` | Riichi / tension text     |

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

| 機能       | 実装レベル                | follow-up                                  |
| ---------- | ------------------------- | ------------------------------------------ |
| 場代       | API + CLI 起動時 1 回供託 | 各局再徴収（局ループ未実装のため未対応）   |
| 二度ヅモ   | API + CLI 即捨て UX       | 連荘フラグ `dealer_won_last` の自動更新    |
| 役満祝儀   | API（ゼロサム保証）       | 役満和了→放銃者特定→自動授受の配線         |
| pot 持ち越し | API（`winner_takes_pot` を呼ばなければ自然持ち越し） | 流局処理ロジック         |

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

#### 現状の実装ステータス

| 機能              | 実装レベル                | follow-up                                |
| ----------------- | ------------------------- | ---------------------------------------- |
| 5 枚配牌          | API + CLI 起動時の自動配牌 | -                                        |
| 和了判定          | API（`can_win_five_tile`） | -                                        |
| テンパイ判定      | API（`is_tenpai_five_tile`） | -                                       |
| タンヤオ点数      | API（`score_five_tile`） | -                                        |
| 字牌特殊効果      | 未実装                    | カード化として別 Issue                   |
| ドラエスカレート  | 未実装                    | 別 Issue                                 |
| ボスステージ      | 未実装                    | 別 Issue                                 |
| イカサマ要素      | 未実装                    | 別 Issue                                 |

#### API（`src/hand.rs`）

- `Hand::can_win_five_tile(&self, winning_tile: &Tile) -> bool` — 5枚麻雀の和了判定
- `Hand::is_tenpai_five_tile(&self) -> bool` — 5枚麻雀のテンパイ判定（5 枚で 34 種を試す）

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

5 枚麻雀の現状は「最低限の動線（配牌 + 和了判定 + タンヤオ点数）」までで、
以下は follow-up Issue で対応する:

- 字牌の特殊効果（カード化）
- ドラエスカレート（複数局でドラが累積）
- ボスステージ（特殊 CPU 戦）
- イカサマ要素（積み込み・牌のすり替え）
- Web UI

