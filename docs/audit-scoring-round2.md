# 役判定・点数計算 OSS数値対比監査 第2ラウンド (#147)

第1ラウンド (#108) は「コード読解 + 標準ルール照合」だった。第2ラウンドは**本物の OSS 実装を実際に走らせ、同一手牌で han/fu/yaku/役満倍率を数値突き合わせ**する。

監査日: 2026-05-30 (session547)
参照: [`MahjongRepository/mahjong`](https://github.com/MahjongRepository/mahjong) (Python、リーチ麻雀採点のデファクト実装)

## 方法

- 参照を `uv run --with mahjong` で実行 (`audit/ref_score.py`)。
- xmj 側は同一手牌を採点する Rust example (`examples/audit_score.rs`、`cargo run --example audit_score`)。
- 共通手牌 20 種 (基本役 / 符エッジ / 役満 / 複合) を両方で採点し、`(han, fu)` (非役満) と役満倍率を diff。
- 役名は綴り差 (Iipeikou vs Iipeiko、Tsumo vs Menzen Tsumo 等) があるため、数値 (han/fu/倍率) で比較する。

再現コマンド:
```sh
uv run --with mahjong python3 audit/ref_score.py   # 参照
cargo run --example audit_score                    # xmj
```

## 検出した差分と対応

### 🔴 大三元の暗刻取りこぼし → 修正
`check_daisangen` が**副露 (pon/kan) のみ**数えており、白白白 發發發 中中中 を**鳴かずに暗刻で揃えた大三元**を取りこぼしていた (代わりに三暗刻 + 役牌×3 = 5飜になっていた)。`all_tiles` から各三元牌が 3 枚以上あるかで判定するよう修正。

- 参照 `daisangen_ron`: han=13 (Daisangen)。 修正前 xmj: 5飜。 修正後: 役満 (一致)。

### 🟠 四暗刻単騎のダブル役満化 → 実装
参照 (および天鳳等の標準) は**四暗刻単騎 = ダブル役満**だが、xmj は単役満固定だった。`agari::suuankou_multiplier_n` を新設し、単騎和了 (雀頭待ち) は倍率 2、シャンポンは 1 を返すよう変更。`check_suuankou` を bool → 倍率 (u32) に。

- 参照 `chinroutou_tsumo`: 39飜 (清老頭 + 四暗刻単騎ダブル = 3倍役満)。 修正前 xmj: 26飜 (2倍)。 修正後: 3倍 (一致)。
- 参照 `tsuuiisou_daisan_ron`: 52飜 (字一色 + 大三元 + 四暗刻単騎ダブル = 4倍)。 修正後 xmj: 4倍 (一致)。

## 一致確認 (20手牌すべて差分 0)

| 手牌 | 参照 han/fu | xmj | 一致 |
|---|---|---|---|
| 平和ツモ / ロン | 2/20, 1/30 | 同 | ✓ |
| 喰いタン / 三色 / 一通 / 一盃口 | 2-3/30 | 同 | ✓ |
| 七対子 | 2/25 | 同 | ✓ |
| 役牌+三色 (暗刻+雀頭役牌符) | 3/40 | 同 | ✓ |
| 嵌張立直 / 単騎立直 | 1-3/40 | 同 | ✓ |
| 么九暗刻ツモ | 1/30 | 同 | ✓ |
| 対々和+三暗刻+役牌 | 5/50 | 同 | ✓ |
| 清一色+一盃口 / 混一+一通+役牌 | 7/40, 6/40 | 同 | ✓ |
| 国士13面 | ダブル役満 | ymax=2 | ✓ |
| 四暗刻シャンポンツモ | 単役満 | ymax=1 | ✓ |
| 大三元(暗刻) | 役満 | ymax=1 | ✓ (修正後) |
| 清老頭+四暗刻単騎 | 3倍 | ymax=3 | ✓ (修正後) |
| 字一色+大三元+四暗刻単騎 | 4倍 | ymax=4 | ✓ (修正後) |
| 親跳満ツモ (立直+ツモ+平和+三色+一盃口) | 6/20 | 同 | ✓ |

**符計算 (面子符・待ち符・雀頭役牌符・門前ロン+10・平和20/30・七対子25)・基本役・点数倍率がすべて参照と一致。** 第1ラウンドの分解ベース刷新が数値レベルで正しいことを OSS 実装で裏付けた。

## スコープ外 / follow-up

- 本ラウンドは**門前手中心**。副露 (鳴き) 手の喰い下がり (三色/一通/チャンタの 1飜化) は xmj 実装済だが本対比では未網羅 → 次ラウンドで参照に Meld を渡して比較。
- ドラ・裏ドラ・赤ドラの枚数は参照の OptionalRules と xmj で別管理のため本対比から除外 (han への加算ロジックは第1ラウンドで確認済)。
- 回帰テスト: `test_daisangen_concealed_ankou` / `test_suuankou_tanki_double` / `test_suuankou_shanpon_single` を追加。
