# DESIGN.md

xmj (Xtreme Mahjong) — Design System

## 1. Visual Theme & Atmosphere

Modern glassmorphism with gradient backgrounds. Each page has a distinct color identity but shares a common design language: frosted glass panels, bold gradients, and text-based tile rendering. The Web UI feels like a stylish arcade cabinet; the CUI feels like a retro terminal session.

Dual interfaces: Web (HTML/CSS/WASM) and CUI (Rust terminal output). Both use text-based tile notation — no graphical tile assets.

## 2. Color Palette & Roles

### Main Battle (index.html) — Purple Gradient

| Color     | Hex       | Usage                     |
| --------- | --------- | ------------------------- |
| Gradient  | `#667eea → #764ba2` | Page background     |
| Emerald   | `#10b981` | Primary button (CTA)      |
| Indigo    | `#6366f1` | Secondary button          |
| Red       | `#ef4444` | Danger button             |
| Amber     | `#f59e0b` | Warning button            |

### Hybrid Mode (hybrid.html) — Dark Navy

| Color     | Hex       | Usage                     |
| --------- | --------- | ------------------------- |
| Gradient  | `#1a1a2e → #16213e` | Page background    |
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
| Tiles & data     | `"Courier New", monospace`                         |

### Type Scale

| Element           | Size    | Weight | Notes                    |
| ----------------- | ------- | ------ | ------------------------ |
| Page title (h1)   | 2–2.5rem | 400  | Text shadow for depth    |
| Section header    | 1.2–1.3em | 400 |                          |
| Subsection        | 1.1–1.2em | 400 |                          |
| Body/card text    | 1rem    | 400    |                          |
| Small labels      | 0.9rem  | 400    |                          |
| Tile display      | 1.1–1.3rem | 400 | Monospace, letter-spacing 0.3rem |

### Text Effects

- Depth shadow: `2px 2px 4px rgba(0,0,0,0.5)` on headers
- Terminal glow: `0 0 10px #0f0` (debug page)
- Gradient clip text: `-webkit-background-clip: text` for title

## 4. Component Stylings

### Glass Cards

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

- Font: monospace, `letter-spacing: 0.3rem`
- Size: 1.3–1.5rem
- Container: dark background box
- Wrapping: `white-space: pre-wrap`, `line-height: 1.8`

### Status Indicators

- Green dot with `animation: pulse 2s infinite` for active states
- Box-shadow glows on active elements

## 5. Layout Principles

### Container

- Max width: `1400px`
- Padding: `1.5–2rem`

### Grid Patterns

- Players grid (main): `grid-template-columns: repeat(4, 1fr)`
- Players grid (hybrid): `grid-template-columns: repeat(2, 1fr)`
- Info grid: `auto-fit, minmax(200px, 1fr)`

### Spacing

Standard gap: `10–20px`. Generous padding throughout.

## 6. Depth & Elevation

### Glassmorphism Layers

- Background gradient (lowest)
- Glass panels with `backdrop-filter: blur(10px)` (mid)
- Interactive elements with enhanced borders (top)

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

- Use glassmorphism (`backdrop-filter: blur(10px)` + semi-transparent backgrounds) for all panels
- Keep tile rendering in monospace text — no tile image assets
- Apply gradient backgrounds per page identity
- Use `translateY(-2px)` hover effect on buttons
- Use consistent transition: `all 0.3s`
- Color-code log messages: green (player), blue (CPU), orange (system)
- Use emoji labels for buttons and status indicators

### Don't

- Mix page color schemes (purple pages stay purple, navy stays navy)
- Use tile graphics or images — text-based rendering is deliberate
- Apply debug-page terminal styling (green glow) to other pages
- Remove the glassmorphism effect — it defines the visual identity
- Use flat/material design patterns

### Animations

| Animation  | Duration | Timing  | Usage              |
| ---------- | -------- | ------- | ------------------ |
| `spin`     | 1s       | linear  | Loading spinner    |
| `pulse`    | 2s       | —       | Active indicator   |
| Hover      | 0.3s     | —       | Button effects     |

## 8. Responsive Behavior

### Grid Adaptation

- Players grid: 4 columns → 2 columns on smaller screens
- Info grid: `auto-fit, minmax(200px, 1fr)` handles collapse
- Viewport meta: `width=device-width, initial-scale=1.0`

### Layout

- Flexbox for button groups, navigation, controls
- Grid for player displays and info cards

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
Main battle:  #667eea → #764ba2  (purple gradient)
Hybrid mode:  #1a1a2e → #16213e  (dark navy)
Debug:        #1a1a1a             (terminal black)
```

### When generating UI for this project

- Glass card pattern: `rgba(255,255,255,0.1)` bg + `blur(10px)` + `rgba(255,255,255,0.2)` border
- Tile text uses monospace with `letter-spacing: 0.3rem`
- Each page gets its own gradient — never cross-contaminate
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
