extern crate rustc_serialize;
use self::rustc_serialize::{Decodable, Decoder, json};
use self::rustc_serialize::json::Json;
use self::rustc_serialize::json::ToJson;

use super::http;
use super::settings;
use super::spotify_api;

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

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub struct PlayerState {
    pub timestamp: u64,
    pub device: ConnectDevice,
    pub progress_ms: Option<u32>,
    pub is_playing: bool,
    pub shuffle_state: bool,
    pub repeat_state: String,
}

pub fn request_oauth_tokens(auth_code: &str, settings: &settings::Settings) -> (String, String) {
    let query = format!("grant_type=authorization_code&code={}&redirect_uri=http://127.0.0.1:{}&client_id={}&client_secret={}",
                        auth_code, settings.port, settings.client_id, settings.secret);
    let json_response = http::http(spotify_api::TOKEN, &query, http::HttpMethod::POST, None).unwrap();
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

#[derive(RustcDecodable, RustcEncodable)]
pub struct PlayContext {
    pub context_uri: Option<String>,
    pub uris: Option<Vec<String>>,
    pub offset: Option<PlayContextOffset>,
}
impl Default for PlayContext {
    fn default() -> PlayContext { PlayContext { context_uri: None, uris: None, offset: None } }
}

fn append_to_json_string<T>(json: String, key: String, value: T) -> String
    where T: ToJson {
    // Highly wasteful implementation.  struct -> string -> obj -> string
    let jdata = Json::from_str(&json).unwrap();
    let mut jobj = jdata.into_object().unwrap();
    jobj.insert(key, value.to_json());
    Json::Object(jobj).to_string()
}

fn append_device_id(json: String, device: Option<DeviceId>) -> String {
    match device {
        Some(x) => {
            // Convert empty string to empty JSON
            let json = match json.len() {
                0 => "{}".to_string(),
                _ => json
            };
            append_to_json_string(json, "device_id".to_string(), x)
        },
        None => json
    }
}

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
        let json_response = http::http(spotify_api::DEVICES, "", http::HttpMethod::GET, Some(&self.access_token)).unwrap();
        json::decode(&json_response).unwrap()
    }
    pub fn request_player_state(&self) -> PlayerState {
        let json_response = http::http(spotify_api::PLAYER_STATE, "", http::HttpMethod::GET, Some(&self.access_token)).unwrap();
        json::decode(&json_response).unwrap()
    }
    pub fn play(&self, device: Option<DeviceId>, context: Option<&PlayContext>) {
        let query = match context {
            Some(x) => append_device_id(json::encode(x).unwrap(), device),
            None => String::new(),
        };
        let _ = http::http(spotify_api::PLAY, &query, http::HttpMethod::PUT, Some(&self.access_token));
    }
    pub fn pause(&self, device: Option<DeviceId>) {
        let query = append_device_id(String::new(), device);
        let _ = http::http(spotify_api::PAUSE, &query, http::HttpMethod::PUT, Some(&self.access_token));
    }
}
