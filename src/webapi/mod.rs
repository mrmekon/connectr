use std::fmt;

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
    let query = format!("grant_type=authorization_code&code={}&redirect_uri=http://127.0.0.1:{}&client_id={}&client_secret={}",
                        auth_code, settings.port, settings.client_id, settings.secret);
    let json_response = http::http(spotify_api::TOKEN, &query, "", http::HttpMethod::POST, None).unwrap();
    parse_spotify_token(&json_response)
}

pub struct SpotifyConnectr {
    settings: settings::Settings,
    auth_code: String,
    access_token: String,
    refresh_token: String,
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

fn device_id_query(device: Option<DeviceId>) -> String {
    match device {
        Some(x) => format!("device_id={}", x),
        None => "".to_string()
    }
}

pub type SpotifyResponse = HttpResponse;

impl SpotifyConnectr {
    pub fn new(settings: settings::Settings) -> SpotifyConnectr {
        SpotifyConnectr {settings: settings, auth_code: String::new(),
                         access_token: String::new(), refresh_token: String::new()}
    }
    pub fn connect(&mut self) {
        self.auth_code = http::authenticate(&self.settings);
        let (access_token, refresh_token) = request_oauth_tokens(&self.auth_code, &self.settings);
        self.access_token = access_token;
        self.refresh_token = refresh_token;
    }
    pub fn request_device_list(&self) -> ConnectDeviceList {
        let json_response = http::http(spotify_api::DEVICES, "", "", http::HttpMethod::GET, Some(&self.access_token)).unwrap();
        json::decode(&json_response).unwrap()
    }
    pub fn request_player_state(&self) -> PlayerState {
        let json_response = http::http(spotify_api::PLAYER_STATE, "", "", http::HttpMethod::GET, Some(&self.access_token)).unwrap();
        json::decode(&json_response).unwrap()
    }
    pub fn play(&self, device: Option<DeviceId>, context: Option<&PlayContext>) -> SpotifyResponse {
        let query = device_id_query(device);
        let body = match context {
            Some(x) => json::encode(x).unwrap(),
            None => String::new(),
        };
        http::http(spotify_api::PLAY, &query, &body, http::HttpMethod::PUT, Some(&self.access_token))
    }
    pub fn pause(&self, device: Option<DeviceId>) -> SpotifyResponse {
        let query = device_id_query(device);
        http::http(spotify_api::PAUSE, &query, "", http::HttpMethod::PUT, Some(&self.access_token))
    }
}
