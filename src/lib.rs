pub mod tile;
pub mod hand;
pub mod game;
pub mod player;
pub mod scoring;
pub mod ai;
pub mod nostr;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "wasm")]
pub mod wasm_nostr;

#[cfg(feature = "wasm")]
pub mod wasm_webrtc;

pub use tile::{Tile, TileType, Suit};
pub use hand::Hand;
pub use game::Game;
pub use player::Player;
pub use ai::{AiEngine, AiLevel};
pub use nostr::{NostrClient, NostrKeys, GameEvent, GameEventType, MatchState};