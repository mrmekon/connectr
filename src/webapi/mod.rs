extern crate rustc_serialize;
use self::rustc_serialize::{Decodable, Decoder, json};
use self::rustc_serialize::json::Json;

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
    pub progress_ms: u32,
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
    pub fn request_device_list() {}
    pub fn go(&self) {
        let json_response = http::http(spotify_api::DEVICES, "", http::HttpMethod::GET, Some(&self.access_token)).unwrap();
        let device_list: ConnectDeviceList = json::decode(&json_response).unwrap();

        let json_response = http::http(spotify_api::PLAYER_STATE, "", http::HttpMethod::GET, Some(&self.access_token)).unwrap();
        let player_state: PlayerState = json::decode(&json_response).unwrap();

        println!("Auth Code: {}...", &self.auth_code[0..5]);
        println!("Access: {}... / Refresh: {}...", &self.access_token[0..5], &self.refresh_token[0..5]);
        for dev in device_list.devices {
            println!("{:?}", dev);
        }
        println!("State: {:?}", player_state);

        let query = format!("{{\"context_uri\": \"spotify:user:mrmekon:playlist:4XqYlbPdDUsranzjicPCgf\"}}");
        let _ = http::http(spotify_api::PLAY, &query, http::HttpMethod::PUT, Some(&self.access_token));
    }
}
