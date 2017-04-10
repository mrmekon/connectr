pub mod http;
pub mod settings;
pub mod webapi;

pub use webapi::*;

extern crate rustc_serialize;

pub mod spotify_api {
    pub const SCOPES: &'static [&'static str] = &["user-read-private", "streaming", "user-read-playback-state"];
    pub const AUTHORIZE: &'static str = "https://accounts.spotify.com/en/authorize";
    pub const TOKEN: &'static str = "https://accounts.spotify.com/api/token";
    pub const DEVICES: &'static str = "https://api.spotify.com/v1/me/player/devices";
    pub const PLAYER_STATE: &'static str = "https://api.spotify.com/v1/me/player";
    pub const PLAY: &'static str = "https://api.spotify.com/v1/me/player/play";
    pub const PAUSE: &'static str = "https://api.spotify.com/v1/me/player/pause";
}
