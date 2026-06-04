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
STAGE = 720×720、中心 (360, 360)、`DISCARD_INNER_MARGIN = 96`。

| Layer | y range | Notes |
| ----- | ------- | ----- |
| 自家副露 (melds row)          | 631-687 | 手牌底辺 + 8px gap、stage 右端揃え、`fitMeldScale` で stage 内に自動縮小 |
| 自家手牌 (handBaseline = 575) | 547-603 | 13 牌 + ツモ牌 × handSpacing 40px (密着) |
| 自家河 (discards)             | 456-561 | 6 列 × 25px × 3 行 × 35px (scale 0.625、密着) |
| Center info panel             | 264-456 | 局・山残・ドラ・直前打牌 |

**CPU 山と自家手牌の交差を避けるため CPU 用は別 baseline** — CPU の手牌は卓中心から
`cpuHandBaseline = 280px` 離した位置 (上下左右いずれも) に置く (session505 で旧 304 から
24px 縮小。CPU 副露 row が stage 外側余白に 8px gap を持って収まる値)。
自家手牌の右端と右家 CPU 山の x 位置が衝突しないよう、`handSpacing` (自家) と
`cpuHandSpacing` (CPU) を連動して選定する。レイアウトを変える時は
**4 方位 × (手牌 / 河 / 副露) の 12 ブロックすべて**のコーナーが衝突しないか確認する。

### 副露 (鳴き面子) row

副露は手牌・河と独立した row として配置する (Issue #83 / PR #88)。

- **自家 (offset 0)**: 手牌の下に並べ、stage 右端揃え (麻雀牌譜の慣習)。`preferredScale = 0.8`、
  横方向 `fitMeldScale(STAGE_WIDTH - 24)` で stage 内に自動縮小 (下限 0.4)。
- **CPU (offset 1/2/3)**: 手牌の **外側** (stage 端方向) に並べる。横方向 (= 手牌と平行) は
  `fitMeldScale(STAGE_HEIGHT - 24)` で詰め、深さ方向 (= stage 端への張り出し) は
  `fitMeldDepthScale(outerSpace)` で更に縮小して **stage からはみ出さない** ことを保証する。
  ここで `outerSpace = (stage 端) - (手牌外側) - STAGE_EDGE_MARGIN(12) - 8(gap)`。
- **kind 別の見た目**:
  - chi / pon / kakan: 3 スロット並び (face/face/sideways)。sideways 位置は `fromOffset` で決まる
    (上家=左端 / 対面=中央 / 下家=右端)。kakan は sideways の上にさらに 1 枚 face を 90° 回転で
    stack する (旧 Pon の上に加えた 1 枚を乗せる慣習表現)。
  - minkan: 4 スロット並び (face×3 + sideways)。sideways 位置は同じく fromOffset から。
    fromOffset=1 (下家) のときは最右端 (= スロット 3)。
  - ankan: 4 スロット並び、両端 face / 中 2 枚は裏向き (createTileBackGraphics)、回転なし。

**牌の重なりはバグとして扱う。** 副露 row も含めて、4 方位の手牌 / 河 / 副露の
12 ブロックが互いに重ならず、stage 端を越えないことが必須条件。

**Tile Spacing Rule (重要)** — 手牌・CPU 手牌・河の **ピッチは「width × scale」ぴったり** に揃え、
すべての牌を隙間なく密着させる。手牌・CPU 手牌・河でばらつきがあると見た目が散らかるので
意図的に統一する (session 2026-05-22 で 42/26/30 → 40/28/25 へ詰めて密着化)。

xmj の現行値:
- `TILE.handSpacing = 40px` (= width 40、密着)
- `TILE.cpuHandSpacing = 28px` (= width 40 × cpuHandScale 0.7、密着)
- `TILE.discardColPitch = 25px` / `TILE.discardRowPitch = 35px` (= width/height × discardScale 0.625、密着)

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
| 牌 (表) | **Pixi 側で外周 stroke は描かない**。`web/public/fonts/noto-sans-symbols2-mahjong.woff2` (Noto Sans Symbols 2 subset, 12KB) を `@font-face: XmjMahjong` として index.html で登録、`fill` 色で mono glyph を直接着色する。角丸の白面 (= 牌の bbox 内側) を背景として 1 枚仕込み、Container の bbox と hit-area の座標基準を維持。**選択中の牌は外側に halo を描かず、面色を `TILE.selectedFaceColor` (0xf5d96a 黄) に塗り替えて状態を示す** (#98)。fallback フォントは持たない (OS 間で glyph を完全に揃えるため) — `main.ts` で `document.fonts.ready` 待ち |
| 牌 (裏) | 同じ `XmjMahjong` で Unicode 🀫 (U+1F02B) を 1 文字描画。地色は `TILE.backFaceColor` (0xf0e2c0 象牙色) で塗り、表向き牌の白と差別化 (#99)。glyph 色は `TILE.backColor` (竹色) |

**Do**: 卓画面 (game-table) の情報表示は卓に直書きされた札のように見せる。操作可能要素 (ボタン) だけ明確な枠で区別する。
**Don't**: 卓画面の表示要素 (点数・ログ・情報帯) に明るい枠を引いて UI チップ感を出さない。

**例外**: タイトル / モード選択 / 場決め等のオーバーレイ系シーン (titleScene, modeSelectScene, diceRollScene) は卓ではなくダイアログ的な性格を持つので、`PANEL_BORDER_COLOR` の枠を持つフレームを許容する。Frame Minimalism のルールは「卓上に重ねた情報表示は枠を持たない」という範囲で適用する。

### Mobile Touch Targets (操作はサイドパネル末尾 = 右下)

スマホ片手操作を想定し、能動的な操作 UI は画面右下に集める。
**現行実装は Pixi canvas に直書きせず、HTML サイドパネルの末尾 section に配置する。**

- **Action area** (打牌・立直など): `#ui-actions-section` をサイドパネル最下段に置く
- **Event log** (受動情報): 同じサイドパネルだが `flex: 1` で大きく取り、操作の上に積む
- **Hint テキスト**: ボタン外の説明行は持たない。`label` (目的: 例「打牌」) と `hotkey` (操作: 例「[D]」) で完結させる
- **Buttons**: `min-height: 38px`、`font-size: 13px` (Apple HIG / Material 推奨 44px は端末幅により下回ることを許容、その代わり面積総和を固定して 1 個あたりが小さくなる方を優先)
- **ボタン配列**: 1 行 flex (no-wrap) で並べ、ボタン数が増えるほど 1 個あたりの幅が縮む。
  `display: flex; flex-wrap: nowrap` + `flex: 1 1 0` + `min-width: 0`
- 「縦積み」は採用しない (面積が増えると操作領域が広がってしまうため)

### Side Panel UI (HTML overlay)

サイドパネルは Pixi 卓の外。卓自体は felt のみに集中させ、テキスト情報はすべて
HTML overlay に集約する。

- **Status bar** (`#ui-status`): `{場風}{局}局 + {本場}本場 + 親: {名前} + 山残 + ドラ` を 1 行で表示
  - 場風は `round` から導出 (round 1-4 = 東場 / 5-8 = 南場)
  - 本場は 0 でも常時 "0本場" を出す (位置の安定性を優先)
  - 親は `gameState.dealer` で索引、強調色 (`--turn-glow`)
- **Score rows** (`#ui-scores`): `{風}{名前}{点数}{立直?}` を 2 列グリッド
  - **重複表記の禁止**: 名前文字列にすでに風漢字が含まれる場合 (例 "CPU 南") は `.wind` span を空にする
  - 人間プレイヤーには `(あなた)` のような追記は付けない (name 自体が "あなた")
- **Event log** (`#ui-log`): MMORPG チャット欄風
  - 行ごとに発信者を検出 (行頭 `あなた` / `CPU 東/南/西/北` / `東家/...` 等) し、発信者タグ + 本文に分割
  - 発信者ごとに色分け (`.spk-self` / `.spk-east` / `.spk-south` / `.spk-west` / `.spk-north`)
  - システム行はイタリック + 控えめな左罫線
  - 最新行は背景 highlight
- **Actions** (`#ui-actions`): 1 行 flex で全ボタンを並べる (上記 Mobile Touch Targets 参照)

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
| 局ループ     | **WASM bridge 公開済** (Issue #27)。`resolveDraw` / `resolveWinTsumo` / `resolveWinRon` / `nextRound` / `getRound` / `getHonba` / `getDealer` / `getRiichiSticks` / `getLastOutcomeJson` を Web UI から駆動可能。山牌切れで自動 resolveDraw → 中間結果シーン → nextRound 復帰までの最小フローまで実装。 | 和了 UI ボタン (ツモ / ロン宣言)、副露和了 (#33)、待ち形精度 (#34) |
| 場代         | API + `next_round` で各局自動再徴収 (Seikyo モード)              | UI 表示                                |
| 二度ヅモ     | API + CLI 即捨て UX、`resolve_win` で `dealer_won_last` 自動更新 | （配線完了）                           |
| 役満祝儀     | **`resolve_win` 内で自動授受** (Issue #28)。`count_yakuman` で役満数を判定し、ロンは放銃者から / ツモは他家全員から `SEIKYO_YAKUMAN_TIP * yakuman_count` を `pay_yakuman_tip` 経由で移動 (ゼロサム保証)。 | （配線完了）                           |
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
| 役判定の自動配線        | **`resolve_win` 内で自動記録** (Issue #29)。和了者の `ScoringResult.yaku` を全件チームに登録 | （配線完了） |

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

- **役判定の自動配線**: ~~未実装~~ → **Issue #29 で `resolve_win` 内に配線済**。和了確定時に
  `ScoringResult.yaku` を `record_team_yaku` に流し、`east_west_winner()` が逐次更新される。
  ただし `ScoringEngine::calculate_score` 自体が Honroutou を検出しない点 (#19 follow-up) は残っている
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

### Optional Rules & Toggles（全モード共通のオプション）

ゲームモード（Standard / Seikyo / ...）とは独立した、対局中に効くオプションルールと
操作トグル。Web 版は `modeSelectScene` の設定 + 操作ボタン群（右下サイドパネル）に集約する。

#### 嘘リーチ（#89）

「テンパイしていないのにリーチを宣言してしまい、和了できず流局で露見する」状態を再現する
オプション。特殊ルールではなく、一般の麻雀でも（うっかり / 故意に）起こりうる状況。

- 有効時（`setUsoRiichiEnabled(true)`）、`canRiichi` のテンパイ・点数要件を外し、門前 + 未リーチ
  であれば常時リーチ宣言できる。宣言時に非テンパイなら `Player.uso_riichi=true` が立つ。
- **追加の罰符は無い。** 嘘リーチの損失は普通の不成立リーチと完全に同じで、
  1. 宣言時に供託したリーチ棒 1000 点の没収（`riichi_sticks` に積まれ、次局の和了者が回収）
  2. テンパイしていないので流局時のノーテン罰符の支払い（`resolve_draw` の `per_noten`）
  の 2 つで完結する。両方とも標準処理でゼロサムが保たれる。
- 嘘リーチは点数要件（1000 点以上）を外しているため、**持ち点 < 1000 点でも宣言できる**。
  この場合のリーチ棒支払いは `pay_unclamped(1000)` で行い（0 クランプせず負スコアを許容）、
  供託に積む 1000 点と必ず整合させる（#144）。`subtract_score`（0 クランプ）を使うと
  実引き額が供託と食い違い点棒総和が増えてゼロサムが壊れる。負スコアはトビとして標準処理で
  検知される。
- 誰かが先に和了した局では嘘リーチ者の手牌は公開しない（#89 要件 3）。流局時のみ
  `getPlayerHandString` で手牌を公開し、嘘リーチが露見する。
- API: `WasmGame::setUsoRiichiEnabled` / `isUsoRiichiEnabled` / `isUsoRiichi(idx)`。

> 注意: 旧実装（〜PR #119）は流局時に嘘リーチ者から `pay_unclamped(1000)` を**追加徴収**して
> いたが、これは (a) 標準ルールに無い二重罰符 (b) 徴収先が無く点棒総和から 1000 点消滅、という
> 二重のバグだった。現行は追加罰符ブロックを削除済み。

#### 理牌トグル（#80）

ツモ後の手牌自動ソート（理牌）を ON/OFF する。鷲巣麻雀など「手牌の並びで情報を読ませない /
読む」プレイと両立させるため、対戦中いつでも切替可能。

- API: `WasmGame::setAutoSort` / `isAutoSortEnabled` / `sortCurrentHand`。
- 操作: 理牌ボタン（常時表示） / S キー。`auto_sort=false` でも手動で 1 回整列できる。

#### 手動ツモ（#81）

自動ツモを OFF にし、人間が明示的にツモ操作をする。

- API: `WasmGame::setAutoDraw` / `isAutoDrawEnabled`。
- 操作: `autoDraw=false` かつ手牌がツモ前枚数のとき「ツモる」ボタン / T キー。
- follow-up #121: 手動ツモ時に**ツモらず打牌して少牌になる**（ツモ忘れの再現）を予定。
  多牌は手番モデル上再現困難なため対象外。

#### デバッグモード（#79）

`?reveal=1` クエリで CPU の手牌を表向き表示する。鷲巣麻雀の予行演習 / 開発時の挙動確認用。

- API: `WasmGame::getPlayerHandString(idx)`。
- 本番ビルドでもクエリだけで有効になる（ローカル単独プレイ前提）。対人配信を想定する場合は
  本番無効化ガードを検討（follow-up）。

#### 食い替え禁止（#59）

チー / ポン直後の打牌で、鳴いた牌と同種（現物）およびリャンメンチーの反対側（筋）を
切れないようにする。標準ルールでは禁止（デフォルト有効）。

- 現物: ポン / チーで鳴いた牌と同じ牌。リャンメンチー（pattern 0 / 2）はさらに筋牌
  （456 を 56 で鳴いたら 7、234 を 23 で鳴いたら 1）も禁止。嵌張チー（pattern 1）は筋なし。
- `Game.kuikae_forbidden` に鳴き直後の禁止牌を積み、`discard_tile` 冒頭で `tile_type` 比較
  （赤ドラ無視）で拒否。打牌成立・闇牌打牌・局リセットでクリア。
- API: `WasmGame::setEnforceKuikae` / `isEnforceKuikae`（デフォルト true）。
- follow-up: 打牌拒否時のユーザー向けフィードバック表示 / 設定 UI。

#### 喰いタン（#129）

鳴きありの手で断么九（タンヤオ）を認めるか。ruleset 依存（デフォルト有効）。

- `allow_open_tanyao=false` のとき、非門前（鳴きあり）の手では断么九を付与しない。門前手は影響なし。
- `ScoringContext.allow_open_tanyao` 経由で `calculate_score_with_context` に渡す。
- API: `WasmGame::setAllowOpenTanyao` / `isAllowOpenTanyao`（デフォルト true）。

#### 特殊ルール一式（session547 で実装、トグル制御）

標準/ローカルの特殊ルールを一括で入れた。ロジックは実装済み、UI 配線は follow-up。

| ルール | Issue | トグル / API | デフォルト |
|---|---|---|---|
| 食い替え禁止 | #59 | `enforce_kuikae` / `setEnforceKuikae` | true |
| ローカル役満（人和/大車輪/四連刻/百万石/三連刻） | #58 | `allow_local_yakuman` / `setAllowLocalYakuman` | false |
| オープンリーチ | #60 | `declareOpenRiichi` / `isPlayerOpenRiichi`（+1飜） | — |
| 本場縛り（2飜/満貫/役満縛り） | #61 | `ShibariRule` / `setShibariRule(0-3)` | 0=標準 |
| 包（責任払い） | #57 | `enforce_pao` / `setEnforcePao` | true |
| 割れ目 | #118 | `warime_player` / `setWarimePlayer(-1=無効)` | 無効 |
| 特殊流局（四風連打/四家立直/四槓散了/九種九牌/流し満貫） | #55 | `allow_abortive_draws` / `declareKyuushu` / `applyAbortiveDraw(0-2)` / `checkSuufonRenda` 等 | true |
| 差し馬 | #117 | `addSashimaBet(a,b,amount)` → 対局終了時 `settle_sashima` | — |

- **包**: `check_pao_after_call` が大三元(三元3種)/大四喜(風4種)/四槓子(槓4)の確定打牌者を記録。`resolve_win` で対象役満の和了なら責任者がツモ全額/ロン折半。
- **特殊流局**: 検出ヘルパーは pure（テスト可）。流し満貫は河が全て么九 + 無鳴き（`discard_taken_from` で追跡）を `resolve_draw` で満貫和了扱い。途中流局は親連荘・聴牌料なし。
- **割れ目とノーテン罰符 (#145)**: 割れ目は和了の払い/受けだけでなく、流局のテンパイ料/ノーテン罰符にも適用する。`resolve_draw` で割れ目プレイヤーの精算額（テンパイなら受領、ノーテンなら支払い）を 2 倍にし、増分は反対グループ（割れ目がノーテンならテンパイ者、テンパイならノーテン者）へ均等転嫁してゼロサムを維持する。流し満貫成立時はテンパイ料自体が発生しないため割れ目補正も無い。
- **差し馬**: 対局終了時に最終点数が高い方が低い方から賭け金を受け取る（`next_round` の game-over パスで `settle_sashima`、二重精算ガード）。
- follow-up（実機/UI）: 各トグルの modeSelectScene UI、特殊流局の手番ループ自動検出配線、九種九牌ボタン、オープンリーチの手牌公開描画と3択UI、割れ目のサイコロ自動決定。
