#[cfg(test)]
mod test;

extern crate time;
extern crate timer;
extern crate chrono;

use std::fmt;
use std::iter;
use std::collections::BTreeMap;
use std::sync::mpsc::{channel, Receiver};

extern crate serde_json;
use self::serde_json::Value;

use super::http;
use super::settings;
use super::SpotifyEndpoints;
use super::SPOTIFY_API;
use super::http::HttpResponse;

pub type DeviceId = String;
pub type SpotifyResponse = HttpResponse;

pub fn parse_spotify_token(json: &str) -> (String, String, u64) {
    if let Ok(json_data) = serde_json::from_str(json) {
        let json_data: Value = json_data;
        let access_token = json_data["access_token"].as_str().unwrap_or("");
        let refresh_token = match json_data.get("refresh_token") {
            Some(j) => j.as_str().unwrap(),
            None => "",
        };
        let expires_in = json_data["expires_in"].as_u64().unwrap_or(0 as u64);
        return (String::from(access_token),String::from(refresh_token), expires_in);
    }
    (String::new(), String::new(), 0)
}

#[derive(Deserialize, Debug)]
pub struct ConnectDevice {
    pub id: Option<String>,
    pub is_active: bool,
    pub is_restricted: bool,
    pub name: String,
    #[serde(rename(deserialize = "type"))]
    pub device_type: String,
    pub volume_percent: Option<u32>
}

impl fmt::Display for ConnectDevice {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let id = match self.id {
            Some(ref id) => id,
            None => "",
        };
        write!(f, "{:<40} <{}>", self.name, id)
    }
}

#[derive(Deserialize)]
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

impl<'a> iter::IntoIterator for &'a ConnectDeviceList {
    type Item = &'a ConnectDevice;
    //type IntoIter = ::std::vec::IntoIter<ConnectDevice>;
    type IntoIter = ::std::slice::Iter<'a, ConnectDevice>;
    fn into_iter(self) -> Self::IntoIter {
        (&self.devices).into_iter()
    }
}

impl iter::IntoIterator for ConnectDeviceList {
    type Item = ConnectDevice;
    type IntoIter = ::std::vec::IntoIter<ConnectDevice>;
    fn into_iter(self) -> Self::IntoIter {
        self.devices.into_iter()
    }
}

#[derive(Deserialize, Debug)]
pub struct ConnectPlaybackArtist {
    pub name: String,
    pub uri: String,
}

#[derive(Deserialize, Debug)]
pub struct ConnectPlaybackAlbum {
    pub name: String,
    pub uri: String,
}

#[derive(Deserialize, Debug)]
pub struct ConnectPlaybackItem {
    pub duration_ms: u32,
    pub name: String,
    pub uri: String,
    pub album: ConnectPlaybackAlbum,
    pub artists: Vec<ConnectPlaybackArtist>,
}

#[derive(Deserialize, Debug)]
pub struct ConnectContext {
    pub uri: String,
}

#[derive(Deserialize, Debug)]
pub struct PlayerState {
    pub timestamp: i64,
    pub device: ConnectDevice,
    pub progress_ms: Option<u32>,
    pub is_playing: bool,
    pub item: Option<ConnectPlaybackItem>,
    pub shuffle_state: bool,
    pub repeat_state: String,
    pub context: Option<ConnectContext>,
}

impl PlayerState {
    pub fn playing_from_context(&self, context: &str) -> bool {
        match self.context {
            Some(ref ctx) => ctx.uri == context,
            None => false,
        }
    }
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
        if let Some(ref item) = self.item {
            let duration: f64 = (item.duration_ms as f64) / 1000.0;
            let progress: f64 = position/duration*100.0;
            write!(f, "{} on {} [Volume {}%]\n{} <{}>\n{}s / {}s ({:.1}%)\n",
                   play_state, self.device.name, volume,
                   &item.name, &item.uri,
                   position, duration, progress)
        }
        else {
            write!(f, "{} on {} [Volume {}%]\n{} <{}>\n{}s / {}s ({:.1}%)\n",
                   play_state, self.device.name, volume,
                   "unknown", "unknown",
                   position, 0, 0)
        }
    }
}

#[derive(Serialize)]
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

#[derive(Serialize)]
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

#[derive(Serialize)]
struct UriList {
    uris: Vec<String>,
}

#[derive(Serialize)]
struct DeviceIdList {
    device_ids: Vec<String>,
    play: bool,
}

pub struct SpotifyConnectr<'a> {
    api: SpotifyEndpoints<'a>,
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
impl<'a> Default for SpotifyConnectr<'a> {
    fn default() -> Self {
        SpotifyConnectr {
            api: SPOTIFY_API,
            settings: Default::default(),
            auth_code: Default::default(),
            access_token: Default::default(),
            refresh_token: Default::default(),
            expire_utc: Default::default(),
            device: Default::default(),
            refresh_timer: timer::Timer::new(),
            refresh_timer_guard: Default::default(),
            refresh_timer_channel: Default::default(),
        }
    }
}

pub struct SpotifyConnectrBuilder<'a> {
    api: SpotifyEndpoints<'a>,
    access: Option<String>,
    refresh: Option<String>,
    expire: Option<u64>,
}
impl<'a> SpotifyConnectrBuilder<'a> {
    pub fn build(&mut self) -> Option<SpotifyConnectr<'a>> {
        let mut settings: settings::Settings = Default::default();
        if self.expire.is_none() {
            settings = match settings::read_settings(self.api.scopes_version) {
                Some(s) => s,
                None => { return None },
            };
            self.expire = settings.expire_utc;
            self.access = settings.access_token.clone();
            self.refresh = settings.refresh_token.clone();
        }
        Some(SpotifyConnectr {api: self.api,
                              settings: settings,
                              auth_code: String::new(),
                              access_token: self.access.clone(),
                              refresh_token: self.refresh.clone(),
                              expire_utc: self.expire,
                              device: None,
                              refresh_timer: timer::Timer::new(),
                              refresh_timer_guard: None,
                              refresh_timer_channel: None})
    }
    #[cfg(test)]
    fn with_api(&mut self, api: SpotifyEndpoints<'a>) -> &mut Self {
        self.api = api;
        self
    }
    #[cfg(test)]
    fn with_oauth_tokens(&mut self, access: &str, refresh: &str, expire: u64) -> &mut Self {
        self.access = Some(access.to_string());
        self.refresh = Some(refresh.to_string());
        self.expire = Some(expire);
        self
    }
}


impl<'a> SpotifyConnectr<'a> {
    pub fn new() -> SpotifyConnectrBuilder<'a> {
        SpotifyConnectrBuilder {
            api: SPOTIFY_API,
            access: None,
            refresh: None,
            expire: None,
        }
    }
    pub fn quick_save_playlist(&self, context: &str) -> Option<&str> {
        self.settings.quick_save_playlist(context)
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
                // Refresh a bit before it expires
                let expire_offset = match expire_offset {
                    x if x > 60 => x - 60,
                    _ => expire_offset,
                };
                let expire_offset = chrono::Duration::seconds(expire_offset);
                info!("Refreshing Spotify credentials in {} sec", expire_offset.num_seconds());
                let closure = move || { tx.send(()).unwrap(); };
                self.refresh_timer_guard = Some(self.refresh_timer.schedule_with_delay(expire_offset, closure));
                Ok(())
            }
            _ => Err(())
        }
    }
    pub fn refresh_access_token(&mut self) {
        info!("Refreshing Spotify credentials now.");
        self.refresh_timer_channel = None;
        match self.refresh_oauth_tokens() {
            Some((access_token, expires_in)) => {
                self.access_token = Some(access_token.clone());
                self.expire_utc = Some(self.expire_offset_to_utc(expires_in));
            },
            None => {
                self.authenticate();
            }
        }
        //let (access_token, expires_in) = ;

        info!("Refreshed credentials.");
        let _ = self.schedule_token_refresh();

        let access_token = self.access_token.clone().unwrap();
        let refresh_token = self.refresh_token.clone().unwrap();
        let _ = settings::save_tokens(self.api.scopes_version,
                                      &access_token,
                                      &refresh_token,
                                      self.expire_utc.unwrap());
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
        self.refresh_access_token();
    }
    pub fn authenticate(&mut self) {
        info!("Requesting fresh credentials.");
        self.auth_code = http::authenticate(self.api.scopes, self.api.authorize, &self.settings);
        let (access_token, refresh_token, expires_in) = self.request_oauth_tokens(&self.auth_code, &self.settings);
        let expire_utc = self.expire_offset_to_utc(expires_in);
        let _ = settings::save_tokens(self.api.scopes_version, &access_token,
                                      &refresh_token, expire_utc);
        self.access_token = Some(access_token);
        self.refresh_token = Some(refresh_token);
        self.expire_utc = Some(expire_utc);
        let _ = self.schedule_token_refresh();
    }
    pub fn request_oauth_tokens(&self, auth_code: &str, settings: &settings::Settings) -> (String, String, u64) {
    let query = QueryString::new()
        .add("grant_type", "authorization_code")
        .add("code", auth_code)
        .add("redirect_uri", format!("http://127.0.0.1:{}", settings.port))
        .add("client_id", settings.client_id.clone())
        .add("client_secret", settings.secret.clone())
        .build();
        let json_response = http::http(self.api.token, Some(&query), None, http::HttpMethod::POST,
                                       http::AccessToken::None).unwrap();
        parse_spotify_token(&json_response)
    }
    pub fn connect(&mut self) {
        if self.access_token.is_some() {
            info!("Reusing saved credentials.");
            self.refresh_access_token();
            return ()
        }
        self.authenticate()
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
    pub fn refresh_oauth_tokens(&self) -> Option<(String, u64)> {
        let query = QueryString::new()
            .add("grant_type", "refresh_token")
            .add("refresh_token", self.refresh_token.as_ref().unwrap())
            .add("client_id", self.settings.client_id.clone())
            .add("client_secret", self.settings.secret.clone())
            .build();
        let json_response = http::http(self.api.token, Some(&query), None,
                                       http::HttpMethod::POST, http::AccessToken::None);
        match json_response.code {
            Some(200) => {
                let (access_token, _, expires_in) = parse_spotify_token(&json_response.data.unwrap());
                Some((access_token, expires_in))
            },
            _ => { None }
        }
    }
    pub fn request_device_list(&mut self) -> Option<ConnectDeviceList> {
        let json_response = http::http(self.api.devices, None, None,
                                       http::HttpMethod::GET, self.bearer_token());
        match json_response.code {
            Some(200) => serde_json::from_str(&json_response.data.unwrap()).unwrap(),
            Some(401) => {
                warn!("Access token invalid.  Attempting to reauthenticate.");
                self.refresh_access_token();
                None
            }
            _ => None
        }
    }
    pub fn request_player_state(&mut self) -> Option<PlayerState> {
        let json_response = http::http(self.api.player_state, None, None,
                                       http::HttpMethod::GET, self.bearer_token());
        match json_response.code {
            Some(200) => match serde_json::from_str(&json_response.data.unwrap()) {
                Ok(json) => json,
                Err(err) => { info!("json error: {}", err); None },
            },
            Some(401) => {
                warn!("Access token invalid.  Attempting to reauthenticate.");
                self.refresh_access_token();
                None
            }
            _ => None
        }
    }
    pub fn set_target_device(&mut self, device: Option<DeviceId>) {
        self.device = device;
    }
    pub fn play(&self, context: Option<&PlayContext>) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        let body = match context {
            Some(x) => serde_json::to_string(x).unwrap(),
            None => String::new(),
        };
        http::http(self.api.play, Some(&query), Some(&body), http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn pause(&self) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        http::http(self.api.pause, Some(&query), None, http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn next(&self) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        http::http(self.api.next, Some(&query), None, http::HttpMethod::POST, self.bearer_token())
    }
    pub fn previous(&self) -> SpotifyResponse {
        let query = QueryString::new().add_opt("device_id", self.device.clone()).build();
        http::http(self.api.previous, Some(&query), None, http::HttpMethod::POST, self.bearer_token())
    }
    pub fn seek(&self, position: u32) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("position_ms", position)
            .build();
        http::http(self.api.seek, Some(&query), None, http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn volume(&self, volume: u32) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("volume_percent", volume)
            .build();
        http::http(self.api.volume, Some(&query), None, http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn shuffle(&self, shuffle: bool) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("state", shuffle)
            .build();
        http::http(self.api.shuffle, Some(&query), None, http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn repeat(&self, repeat: SpotifyRepeat) -> SpotifyResponse {
        let query = QueryString::new()
            .add_opt("device_id", self.device.clone())
            .add("state", repeat)
            .build();
        http::http(self.api.repeat, Some(&query), None, http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn transfer_multi(&mut self, devices: Vec<String>, play: bool) -> SpotifyResponse {
        let device = devices[0].clone();
        let body = serde_json::to_string(&DeviceIdList {device_ids: devices, play: play}).unwrap();
        self.set_target_device(Some(device));
        http::http(self.api.player, None, Some(&body), http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn transfer(&mut self, device: String, play: bool) -> SpotifyResponse {
        let body = serde_json::to_string(&DeviceIdList {device_ids: vec![device.clone()], play: play}).unwrap();
        self.set_target_device(Some(device));
        http::http(self.api.player, None, Some(&body), http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn save_track(&mut self, track: String, playlist: String) -> SpotifyResponse {
        let playlist_id = playlist.split(":").last().unwrap();
        let user_id = playlist.split(":").nth(2).unwrap();
        let uri = format!("{}/{}/playlists/{}/tracks",
                          self.api.add_to_playlist,
                          user_id, playlist_id);

        let body = serde_json::to_string(&UriList {uris: vec![track.clone()]}).unwrap();
        http::http(&uri, None, Some(&body), http::HttpMethod::POST, self.bearer_token())
    }
    pub fn get_presets(&mut self) -> &Vec<(String,String)> {
        &self.settings.presets
    }
}
