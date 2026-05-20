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
