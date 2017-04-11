use std::fmt;
use std::collections::BTreeMap;

extern crate rustc_serialize;
use self::rustc_serialize::{Decodable, Decoder, json};
use self::rustc_serialize::json::Json;

use super::http;
use super::settings;
use super::spotify_api;
use super::http::HttpResponse;

pub fn parse_spotify_token(json: &str) -> (String, String) {
    let json_data = Json::from_str(&json).unwrap();
    let obj = json_data.as_object().unwrap();
    let access_token = obj.get("access_token").unwrap().as_string().unwrap();
    let refresh_token = obj.get("refresh_token").unwrap().as_string().unwrap();
    (String::from(access_token),String::from(refresh_token))
}

//#[derive(RustcDecodable, RustcEncodable, Debug)]
#[derive(RustcEncodable, Debug)]
pub struct ConnectDevice {
    pub id: String,
    pub is_active: bool,
    pub is_restricted: bool,
    pub name: String,
    pub device_type: String,
    pub volume_percent: u32
}

impl Decodable for ConnectDevice {
    fn decode<D: Decoder>(d: &mut D) -> Result<ConnectDevice, D::Error> {
        d.read_struct("ConnectDevice", 6, |d| {
            let id = try!(d.read_struct_field("id", 0, |d| { d.read_str() }));
            let is_active = try!(d.read_struct_field("is_active", 1, |d| { d.read_bool() }));
            let is_restricted = try!(d.read_struct_field("is_restricted", 2, |d| { d.read_bool() }));
            let name = try!(d.read_struct_field("name", 3, |d| { d.read_str() }));
            let device_type = try!(d.read_struct_field("type", 4, |d| { d.read_str() }));
            let volume_percent = try!(d.read_struct_field("volume_percent", 5, |d| { d.read_u32() }));
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
    pub devices: Vec<ConnectDevice>
}

impl fmt::Display for ConnectDeviceList {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for dev in &self.devices {
            let _ = write!(f, "{:?}\n", dev);
        }
        Ok(())
    }
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub struct PlayerState {
    pub timestamp: u64,
    pub device: ConnectDevice,
    pub progress_ms: Option<u32>,
    pub is_playing: bool,
    pub shuffle_state: bool,
    pub repeat_state: String,
}

impl fmt::Display for PlayerState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}\n", self)
    }
}

pub fn request_oauth_tokens(auth_code: &str, settings: &settings::Settings) -> (String, String) {
    let query = QueryString::new()
        .add("grant_type", "authorization_code")
        .add("code", auth_code)
        .add("redirect_uri", format!("http://127.0.0.1:{}", settings.port))
        .add("client_id", settings.client_id.clone())
        .add("client_secret", settings.secret.clone())
        .build();

    let json_response = http::http(spotify_api::TOKEN, &query, "", http::HttpMethod::POST, None).unwrap();
    parse_spotify_token(&json_response)
}

pub type DeviceId = String;

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

pub type SpotifyResponse = HttpResponse;

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
    access_token: String,
    refresh_token: String,
    device: Option<DeviceId>,
}

impl SpotifyConnectr {
    pub fn new(settings: settings::Settings) -> SpotifyConnectr {
        SpotifyConnectr {settings: settings, auth_code: String::new(),
                         access_token: String::new(), refresh_token: String::new(),
                         device: None}
    }
    pub fn connect(&mut self) {
        self.auth_code = http::authenticate(&self.settings);
        let (access_token, refresh_token) = request_oauth_tokens(&self.auth_code, &self.settings);
        self.access_token = access_token;
        self.refresh_token = refresh_token;
    }
    pub fn request_device_list(&self) -> ConnectDeviceList {
        let json_response = http::http(spotify_api::DEVICES, "", "",
                                       http::HttpMethod::GET, Some(&self.access_token)).unwrap();
        json::decode(&json_response).unwrap()
    }
    pub fn request_player_state(&self) -> PlayerState {
        let json_response = http::http(spotify_api::PLAYER_STATE, "", "",
                                       http::HttpMethod::GET, Some(&self.access_token)).unwrap();
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
        http::http(spotify_api::PLAY, &query, &body, http::HttpMethod::PUT, Some(&self.access_token))
    }
    pub fn pause(&self) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        http::http(spotify_api::PAUSE, &query, "", http::HttpMethod::PUT, Some(&self.access_token))
    }
    pub fn next(&self) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        http::http(spotify_api::NEXT, &query, "", http::HttpMethod::POST, Some(&self.access_token))
    }
    pub fn previous(&self) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        http::http(spotify_api::PREVIOUS, &query, "", http::HttpMethod::POST, Some(&self.access_token))
    }
    pub fn seek(&self, position: u32) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("position_ms", position)
            .build();
        http::http(spotify_api::SEEK, &query, "", http::HttpMethod::PUT, Some(&self.access_token))
    }
    pub fn volume(&self, volume: u32) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("volume_percent", volume)
            .build();
        http::http(spotify_api::VOLUME, &query, "", http::HttpMethod::PUT, Some(&self.access_token))
    }
    pub fn shuffle(&self, shuffle: bool) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("state", shuffle)
            .build();
        http::http(spotify_api::SHUFFLE, &query, "", http::HttpMethod::PUT, Some(&self.access_token))
    }
    pub fn repeat(&self, repeat: SpotifyRepeat) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("state", repeat)
            .build();
        http::http(spotify_api::REPEAT, &query, "", http::HttpMethod::PUT, Some(&self.access_token))
    }
    pub fn transfer_multi(&self, devices: Vec<String>, play: bool) -> SpotifyResponse {
        let body = json::encode(&DeviceIdList {device_ids: devices, play: play}).unwrap();
        http::http(spotify_api::PLAYER, "", &body, http::HttpMethod::PUT, Some(&self.access_token))
    }
    pub fn transfer(&self, device: String, play: bool) -> SpotifyResponse {
        let body = json::encode(&DeviceIdList {device_ids: vec![device], play: play}).unwrap();
        http::http(spotify_api::PLAYER, "", &body, http::HttpMethod::PUT, Some(&self.access_token))
    }
}
