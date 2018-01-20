#[cfg(test)]
mod test;

extern crate time;
extern crate timer;

extern crate chrono;
use self::chrono::{DateTime, Local, Datelike, Timelike, Weekday};

use std::fmt;
use std::iter;
use std::iter::Iterator;
use std::collections::BTreeMap;
use std::sync::mpsc::{channel, Receiver};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::str::FromStr;

extern crate serde_json;
use self::serde_json::Value;

use super::{Scrobbler, Scrobble};

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
    // Default to blank, expiring in 5s to trigger a retry.
    (String::new(), String::new(), 5000)
}

#[derive(Deserialize, Debug, Default)]
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

#[derive(Deserialize, Debug, Default)]
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

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum AlarmRepeat {
    Daily,
    Weekdays,
    Weekends,
}
impl Default for AlarmRepeat {
    fn default() -> Self { AlarmRepeat::Daily }
}
impl fmt::Display for AlarmRepeat {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            &AlarmRepeat::Daily => "daily",
            &AlarmRepeat::Weekdays => "weekdays",
            &AlarmRepeat::Weekends => "weekends",
        };
        write!(f, "{}", s)
    }
}
impl FromStr for AlarmRepeat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "weekdays" => Ok(AlarmRepeat::Weekdays),
            "weekends" => Ok(AlarmRepeat::Weekends),
            _ => Ok(AlarmRepeat::Daily),
        }
    }
}

pub struct AlarmEntry {
    pub time: String,
    pub repeat: AlarmRepeat,
    pub context: PlayContext,
    pub device: DeviceId,
    #[cfg(test)]
    pub now: Option<DateTime<Local>>,
}

impl<'a> From<&'a AlarmConfig> for AlarmEntry {
    fn from(alarm: &AlarmConfig) -> Self {
        AlarmEntry {
            time: format!("{:02}:{:02}", alarm.hour, alarm.minute),
            repeat: alarm.repeat,
            context: PlayContext::new()
                .context_uri(&alarm.context)
                .offset_position(0)
                .build(),
            device: alarm.device.clone(),
            #[cfg(test)]
            now: None,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct AlarmConfig {
    pub hour: u32,
    pub minute: u32,
    pub context: String,
    pub repeat: AlarmRepeat,
    pub device: String,
}
impl FromStr for AlarmConfig {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut fields = s.split(",");
        let time = fields.next().ok_or("Missing time")?;
        let repeat = fields.next().ok_or("Missing repeat")?;
        let context = fields.next().ok_or("Missing context")?;
        let device = fields.next().ok_or("Missing device")?;
        if device.is_empty() || context.is_empty() || time.is_empty() {
            return Err("Invalid alarm.".into());
        }
        let mut time_fields = time.split(":");
        let hour = time_fields.next().ok_or("Missing hour")?;
        let minute = time_fields.next().ok_or("Missing minute")?;
        Ok(AlarmConfig {
            hour: hour.trim().parse().unwrap_or(0),
            minute: minute.trim().parse().unwrap_or(0),
            context: context.trim().to_owned(),
            repeat: AlarmRepeat::from_str(repeat)
                .unwrap_or(AlarmRepeat::Daily),
            device: device.trim().to_owned(),
        })
    }
}
impl fmt::Display for AlarmConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02}:{:02},{},{},{}", self.hour, self.minute, self.repeat, self.context, self.device)
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

pub type AlarmId = usize;
struct AlarmTimer {
    entry: AlarmEntry,
    #[allow(dead_code)]
    timer: timer::Timer,
    #[allow(dead_code)]
    guard: Option<timer::Guard>,
    channel: Option<Receiver<()>>,
    time: Option<DateTime<Local>>,
    id: usize,
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

    alarms: Vec<AlarmTimer>,
    next_alarm_id: AtomicUsize,

    scrobbler: Option<Scrobbler>,
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
            alarms: Vec::new(),
            next_alarm_id: AtomicUsize::new(0),
            scrobbler: None,
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
        let mut cnr = SpotifyConnectr {
            api: self.api,
            settings: settings,
            auth_code: String::new(),
            access_token: self.access.clone(),
            refresh_token: self.refresh.clone(),
            expire_utc: self.expire,
            device: None,
            refresh_timer: timer::Timer::new(),
            refresh_timer_guard: None,
            refresh_timer_channel: None,
            alarms: Vec::new(),
            next_alarm_id: AtomicUsize::new(0),
            scrobbler: None,
        };
        let alarms: Vec<AlarmConfig> = cnr.settings.alarms.clone();
        for alarm in &alarms {
            let _ = cnr.schedule_alarm(alarm.into());
        }
        cnr.scrobbler_authenticate();
        Some(cnr)
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
    fn scrobbler_authenticate(&mut self) {
        let scrobbler = match self.settings.lastfm_enabled {
            false => None,
            true => match self.settings.lastfm {
                None => None,
                Some(ref fm) => {
                    let mut scrob = Scrobbler::new(fm.key.to_owned(), fm.secret.to_owned());
                    scrob.authenticate_with_session_key(fm.session_key.to_owned());
                    Some(scrob)
                }
            }
        };
        self.scrobbler = scrobbler;
    }
    pub fn reread_settings(&mut self) {
        if let Some(settings) = settings::read_settings(self.api.scopes_version) {
            self.settings = settings;
        }
        self.scrobbler_authenticate();
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
    fn alarm_current_time(_entry: &AlarmEntry) -> DateTime<Local> {
        // Allow unit tests to override 'now', so days can be deterministic
        #[cfg(test)]
        return match _entry.now {
            Some(dt) => dt.clone(),
            None => Local::now(),
        };
        #[cfg(not(test))]
        Local::now()
    }
    fn next_alarm_datetime(entry: &AlarmEntry) -> Result<DateTime<Local>, String> {
        let now = Self::alarm_current_time(&entry);
        //let format_12h = "%I:%M %p";
        let format_24h = "%H:%M";
        let alarm_time = match time::strptime(&entry.time, format_24h) {
            Ok(t) => t,
            _ => return Err("Could not parse alarm clock time.".to_string()),
        };
        let mut alarm = Self::alarm_current_time(&entry)
            .with_hour(alarm_time.tm_hour as u32).ok_or("Invalid hour")?
            .with_minute(alarm_time.tm_min as u32).ok_or("Invalid minute")?
            .with_second(0).ok_or("Invalid second")?
            .with_nanosecond(0).ok_or("Invalid nanosecond")?;
        // Increment by one day if the current hour has already passed
        if alarm < now {
            alarm = alarm + chrono::Duration::days(1);
        }
        // Increment until a day is found that matches the alarm repeat options
        loop {
            let is_weekday = match alarm.weekday() {
                Weekday::Sat | Weekday::Sun => false,
                _ => true,
            };
            if entry.repeat == AlarmRepeat::Daily ||
                (is_weekday && entry.repeat == AlarmRepeat::Weekdays) ||
                (!is_weekday && entry.repeat == AlarmRepeat::Weekends) {
                    break;
            }
            alarm = alarm + chrono::Duration::days(1);
        }
        Ok(alarm)
    }
    fn alarm_with_id(&mut self, id: AlarmId) -> Result<&mut AlarmTimer, ()> {
        let mut result = self.alarms
            .iter_mut()
            .filter(|x| {x.id == id})
            .collect::<Vec<&mut AlarmTimer>>();
        match result.pop() {
            Some(alarm) => Ok(alarm),
            _ => Err(()),
        }
    }
    pub fn alarm_time(&mut self, id: AlarmId) -> Result<DateTime<Local>, ()> {
        let alarm = self.alarm_with_id(id)?;
        match alarm.time {
            Some(time) => Ok(time),
            _ => Err(()),
        }
    }
    pub fn alarm_disable(&mut self, id: AlarmId) -> Result<(), ()> {
        let alarm = self.alarm_with_id(id)?;
        alarm.guard = None;
        alarm.channel = None;
        alarm.time = None;
        Ok(())
    }
    pub fn alarm_enabled(&mut self, id: AlarmId) -> bool {
        let alarm = self.alarm_with_id(id);
        match alarm {
            Ok(alarm) => alarm.guard.is_some(),
            _ => false,
        }
    }
    pub fn alarm_reschedule(&mut self, id: AlarmId) -> Result<(), ()> {
        let alarm = { self.alarm_with_id(id)? };
        let alarm_time = Self::next_alarm_datetime(&alarm.entry).unwrap();
        let (tx, rx) = channel::<>();

        let closure = move || {
            tx.send(()).unwrap_or_else(|_| { warn!("Alarm clock skipped."); });
        };
        let guard = alarm.timer.schedule_with_date(alarm_time, closure);
        let duration = alarm_time.signed_duration_since(Local::now());
        info!("Alarm {} set for {} hours from now ({} mins)", alarm.entry.time,
              duration.num_hours(), duration.num_minutes());

        alarm.channel = Some(rx);
        alarm.guard = Some(guard);
        alarm.time = Some(alarm_time);
        Ok(())
    }
    pub fn alarm_configure(&mut self, devices: Option<&ConnectDeviceList>) {
        let alarm_config = settings::request_web_alarm_config(&self.settings.alarms, devices);
        if settings::save_web_alarm_config(alarm_config).is_ok() {
            if let Some(settings) = settings::read_settings(self.api.scopes_version) {
                let ids: Vec<AlarmId> = self.alarms.iter().map(|x| {x.id}).collect();
                for id in ids {
                    let _ = self.alarm_disable(id);
                }
                self.alarms.clear();
                for alarm in &settings.alarms {
                    let _ = self.schedule_alarm(alarm.into());
                }
                self.settings = settings;
            };
        }
    }
    pub fn schedule_alarm(&mut self, entry: AlarmEntry) -> Result<AlarmId, ()> {
        let timer = timer::Timer::new();
        let id = self.next_alarm_id.fetch_add(1, Ordering::SeqCst);
        self.alarms.push(AlarmTimer {
            entry: entry,
            timer: timer,
            guard: None,
            channel: None,
            time: None,
            id: id,
        });
        self.alarm_reschedule(id)?;
        Ok(id as AlarmId)
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
                let closure = move || {
                    tx.send(()).unwrap_or_else(|_| { warn!("Token refresh lost."); });
                };
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

        // Get list of which alarms have expired
        let expired = self.alarms.iter().enumerate()
            .map(|(_idx, alarm)| {
                match alarm.channel {
                    Some(ref rx) => rx.try_recv().is_ok(),
                    _ => false,
                }
            }).collect::<Vec<_>>();
        // For each expired alarm, remove it, execute it, and reschedule it
        for (idx, exp) in expired.iter().enumerate() {
            if *exp {
                let old_alarm = self.alarms.swap_remove(idx);
                info!("Alarm started: {} on {}",
                      old_alarm.entry.context.context_uri.as_ref().unwrap(),
                      old_alarm.entry.device);
                self.set_target_device(Some(old_alarm.entry.device.clone()));
                self.play(Some(&old_alarm.entry.context));
                self.set_target_device(None);
                let id = old_alarm.id;
                self.alarms.push(old_alarm);
                let _ = self.alarm_reschedule(id);
            }
        }

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
                                       http::AccessToken::None);
        match json_response.code {
            Some(200) => {
                parse_spotify_token(&json_response.unwrap())
            },
            // Default to blank, expiring in 5s to trigger a retry.
            _ => { (String::new(), String::new(), 5000) }
        }
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
            Some(202) => {
                warn!("Spotify returned no state.");
                Default::default()
            }
            Some(401) => {
                warn!("Access token invalid.  Attempting to reauthenticate.");
                self.refresh_access_token();
                Default::default()
            }
            _ => Default::default()
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
        let body = serde_json::to_string(&DeviceIdList {device_ids: devices, play: play}).unwrap();
        http::http(self.api.player, None, Some(&body), http::HttpMethod::PUT, self.bearer_token())
    }
    pub fn transfer(&mut self, device: String, play: bool) -> SpotifyResponse {
        let body = serde_json::to_string(&DeviceIdList {device_ids: vec![device.clone()], play: play}).unwrap();
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
    fn device_can_scrobble(&self, device_type: &str) -> bool {
        if let Some(ref fm) = self.settings.lastfm {
            let dev = device_type.to_lowercase();
            if fm.ignore_pc && dev == "computer" {
                return false;
            }
            if fm.ignore_phone && ["smartphone".to_string(),"tablet".to_string()].contains(&dev) {
                return false;
            }
        }
        true
    }
    pub fn scrobbler_now_playing(&mut self, artist: String, track: String,
                                 album: String, device_type: String) {
        if self.device_can_scrobble(&device_type) {
            if let Some(ref scrobbler) = self.scrobbler {
                let s = Scrobble::new(artist, track.clone(), album);
                match scrobbler.now_playing(s) {
                    Ok(_) => { info!("Scrobbler now playing: {}", track); },
                    Err(e) => {error!("Scrobbler update failed: {}", e)},
                }
            }
        }
    }
    pub fn scrobble(&mut self, artist: String, track: String,
                    album: String, device_type: String) {
        if self.device_can_scrobble(&device_type) {
            if let Some(ref scrobbler) = self.scrobbler {
                let s = Scrobble::new(artist, track.clone(), album);
                match scrobbler.scrobble(s) {
                    Ok(_) => { info!("Scrobbled: {}", track); },
                    Err(e) => {error!("Scrobbler update failed: {}", e)},
                }
            }
        }
    }
    pub fn settings(&self) -> &settings::Settings {
        &self.settings
    }
}
