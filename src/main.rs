use xmj_core::{Game, GameMode, Tile, AiEngine, AiLevel, SEIKYO_SEAT_FEE, SEIKYO_YAKUMAN_TIP, Team};
use std::io::{self, Write};

fn main() {
    println!("邪雀 Xtreme Mahjong (xmj) - CUIクライアント");
    println!("==========================================");

    // 引数解析（std のみ）: --mode standard | seikyo | washizu
    let mode = parse_mode_from_args();
    match mode {
        GameMode::Standard => println!("ルール: 標準麻雀"),
        GameMode::Seikyo => println!(
            "ルール: 誠京麻雀（場代 {} / 役満祝儀 {}点)",
            SEIKYO_SEAT_FEE, SEIKYO_YAKUMAN_TIP
        ),
        GameMode::Washizu => {
            println!("ルール: 鷲巣麻雀（3/4 透明牌、他家の glass 牌が見える）")
        }
        GameMode::FiveTile => {
            println!("ルール: 5枚麻雀（クライマックスだけ麻雀）")
        }
        GameMode::EastWest => {
            println!("ルール: 東西戦（クリア麻雀）");
            println!("  クリア対象役: 三色同順 / 一気通貫 / 対々和 / 全帯么 / 混老頭");
            println!("  東チーム = 東家(座席0) + 西家(座席2)");
            println!("  西チーム = 南家(座席1) + 北家(座席3)");
            println!("  チームとして指定二翻役5種を先に全部揃えたチームの勝利");
        }
        GameMode::Yamima => {
            println!("ルール: 闇麻（闇牌 1000 / 照射 500）");
            println!("  打牌入力に `?` プレフィックスを付けると闇牌（裏向き）打牌");
            println!("  例: `?1m` で 1m を闇牌として河に置く（1000 点支払い）");
            println!("  闇牌は他家からはロン・鳴き不可。照射 API で公開できる");
        }
    }

    let player_names = vec![
        "あなた".to_string(),
        "CPU1".to_string(),
        "CPU2".to_string(),
        "CPU3".to_string(),
    ];

    let mut game = Game::new_with_mode(player_names, mode);

    // 誠京麻雀: 局開始時に場代供託
    // ※ 現状は局ループ未実装のためゲーム開始時に 1 回のみ。
    //   局ごとの再徴収配線は follow-up Issue。
    if game.mode == GameMode::Seikyo {
        game.collect_seat_fee(SEIKYO_SEAT_FEE);
        println!(
            "[誠京] 場代 {} 点ずつ供託しました（pot: {} 点）",
            SEIKYO_SEAT_FEE, game.pot
        );
    }

    println!("{}", game.get_game_state_string());

    loop {
        if game.is_game_over() {
            if game.mode == GameMode::EastWest {
                match game.east_west_winner() {
                    Some(Team::East) => println!("東チーム勝利！（東家+西家）"),
                    Some(Team::West) => println!("西チーム勝利！（南家+北家）"),
                    None => println!("ゲーム終了（流局）"),
                }
            } else {
                println!("ゲーム終了");
            }
            break;
        }

        let current_player = game.get_current_player();
        println!("\n{} のターン:", current_player.name);

        if current_player.id == 0 {
            // プレイヤーのターン
            handle_player_turn(&mut game);
        } else {
            // CPUのターン
            handle_cpu_turn(&mut game);
        }

        println!("{}", game.get_game_state_string());
    }
}

/// `--mode <value>` または `--mode=<value>` から値を取り出す。
/// 値が無い場合は None を返す。
fn extract_mode_value(args: &[String]) -> Option<String> {
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        if arg == "--mode" {
            // 次のトークンを値として取る。無ければ None（呼び出し側で warning）
            return Some(iter.next().cloned().unwrap_or_default());
        }
        if let Some(rest) = arg.strip_prefix("--mode=") {
            return Some(rest.to_string());
        }
    }
    None
}

/// `--mode seikyo` / `--mode standard` を解析する。未指定なら Standard。
/// 未知の値・値なしのときは warning を出して Standard にフォールバック。
fn parse_mode_from_args() -> GameMode {
    let args: Vec<String> = std::env::args().collect();
    match extract_mode_value(&args) {
        None => GameMode::Standard,
        Some(val) if val.is_empty() => {
            eprintln!("[warn] --mode に値が指定されていません、standard で起動します");
            GameMode::Standard
        }
        Some(val) => match val.to_lowercase().as_str() {
            "seikyo" => GameMode::Seikyo,
            "washizu" => GameMode::Washizu,
            "standard" => GameMode::Standard,
            "five-tile" | "five_tile" | "fivetile" => GameMode::FiveTile,
            "east-west" | "east_west" | "eastwest" => GameMode::EastWest,
            "yamima" => GameMode::Yamima,
            other => {
                eprintln!("[warn] 未知のモード '{}'、standard で起動します", other);
                GameMode::Standard
            }
        },
    }
}

fn handle_player_turn(game: &mut Game) {
    // 誠京麻雀: 親かつ連荘なら二度ヅモを試みる
    let is_dealer = game.current_player == game.dealer;
    let seikyo_double = game.mode == GameMode::Seikyo && game.dealer_won_last && is_dealer;

    if seikyo_double {
        println!("[誠京] 親二度ヅモ可能（2 枚ツモして 1 枚捨てる）");
        if let Some((t1, t2)) = game.dealer_double_draw() {
            println!("ツモ1: {}  ツモ2: {}", t1.to_string(), t2.to_string());

            // 二度ヅモ後の手牌は 15 枚。和了判定 tile_count() == 14 を維持するため、
            // 即「どちらを捨てるか」を選ばせる UX を提供する。
            // 入力なし・EOF 時は決定論的フォールバック（= 1 枚目 t1 を即捨て、2 枚目 t2 を残す）。
            println!("どちらを即捨てしますか？");
            println!("  (1) 1 枚目 {} を捨てる  ※デフォルト", t1.to_string());
            println!("  (2) 2 枚目 {} を捨てる", t2.to_string());
            println!("  (3) 通常打牌（手から好きな 1 枚を選ぶ）");
            print!("選択 (1/2/3, Enter=1): ");
            io::stdout().flush().unwrap();

            let mut input = String::new();
            let choice = match io::stdin().read_line(&mut input) {
                Ok(_) => input.trim().to_string(),
                Err(_) => String::new(),
            };

            match choice.as_str() {
                "2" => {
                    if game.discard_tile(t2) {
                        println!("[誠京] 2 枚目 {} を即捨て", t2.to_string());
                        return;
                    } else {
                        eprintln!("[誠京] 2 枚目即捨てに失敗。通常打牌に移行");
                    }
                }
                "3" => {
                    // 通常打牌フローへ続く（後段の discard ループに任せる）
                    // ※ 手牌は 15 枚のままなのでユーザーは任意の 1 枚を捨てる
                }
                // "1" or "" (Enter/EOF) or その他 → 1 枚目を捨てるフォールバック
                _ => {
                    if game.discard_tile(t1) {
                        println!("[誠京] 1 枚目 {} を即捨て（フォールバック）", t1.to_string());
                        return;
                    } else {
                        eprintln!("[誠京] 1 枚目即捨てに失敗。通常打牌に移行");
                    }
                }
            }
            // ここに落ちたら通常打牌ループへ（手牌 14 or 15 枚）
        } else {
            // 山が足りない等のフォールバック
            if !game.current_player_draw() {
                println!("山牌がありません");
                return;
            }
        }
    } else {
        // 通常ツモ
        if !game.current_player_draw() {
            println!("山牌がありません");
            return;
        }
    }

    let player = game.get_current_player();
    println!("ツモ: 手牌 {}", player.get_hand_string());

    // 和了チェック（簡易）
    // - 通常モード: 手牌 14 枚 + テンパイ
    // - FiveTile モード: 手牌 5 枚で「雀頭+面子」の完成形（最後の 1 枚がアガリ牌相当）
    let can_declare_tsumo = match game.mode {
        GameMode::FiveTile => {
            let tiles = player.hand.get_tiles();
            tiles.len() == 5
                && tiles
                    .last()
                    .map(|t| player.hand.can_win_five_tile(t))
                    .unwrap_or(false)
        }
        _ => player.tile_count() == 14 && player.is_tenpai(),
    };
    if can_declare_tsumo {
        print!("ツモ和了しますか？ (y/n): ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        if input.trim().to_lowercase() == "y" {
            println!("ツモ！");
            return;
        }
    }

    // 打牌選択
    let yamima = game.mode == GameMode::Yamima;
    loop {
        if yamima {
            print!("打牌する牌を入力してください (例: 1m / 闇牌は ?1m): ");
        } else {
            print!("打牌する牌を入力してください (例: 1m, 5p, to): ");
        }
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            return;
        }
        let input = input.trim();
        if input.is_empty() {
            // EOF やパイプ終了時はループから抜ける（実機 1 ターン検証用）
            return;
        }

        // Yamima ルール: `?` プレフィックスで闇牌打牌
        if yamima {
            if let Some(rest) = input.strip_prefix('?') {
                if let Some(tile) = Tile::from_string(rest) {
                    if game.discard_hidden_tile(tile) {
                        println!("[闇麻] 闇牌打牌（1000 点支払い）");
                        break;
                    } else {
                        println!(
                            "闇牌打牌に失敗しました（手牌にないか点棒不足、または非 Yamima モード）"
                        );
                        continue;
                    }
                } else {
                    println!("無効な牌です");
                    continue;
                }
            }
        }

        if let Some(tile) = Tile::from_string(input) {
            if game.discard_tile(tile) {
                println!("打牌: {}", tile.to_string());
                break;
            } else {
                println!("その牌は手牌にありません");
            }
        } else {
            println!("無効な牌です");
        }
    }
}

fn handle_cpu_turn(game: &mut Game) {
    let player_name = game.get_current_player().name.clone();

    // ツモ
    if !game.current_player_draw() {
        println!("山牌がありません");
        return;
    }

    // AIエンジンで打牌を選択（レベル3: シャンテン数ベース）
    let ai = AiEngine::new(AiLevel::Intermediate);
    let hand = &game.get_current_player().hand;

    if let Some(discard_tile) = ai.select_discard(hand) {
        game.discard_tile(discard_tile);
        println!("{} が {} を打牌 [シャンテン数: {}]",
            player_name,
            discard_tile.to_string(),
            game.get_current_player().hand.shanten()
        );
    }
}
