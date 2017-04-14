pub mod http;
pub mod settings;
pub mod webapi;

// Re-export webapi interface to connectr root
pub use webapi::*;

#[cfg(target_os = "macos")]
pub mod osx;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

extern crate rustc_serialize;

pub mod spotify_api {
    pub const SCOPES: &'static [&'static str] = &[
        "user-read-private", "streaming", "user-read-playback-state"
    ];
    pub const AUTHORIZE: &'static str = "https://accounts.spotify.com/en/authorize";
    pub const TOKEN: &'static str = "https://accounts.spotify.com/api/token";
    pub const DEVICES: &'static str = "https://api.spotify.com/v1/me/player/devices";
    pub const PLAYER_STATE: &'static str = "https://api.spotify.com/v1/me/player";
    pub const PLAY: &'static str = "https://api.spotify.com/v1/me/player/play";
    pub const PAUSE: &'static str = "https://api.spotify.com/v1/me/player/pause";
    pub const NEXT: &'static str = "https://api.spotify.com/v1/me/player/next";
    pub const PREVIOUS: &'static str = "https://api.spotify.com/v1/me/player/previous";
    pub const SEEK: &'static str = "https://api.spotify.com/v1/me/player/seek";
    pub const VOLUME: &'static str = "https://api.spotify.com/v1/me/player/volume";
    pub const SHUFFLE: &'static str = "https://api.spotify.com/v1/me/player/shuffle";
    pub const REPEAT: &'static str = "https://api.spotify.com/v1/me/player/repeat";
    pub const PLAYER: &'static str = "https://api.spotify.com/v1/me/player";
}
