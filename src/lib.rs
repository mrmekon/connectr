pub mod http;
pub mod settings;
pub mod webapi;

// Re-export webapi interface to connectr root
pub use webapi::*;

#[cfg(test)]
#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[cfg(target_os = "macos")]
pub mod osx;

#[cfg(target_os = "windows")]
pub mod win;

#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

extern crate rustc_serialize;

#[derive(Clone, Copy)]
pub struct SpotifyEndpoints<'a> {
    scopes: &'a str,
    authorize: &'a str,
    token: &'a str,
    devices: &'a str,
    player_state: &'a str,
    play: &'a str,
    pause: &'a str,
    next: &'a str,
    previous: &'a str,
    seek: &'a str,
    volume: &'a str,
    shuffle: &'a str,
    repeat: &'a str,
    player: &'a str,
}

pub const SPOTIFY_API: SpotifyEndpoints = SpotifyEndpoints {
    scopes: "user-read-private streaming user-read-playback-state",
    authorize: "https://accounts.spotify.com/en/authorize",
    token: "https://accounts.spotify.com/api/token",
    devices: "https://api.spotify.com/v1/me/player/devices",
    player_state: "https://api.spotify.com/v1/me/player",
    play: "https://api.spotify.com/v1/me/player/play",
    pause: "https://api.spotify.com/v1/me/player/pause",
    next: "https://api.spotify.com/v1/me/player/next",
    previous: "https://api.spotify.com/v1/me/player/previous",
    seek: "https://api.spotify.com/v1/me/player/seek",
    volume: "https://api.spotify.com/v1/me/player/volume",
    shuffle: "https://api.spotify.com/v1/me/player/shuffle",
    repeat: "https://api.spotify.com/v1/me/player/repeat",
    player: "https://api.spotify.com/v1/me/player",
};

#[cfg(target_os = "linux")]
pub type Object = u64;
#[cfg(target_os = "windows")]
pub type Object = u32;
#[cfg(target_os = "macos")]
pub type Object = osx::Object;

#[cfg(target_os = "linux")]
pub type StatusBar = DummyStatusBar;
#[cfg(target_os = "macos")]
pub type StatusBar = osx::OSXStatusBar;
#[cfg(target_os = "windows")]
pub type StatusBar = win::WindowsStatusBar;

pub type MenuItem = *mut Object;
pub trait TStatusBar {
    type S: TStatusBar;
    fn new(tx: Sender<String>) -> Self::S;
    fn can_redraw(&mut self) -> bool;
    fn clear_items(&mut self);
    fn add_separator(&mut self);
    fn add_label(&mut self, label: &str);
    fn add_item(&mut self, item: &str, callback: NSCallback, selected: bool) -> *mut Object;
    fn add_quit(&mut self, label: &str);
    fn update_item(&mut self, item: *mut Object, label: &str);
    fn sel_item(&mut self, sender: u64);
    fn unsel_item(&mut self, sender: u64);
    fn set_tooltip(&mut self, text: &str);
    fn run(&mut self, block: bool);
}

use std::sync::mpsc::Sender;
pub type NSCallback = Box<Fn(u64, &Sender<String>)>;

pub struct DummyStatusBar {}
impl TStatusBar for DummyStatusBar {
    type S = DummyStatusBar;
    fn new(_: Sender<String>) -> Self::S { DummyStatusBar {} }
    fn can_redraw(&mut self) -> bool { true }
    fn clear_items(&mut self) {}
    fn add_separator(&mut self) {}
    fn add_label(&mut self, _: &str) {}
    fn add_item(&mut self, _: &str, _: NSCallback, _: bool) -> *mut Object { 0 as *mut Object }
    fn add_quit(&mut self, _: &str) {}
    fn update_item(&mut self, _: *mut Object, _: &str) {}
    fn sel_item(&mut self, _: u64) {}
    fn unsel_item(&mut self, _: u64) {}
    fn set_tooltip(&mut self, _: &str) {}
    fn run(&mut self, _: bool) {}
}

pub fn search_paths() -> Vec<String> {
    use std::collections::BTreeSet;
    //let mut v = Vec::<String>::new();
    let mut v = BTreeSet::<String>::new();

    // $HOME
    if let Some(dir) = std::env::home_dir() {
        v.insert(dir.display().to_string());
    }

    #[cfg(not(target_os = "macos"))]
    let bundle: Option<String> = None;
    #[cfg(target_os = "macos")]
    let bundle = osx::resource_dir();

    // OS bundle/resource dir
    if let Some(dir) = bundle {
        v.insert(dir);
    }

    // $CWD
    if let Ok(dir) = std::env::current_dir() {
        v.insert(dir.display().to_string());
    }

    // exe_dir
    if let Ok(mut dir) = std::env::current_exe() {
        dir.pop(); // remove the actual executable
        v.insert(dir.display().to_string());
    }

    let mut list: Vec<String> = Vec::new();
    for dir in v {
        list.push(dir);
    }
    list
}
