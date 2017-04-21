extern crate time;
extern crate timer;
extern crate chrono;

use std::fmt;
use std::iter;
use std::process;
use std::collections::BTreeMap;
use std::sync::mpsc::{channel, Receiver};

extern crate rustc_serialize;
use self::rustc_serialize::{Decodable, Decoder, json};
use self::rustc_serialize::json::Json;

use super::http;
use super::settings;
use super::spotify_api;
use super::http::HttpResponse;

pub type DeviceId = String;
pub type SpotifyResponse = HttpResponse;

pub fn parse_spotify_token(json: &str) -> (String, String, u64) {
    let json_data = Json::from_str(&json).unwrap();
    let obj = json_data.as_object().unwrap();
    let access_token = obj.get("access_token").unwrap().as_string().unwrap();
    let refresh_token = match obj.get("refresh_token") {
        Some(j) => j.as_string().unwrap(),
        None => "",
    };
    let expires_in = obj.get("expires_in").unwrap().as_u64().unwrap();
    (String::from(access_token),String::from(refresh_token), expires_in)
}

//#[derive(RustcDecodable, RustcEncodable, Debug)]
#[derive(RustcEncodable, Debug)]
pub struct ConnectDevice {
    pub id: String,
    pub is_active: bool,
    pub is_restricted: bool,
    pub name: String,
    pub device_type: String,
    pub volume_percent: Option<u32>
}

impl fmt::Display for ConnectDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:<40} <{}>", self.name, self.id)
    }
}

impl Decodable for ConnectDevice {
    fn decode<D: Decoder>(d: &mut D) -> Result<ConnectDevice, D::Error> {
        d.read_struct("ConnectDevice", 6, |d| {
            let id = try!(d.read_struct_field("id", 0, |d| { d.read_str() }));
            let is_active = try!(d.read_struct_field("is_active", 1, |d| { d.read_bool() }));
            let is_restricted = try!(d.read_struct_field("is_restricted", 2, |d| { d.read_bool() }));
            let name = try!(d.read_struct_field("name", 3, |d| { d.read_str() }));
            let device_type = try!(d.read_struct_field("type", 4, |d| { d.read_str() }));
            let volume_percent = try!(d.read_struct_field("volume_percent", 5, |d| {
                match d.read_u32() {
                    Ok(x) => Ok(Some(x)),
                    // 'null' triggers a decode error.  Convert error to valid None:
                    Err(_) => Ok(None),
                }}));
            Ok(ConnectDevice{ id: id,
                              is_active: is_active,
                              is_restricted: is_restricted,
                              name: name,
                              device_type: device_type,
                              volume_percent: volume_percent})
        })
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct ConnectDeviceList {
    pub devices: Vec<ConnectDevice>,
}

impl fmt::Display for ConnectDeviceList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for dev in &self.devices {
            let _ = write!(f, "{}\n", dev);
        }
        Ok(())
    }
}

impl iter::IntoIterator for ConnectDeviceList {
    type Item = ConnectDevice;
    type IntoIter = ::std::vec::IntoIter<ConnectDevice>;
    fn into_iter(self) -> Self::IntoIter {
        self.devices.into_iter()
    }
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub struct ConnectPlaybackArtist {
    pub name: String,
    pub uri: String,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub struct ConnectPlaybackAlbum {
    pub name: String,
    pub uri: String,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub struct ConnectPlaybackItem {
    pub duration_ms: u32,
    pub name: String,
    pub uri: String,
    pub album: ConnectPlaybackAlbum,
    pub artists: Vec<ConnectPlaybackArtist>,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub struct ConnectContext {
    uri: String,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub struct PlayerState {
    pub timestamp: u64,
    pub device: ConnectDevice,
    pub progress_ms: Option<u32>,
    pub is_playing: bool,
    pub item: ConnectPlaybackItem,
    pub shuffle_state: bool,
    pub repeat_state: String,
    pub context: Option<ConnectContext>,
}

impl fmt::Display for PlayerState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let play_state = match self.is_playing {
            true => "Playing",
            false => "Paused",
        };
        let volume = match self.device.volume_percent {
            Some(x) => x.to_string(),
            None => "???".to_string(),
        };
        let position: f64 = match self.progress_ms {
            Some(x) => (x as f64)/1000.0,
            None => 0.0,
        };
        let duration: f64 = (self.item.duration_ms as f64) / 1000.0;
        let progress: f64 = position/duration*100.0;
        write!(f, "{} on {} [Volume {}%]\n{} <{}>\n{}s / {}s ({:.1}%)\n",
               play_state, self.device.name, volume,
               &self.item.name, &self.item.uri,
               position, duration, progress)
    }
}

pub fn request_oauth_tokens(auth_code: &str, settings: &settings::Settings) -> (String, String, u64) {
    let query = QueryString::new()
        .add("grant_type", "authorization_code")
        .add("code", auth_code)
        .add("redirect_uri", format!("http://127.0.0.1:{}", settings.port))
        .add("client_id", settings.client_id.clone())
        .add("client_secret", settings.secret.clone())
        .build();

    let json_response = http::http(spotify_api::TOKEN, &query, "", http::HttpMethod::POST,
                                   http::AccessToken::None).unwrap();
    parse_spotify_token(&json_response)
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct PlayContextOffset {
    pub position: Option<u32>,
    pub uri: Option<String>,
}
impl Default for PlayContextOffset {
    fn default() -> PlayContextOffset { PlayContextOffset { position: None, uri: None } }
}
impl Clone for PlayContextOffset {
    fn clone(&self) -> PlayContextOffset {
        PlayContextOffset {
            position: self.position.clone(),
            uri: self.uri.clone(),
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct PlayContext {
    pub context_uri: Option<String>,
    pub uris: Option<Vec<String>>,
    pub offset: Option<PlayContextOffset>,
}
impl Default for PlayContext {
    fn default() -> PlayContext { PlayContext { context_uri: None, uris: None, offset: None } }
}
impl PlayContext {
    pub fn new() -> PlayContext {
        PlayContext::default()
    }
    pub fn context_uri<'a>(&'a mut self, uri: &str) -> &'a mut PlayContext {
        self.context_uri = Some(uri.to_string());
        self
    }
    pub fn uri<'a>(&'a mut self, uri: &str) -> &'a mut PlayContext {
        match self.uris {
            Some(ref mut uris) => uris.push(uri.to_string()),
            None => {
                let mut vec = Vec::<String>::new();
                vec.push(uri.to_string());
                self.uris = Some(vec);
            },
        };
        self
    }
    pub fn offset_position<'a>(&'a mut self, position: u32) -> &'a mut PlayContext {
        match self.offset {
            Some(ref mut o) => o.position = Some(position),
            None => {
                let mut o = PlayContextOffset::default();
                o.position = Some(position);
                self.offset = Some(o);
            }
        };
        self
    }
    pub fn offset_uri<'a>(&'a mut self, uri: &str) -> &'a mut PlayContext {
        match self.offset {
            Some(ref mut o) => o.uri = Some(uri.to_string()),
            None => {
                let mut o = PlayContextOffset::default();
                o.uri = Some(uri.to_string());
                self.offset = Some(o);
            }
        };
        self
    }
    pub fn build(&self) -> PlayContext {
        PlayContext { context_uri: self.context_uri.clone(),
                      uris: self.uris.clone(),
                      offset: self.offset.clone() }
    }
}

struct QueryString {
    map: BTreeMap<String,String>,
}
impl QueryString {
    fn new() -> QueryString { QueryString { map: BTreeMap::<String,String>::new() } }
    fn add_opt(&mut self, key: &str, value: Option<String>) -> &mut QueryString {
        match value {
            Some(v) => { self.map.insert(key.to_string(), v); },
            None => {},
        }
        self
    }
    fn add<A>(&mut self, key: &str, value: A) -> &mut QueryString
        where A: ToString {
        self.map.insert(key.to_string(), value.to_string());
        self
    }
    fn build(&self) -> String {
        let mut s = String::new();
        for (key, val) in &self.map {
            match s.len() {
                0 => { } // '?' inserted in HTTP layer
                _ => { s = s + "&"; }
            }
            s = s + &format!("{}={}", key, val);
        }
        s
    }
}

pub enum SpotifyRepeat {
    Off,
    Track,
    Context,
}
impl ToString for SpotifyRepeat {
    fn to_string(&self) -> String {
        match self {
            &SpotifyRepeat::Off => "off".to_string(),
            &SpotifyRepeat::Track => "track".to_string(),
            &SpotifyRepeat::Context => "context".to_string(),
        }
    }
}

#[derive(RustcDecodable, RustcEncodable)]
struct DeviceIdList {
    device_ids: Vec<String>,
    play: bool,
}

pub struct SpotifyConnectr {
    settings: settings::Settings,
    auth_code: String,
    access_token: Option<String>,
    refresh_token: Option<String>,
    expire_utc: Option<u64>,
    device: Option<DeviceId>,

    refresh_timer: timer::Timer,
    refresh_timer_guard: Option<timer::Guard>,
    refresh_timer_channel: Option<Receiver<()>>,
}

impl SpotifyConnectr {
    pub fn new() -> SpotifyConnectr {
        let settings = match settings::read_settings() {
            Some(s) => s,
            None => process::exit(0),
        };
        let expire = settings.expire_utc;
        let access = settings.access_token.clone();
        let refresh = settings.refresh_token.clone();
        SpotifyConnectr {settings: settings,
                         auth_code: String::new(),
                         access_token: access,
                         refresh_token: refresh,
                         expire_utc: expire,
                         device: None,
                         refresh_timer: timer::Timer::new(),
                         refresh_timer_guard: None,
                         refresh_timer_channel: None}
    }
    fn is_token_expired(&self) -> bool {
        let now = time::now_utc().to_timespec().sec as u64;
        let expire_utc = self.expire_utc.unwrap_or(0);
        expire_utc <= (now - 60)
    }
    fn expire_offset_to_utc(&self, expires_in: u64) -> u64 {
        let now = time::now_utc().to_timespec().sec as u64;
        now + expires_in
    }
    fn expire_utc_to_offset(&self, expire_utc: u64) -> u64 {
        let now = time::now_utc().to_timespec().sec as i64;
        let offset = expire_utc as i64 - now;
        match offset {
            x if x > 0 => x as u64,
            _ => 0,
        }
    }
    fn schedule_token_refresh(&mut self) -> Result<(), ()> {
        match self.expire_utc {
            Some(expire_utc) => {
                let (tx, rx) = channel::<()>();
                self.refresh_timer_channel = Some(rx);
                let expire_offset = self.expire_utc_to_offset(expire_utc) as i64;
                let expire_offset = chrono::Duration::seconds(expire_offset);
                let closure = move || { tx.send(()).unwrap(); };
                self.refresh_timer_guard = Some(self.refresh_timer.schedule_with_delay(expire_offset, closure));
                Ok(())
            }
            _ => Err(())
        }
    }
    pub fn await_once(&mut self, blocking: bool) {
        // Choose between blocking or non-blocking receive.
        let recv_fn: Box<Fn(&Receiver<()>) -> bool> = match blocking {
            true  => Box::new(move |rx| { match rx.recv() { Ok(_) => true, Err(_) => false } }),
            false => Box::new(move |rx| { match rx.try_recv() { Ok(_) => true, Err(_) => false } }),
        };
        let need_refresh = match self.refresh_timer_channel.as_ref() {
            Some(rx) => recv_fn(rx),
            _ => false,
        };
        if !need_refresh {
            return ()
        }
        self.refresh_timer_channel = None;
        let (access_token, expires_in) = self.refresh_oauth_tokens();
        self.access_token = Some(access_token.clone());
        self.expire_utc = Some(self.expire_offset_to_utc(expires_in));
        println!("Refreshed credentials.");
        let _ = self.schedule_token_refresh();

        let access_token = self.access_token.clone().unwrap();
        let refresh_token = self.refresh_token.clone().unwrap();
        let _ = settings::save_tokens(&access_token,
                                      &refresh_token,
                                      self.expire_utc.unwrap());
    }
    pub fn connect(&mut self) {
        if self.access_token.is_some() && !self.is_token_expired() {
            println!("Reusing saved credentials.");
            let _ = self.schedule_token_refresh();
            return ()
        }
        println!("Requesting fresh credentials.");
        self.auth_code = http::authenticate(&self.settings);
        let (access_token, refresh_token, expires_in) = request_oauth_tokens(&self.auth_code, &self.settings);
        let expire_utc = self.expire_offset_to_utc(expires_in);
        let _ = settings::save_tokens(&access_token, &refresh_token, expire_utc);
        self.access_token = Some(access_token);
        self.refresh_token = Some(refresh_token);
        self.expire_utc = Some(expire_utc);
        let _ = self.schedule_token_refresh();
    }
    pub fn bearer_token(&self) -> http::AccessToken {
        match self.access_token {
            Some(ref x) => http::AccessToken::Bearer(x),
            None => http::AccessToken::None,
        }
    }
    pub fn basic_token(&self) -> http::AccessToken {
        match self.access_token {
            Some(ref x) => http::AccessToken::Basic(x),
            None => http::AccessToken::None,
        }
    }
    pub fn refresh_oauth_tokens(&self) -> (String, u64) {
        let query = QueryString::new()
            .add("grant_type", "refresh_token")
            .add("refresh_token", self.refresh_token.as_ref().unwrap())
            .add("client_id", self.settings.client_id.clone())
            .add("client_secret", self.settings.secret.clone())
            .build();
        let json_response = http::http(spotify_api::TOKEN, &query, "",
                                       http::HttpMethod::POST, http::AccessToken::None).unwrap();
        let (access_token, _, expires_in) = parse_spotify_token(&json_response);
        (access_token, expires_in)
    }
    pub fn request_device_list(&self) -> ConnectDeviceList {
        let json_response = http::http(spotify_api::DEVICES, "", "",
                                       http::HttpMethod::GET, self.bearer_token()).unwrap();
        json::decode(&json_response).unwrap()
    }
    pub fn request_player_state(&self) -> PlayerState {
        let json_response = http::http(spotify_api::PLAYER_STATE, "", "",
                                       http::HttpMethod::GET, self.bearer_token()).unwrap();
        json::decode(&json_response).unwrap()
    }
    pub fn set_target_device(&mut self, device: Option<DeviceId>) {
        self.device = device;
    }
    pub fn play(&self, context: Option<&PlayContext>) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        let body = match context {
            Some(x) => json::encode(x).unwrap(),
            None => String::new(),
        };
        http::http(spotify_api::PLAY, &query, &body, http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn pause(&self) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        http::http(spotify_api::PAUSE, &query, "", http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn next(&self) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        http::http(spotify_api::NEXT, &query, "", http::HttpMethod::POST, self.bearer_token())
    }
    pub fn previous(&self) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        http::http(spotify_api::PREVIOUS, &query, "", http::HttpMethod::POST, self.bearer_token())
    }
    pub fn seek(&self, position: u32) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("position_ms", position)
            .build();
        http::http(spotify_api::SEEK, &query, "", http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn volume(&self, volume: u32) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("volume_percent", volume)
            .build();
        http::http(spotify_api::VOLUME, &query, "", http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn shuffle(&self, shuffle: bool) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("state", shuffle)
            .build();
        http::http(spotify_api::SHUFFLE, &query, "", http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn repeat(&self, repeat: SpotifyRepeat) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("state", repeat)
            .build();
        http::http(spotify_api::REPEAT, &query, "", http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn transfer_multi(&mut self, devices: Vec<String>, play: bool) -> SpotifyResponse {
        let device = devices[0].clone();
        let body = json::encode(&DeviceIdList {device_ids: devices, play: play}).unwrap();
        self.set_target_device(Some(device));
        http::http(spotify_api::PLAYER, "", &body, http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn transfer(&mut self, device: String, play: bool) -> SpotifyResponse {
        let body = json::encode(&DeviceIdList {device_ids: vec![device.clone()], play: play}).unwrap();
        self.set_target_device(Some(device));
        http::http(spotify_api::PLAYER, "", &body, http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn get_presets(&mut self) -> &Vec<(String,String)> {
        &self.settings.presets
    }
}
