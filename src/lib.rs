pub mod tile;
pub mod hand;
pub mod game;
pub mod player;
pub mod scoring;
pub mod agari;
pub mod ai;
pub mod nostr;
pub mod realtime;

#[cfg(feature = "wasm")]
pub mod wasm;

#[cfg(feature = "wasm")]
pub mod wasm_nostr;

#[cfg(feature = "wasm")]
pub mod wasm_webrtc;

pub use tile::{Tile, TileType, Suit};
pub use hand::Hand;
pub use game::{Game, GameMode, Team, team_of, east_west_target_yaku, SEIKYO_SEAT_FEE, SEIKYO_YAKUMAN_TIP, YAMIMA_HIDDEN_COST, YAMIMA_LIGHT_UP_COST};
pub use scoring::Yaku;
pub use player::Player;
pub use ai::{AiEngine, AiLevel};
pub use nostr::{NostrClient, NostrKeys, GameEvent, GameEventType, MatchState};
pub use realtime::{Call, CallKind, PlayerTimer, DEFAULT_TIMER_LIMIT_MS};
