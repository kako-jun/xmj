use xmj_core::{Game, GameMode, Tile, AiEngine, AiLevel, SEIKYO_SEAT_FEE};
use std::io::{self, Write};

fn main() {
    println!("邪雀 Xtreme Mahjong (xmj) - CUIクライアント");
    println!("==========================================");

    // 引数解析（std のみ）: --mode standard | seikyo
    let mode = parse_mode_from_args();
    match mode {
        GameMode::Standard => println!("ルール: 標準麻雀"),
        GameMode::Seikyo => println!("ルール: 誠京麻雀（場代 {} / 役満祝儀 8000）", SEIKYO_SEAT_FEE),
    }

    let player_names = vec![
        "あなた".to_string(),
        "CPU1".to_string(),
        "CPU2".to_string(),
        "CPU3".to_string(),
    ];

    let mut game = Game::new_with_mode(player_names, mode);

    // 誠京麻雀: 局開始時に場代供託
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
            println!("ゲーム終了");
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

/// `--mode seikyo` / `--mode standard` を解析する。未指定なら Standard。
fn parse_mode_from_args() -> GameMode {
    let args: Vec<String> = std::env::args().collect();
    let mut iter = args.iter().skip(1);
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--mode" => {
                if let Some(val) = iter.next() {
                    match val.to_lowercase().as_str() {
                        "seikyo" => return GameMode::Seikyo,
                        "standard" => return GameMode::Standard,
                        other => {
                            eprintln!("[warn] 未知のモード '{}'、standard で起動します", other);
                            return GameMode::Standard;
                        }
                    }
                }
            }
            a if a.starts_with("--mode=") => {
                let val = &a["--mode=".len()..];
                match val.to_lowercase().as_str() {
                    "seikyo" => return GameMode::Seikyo,
                    "standard" => return GameMode::Standard,
                    other => {
                        eprintln!("[warn] 未知のモード '{}'、standard で起動します", other);
                        return GameMode::Standard;
                    }
                }
            }
            _ => {}
        }
    }
    GameMode::Standard
}

fn handle_player_turn(game: &mut Game) {
    // 誠京麻雀: 親かつ連荘なら二度ヅモを試みる
    let is_dealer = game.current_player == game.dealer;
    let seikyo_double = game.mode == GameMode::Seikyo && game.dealer_won_last && is_dealer;

    if seikyo_double {
        println!("[誠京] 親二度ヅモ可能（2 枚ツモして 1 枚捨てる）");
        if let Some((t1, t2)) = game.dealer_double_draw() {
            println!("ツモ1: {}  ツモ2: {}", t1.to_string(), t2.to_string());
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
    if player.tile_count() == 14 && player.is_tenpai() {
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
    loop {
        print!("打牌する牌を入力してください (例: 1m, 5p, to): ");
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
