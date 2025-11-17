use xmj_core::{Game, Tile, AiEngine, AiLevel};
use std::io::{self, Write};

fn main() {
    println!("邪雀 Xtreme Mahjong (xmj) - CUIクライアント");
    println!("==========================================");
    
    let player_names = vec![
        "あなた".to_string(),
        "CPU1".to_string(), 
        "CPU2".to_string(),
        "CPU3".to_string(),
    ];
    
    let mut game = Game::new(player_names);
    
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

fn handle_player_turn(game: &mut Game) {
    // ツモ
    if !game.current_player_draw() {
        println!("山牌がありません");
        return;
    }
    
    let player = game.get_current_player();
    println!("ツモ: 手牌 {}", player.get_hand_string());
    
    // 和了チェック
    if player.tile_count() == 14 {
        // 簡単な和了チェック（完全ではない）
        if player.is_tenpai() {
            print!("ツモ和了しますか？ (y/n): ");
            io::stdout().flush().unwrap();
            let mut input = String::new();
            io::stdin().read_line(&mut input).unwrap();
            
            if input.trim().to_lowercase() == "y" {
                println!("ツモ！");
                return;
            }
        }
    }
    
    // 打牌選択
    loop {
        print!("打牌する牌を入力してください (例: 1m, 5p, to): ");
        io::stdout().flush().unwrap();
        
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim();
        
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
