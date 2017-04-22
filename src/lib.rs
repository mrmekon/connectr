pub mod http;
pub mod settings;
pub mod webapi;

// Re-export webapi interface to connectr root
pub use webapi::*;

#[macro_use]
extern crate log;

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

#[cfg(target_os = "macos")]
pub type Object = osx::Object;
#[cfg(not(target_os = "macos"))]
pub type Object = u64;

pub type MenuItem = *mut Object;
pub trait TStatusBar {
    type S: TStatusBar;
    fn new(tx: Sender<String>) -> Self::S;
    fn clear_items(&mut self);
    fn add_separator(&mut self);
    fn add_label(&mut self, label: &str);
    fn add_item(&mut self, item: &str, callback: NSCallback, selected: bool) -> *mut Object;
    fn add_quit(&mut self, label: &str);
    fn update_item(&mut self, item: *mut Object, label: &str);
    fn sel_item(&mut self, sender: u64);
    fn unsel_item(&mut self, sender: u64);
    fn set_tooltip(&self, text: &str);
    fn run(&mut self, block: bool);
}

use std::sync::mpsc::Sender;
pub type NSCallback = Box<Fn(u64, &Sender<String>)>;

pub struct DummyStatusBar {}
impl TStatusBar for DummyStatusBar {
    type S = DummyStatusBar;
    fn new(_: Sender<String>) -> Self::S { DummyStatusBar {} }
    fn clear_items(&mut self) {}
    fn add_separator(&mut self) {}
    fn add_label(&mut self, _: &str) {}
    fn add_item(&mut self, _: &str, _: NSCallback, _: bool) -> *mut Object { 0 as *mut Object }
    fn add_quit(&mut self, _: &str) {}
    fn update_item(&mut self, _: *mut Object, _: &str) {}
    fn sel_item(&mut self, _: u64) {}
    fn unsel_item(&mut self, _: u64) {}
    fn set_tooltip(&self, _: &str) {}
    fn run(&mut self, _: bool) {}
}
