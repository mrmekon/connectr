extern crate ini;
use self::ini::Ini;
use super::http;
use super::AlarmRepeat;
use super::AlarmConfig;
use super::ConnectDeviceList;
use super::Scrobbler;

extern crate time;
extern crate fruitbasket;

use std::path;
use std::str::FromStr;
use std::collections::BTreeMap;

const INIFILE: &'static str = "connectr.ini";
const PORT: u32 = 5432;
pub const WEB_PORT: u32 = 5676;

#[derive(Default)]
pub struct LastfmSettings {
    pub key: String,
    pub secret: String,
    pub session_key: String,
    pub username: String,
    pub ignore_pc: bool,
    pub ignore_phone: bool,
}

#[derive(Default)]
pub struct Settings {
    pub port: u32,
    pub secret: String,
    pub client_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expire_utc: Option<u64>,
    pub presets: Vec<(String,String)>,
    pub default_quicksave: Option<String>,
    pub quicksave: BTreeMap<String, String>,
    pub alarms: Vec<AlarmConfig>,
    pub lastfm_enabled: bool,
    pub lastfm: Option<LastfmSettings>,
}

impl Settings {
    pub fn quick_save_playlist(&self, context: &str) -> Option<&str> {
        match self.quicksave.get(context) {
            Some(ref uri) => Some(&uri),
            None => {
                match self.default_quicksave {
                    Some(ref uri) => Some(&uri),
                    None => None,
                }
            }
        }
    }
}

fn default_inifile() -> String {
    format!("{}/.{}", dirs::home_dir().unwrap().display(), INIFILE)
}

fn inifile() -> String {
    // Try to load INI file from home directory
    let path = format!("{}/.{}", dirs::home_dir().unwrap().display(), INIFILE);
    if path::Path::new(&path).exists() {
        info!("Found config: {}", path);
        return path;
    }

    // Default to looking in current working directory
    let path = INIFILE.to_string();
    if path::Path::new(&path).exists() {
        info!("Found config: {}", path);
        return path;
    }

    String::new()
}

pub fn request_web_config(settings: Option<&Settings>) -> BTreeMap<String,String> {
    let mut form = format!(r###"
{}
<!DOCTYPE HTML>
<html>
<meta http-equiv="cache-control" content="no-cache" /><meta http-equiv="expires" content="0"></head>
<head><title>Connectr Installation</title></head>
<body>
<h2>Connectr Installation</h2>
Connectr requires a <em>paid</em> Spotify Premium account and a <em>free</em> Spotify developer application.</br>
If you don't have a Premium account, perhaps try a <a href="https://www.spotify.com/us/premium/">free trial</a>.</br>
</br>
To create your free developer application for Connectr, follow these instructions:</br>
<p><ul>
<li> Go to your <a href="https://developer.spotify.com/my-applications/#!/applications/create">Spotify Applications</a> page (login with your Spotify credentials)
<li> Click "CREATE AN APP" in the upper-right corner
<li> Enter a name (perhaps 'Connectr') and description ("Use Connectr app with my account.")
<li> Add a Redirect URI: <em>http://127.0.0.1:{}</em>
<li> Copy your <em>Client ID</em> and <em>Client Secret</em> to the fields below.
<li> Press the <em>SAVE</em> button at the bottom of Spotify's webpage
<li> Submit this configuration form
</ul></p>
<form method="POST" action="#" accept-charset="UTF-8"><table>
<tr><td colspan=2><h3>Spotify Credentials (all fields required):</h3></td></tr>
"###,
                           "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-cache, no-store, must-revalidate, max-age=0\r\n\r\n",
                           PORT);
    let client_id = settings.map_or("", |s| &s.client_id);
    let secret = settings.map_or("", |s| &s.secret);
    form.push_str(&format!(r###"
<tr><td>Client ID:</td><td><input type="text" name="client_id" value="{}" style="width:400px;"></td></tr>
<tr><td>Client Secret:</td><td><input type="text" name="secret" value="{}" style="width:400px;"></td></tr>
"###,
                           client_id, // client id
                           secret // secret
                           ));
    form.push_str(&format!(r###"
<tr><td colspan=2></br></br></tr></tr>
<tr><td colspan=2><h3>Presets (all fields optional):</h3>
    <div style="width:600px;">
    Presets let you start your favorite Spotify contexts (playlist, album, artist, etc) from Connectr.
    You can add an optional "quick save" playlist for each preset, to quickly save tracks you like to a known playlist.
    For instance, you might have a "Discover Weekly" preset, and a quick save to a "Best of Discover Weekly" playlist.
    You can also set a global "quick save" playlist, where tracks are saved if not playing from a preset with an associated quick-save playlist.</br>
    </br>
    All contexts must be specified in Spotify's URI format: ex: <code>spotify:album:2p2UgYlbg4yG44IKDp08Q8</code>
    </div>
    </br>
    One preset per line, in either format::</br>
    &nbsp;&nbsp;&nbsp;<code>[Preset Name] = [Context URI]</code></br>
    &nbsp;&nbsp;&nbsp;<code>[Preset Name] = [Context URI],[Quick-save Playlist URI]</code>
    </br></br>
</td></tr>
"###));
    let presets = match settings {
        Some(ref settings) => {
            let mut p = String::new();
            for &(ref name, ref uri) in &settings.presets {
                let qsave = match settings.quicksave.get(uri) {
                    Some(ref q) => format!(",{}", q),
                    None => String::new(),
                };
                p.push_str(&format!("{} = {}{}", name, uri, qsave));
            }
            p
        },
        None => String::new(),
    };
    let quicksave = match settings {
        Some(ref s) => match s.default_quicksave {
            Some(ref d) => &d,
            None => "",
        },
        None => "",
    };
    form.push_str(&format!(r###"
<tr><td>Presets:</br>(one per line)</td><td><textarea rows="10" cols="100"  name="presets" placeholder="First Preset Name = spotify:user:spotify:playlist:37i9dQZEVXboyJ0IJdpcuT">{}</textarea></td></tr>
<tr><td style="width:200px;">Quick-Save URI:</br>(playlist URI)</td><td>
    <input type="text" name="quicksave_default" value="{}" style="width:400px;"></td></tr>
"###,
                           presets, // Preset list
                           quicksave, // quicksave default
    ));

    form.push_str(&format!(r###"
<tr><td colspan=2></br></br></tr></tr>
<tr><td colspan=2><h3>Last.fm Scrobbling (optional):</h3>
    <div style="width:600px;">
    Enable Scrobbling to <a href="https://last.fm">Last.fm</a>.  If enabled, Connectr will scrobble any tracks that it sees playing on your Spotify account.  Note that Connectr must be running and online to scrobble, so this feature is most useful when Connectr is hosted on an always-on server like a home media machine or a VPS.</br></br>
    This whole section is optional, but all fields are required if any of them are specified.</br></br>
    Scrobbling requires a Last.fm account (free) and a developer API key (also free).  After you have signed up, you can <a href="https://www.last.fm/api/account/create">request an API key here</a>.</br></br>
    Spotify's desktop and mobile clients have Last.fm scrobbling built in, while Spotify Connect devices like speakers and TVs do not.  Below you can set whether you want Connectr to ignore (not scrobble) certain classes of devices.  This is especially handy for mobile devices, which can scrobble tracks that were played offline.</br></br>
    </div>
"###));

    let enabled = match settings {
        None => false,
        Some(settings) => match settings.lastfm {
            None => false,
            Some(_) => match settings.lastfm_enabled {
                false => false,
                true => true,
            }
        }
    };
    let mut key = String::new();
    let mut secret = String::new();
    let mut username = String::new();
    let mut password = String::new();
    let mut ignore_pc: &str = "";
    let mut ignore_phone: &str = "";
    if let Some(settings) = settings {
        if let Some(ref lastfm) = settings.lastfm {
            key = lastfm.key.clone();
            secret = lastfm.secret.clone();
            username = lastfm.username.clone();
            password = "<UNCHANGED>".to_string();
            ignore_pc = match lastfm.ignore_pc {
                false => "",
                true => "checked",
            };
            ignore_phone = match lastfm.ignore_phone {
                false => "",
                true => "checked",
            };
        }
    }
    let enabled = match enabled {
        true => "checked",
        false => "",
    };
    form.push_str(&format!(r###"
<tr><td>Scrobbling enabled:</td><td><input type="checkbox" name="lastfm_enabled" {}></input></td></tr>
<tr><td style="width: 200px;">Last.fm API key:</td><td><input type="text" name="lastfm_key" value="{}" style="width:400px;"></td></tr>
<tr><td>Last.fm API Secret:</td><td><input type="text" name="lastfm_secret" value="{}" style="width:400px;"></td></tr>
<tr><td>Last.fm username:</td><td><input type="text" name="lastfm_username" value="{}" style="width:400px;"></td></tr>
<tr><td>Last.fm password:</td><td><input type="password" name="lastfm_password" value="{}" style="width:400px;"></td></tr>
<tr><td>Ignore device types:</td>
<td>
<input type="checkbox" name="lastfm_ignore_pc" {}> Computers</input></br>
<input type="checkbox" name="lastfm_ignore_phone" {}> Smartphones</input></br>
</td>
</tr>
"###,
    enabled, key, secret, username, password, ignore_pc, ignore_phone));

    form.push_str(&format!(r###"
<tr><td colspan=2></br></br></tr></tr>
<tr><td colspan=2><center><input type="submit" name="cancel" value="Cancel" style="height:50px; width: 300px; font-size:20px;"> &nbsp; <input type="submit" value="Save Configuration" style="height:50px; width: 300px; font-size:20px;"></center></td></tr>
</br>
</table></form>
</br>
<small>Config will be saved as: <em>{}</em></br>
If something goes wrong or changes, edit or delete that file.</small>
</body></html>
"###,
                           default_inifile()));
    let reply = format!("{}Configuration saved.  You can close this window.",
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n");
    let mut config = BTreeMap::<String,String>::new();
    config.insert("port".to_string(), PORT.to_string());
    config.append(&mut http::config_request_local_webserver(WEB_PORT, form, reply));
    config
}

pub fn request_web_alarm_config(alarms: &Vec<AlarmConfig>,
                                devices: Option<&ConnectDeviceList>) -> BTreeMap<String,String> {
    let mut form = format!(
        r###"
{}
<!DOCTYPE HTML>
<html><head><title>Connectr Alarm Clocks</title><style>
tr:nth-child(even) {{ background: #f2f2f2; }}
th {{ width:120px;border-bottom: 1px solid #ddd; }}
a.tooltip {{ position: relative; }}
a.tooltip::before {{ content: attr(data-tip); position:absolute; z-index: 999;
white-space:normal; bottom:9999px; left: 50%; background:#000; color:#e0e0e0;
padding:2px 7px 2px 7px; line-height: 16px; opacity: 0; width: inherit; max-width: 500px;
transition:opacity 0.4s ease-out; font-weight: normal; text-align: left; min-width: 100px; }}
a.tooltip:hover::before {{ opacity: 1; bottom:-35px; }}
a.tooltip {{ color: #4e4e4e; text-decoration: none; vertical-align: super; font-size: 11px; font-weight: normal; }}
</style>
<meta http-equiv="cache-control" content="no-cache" /><meta http-equiv="expires" content="0"></head>
<body><h2>Connectr Alarm Clocks</h2>
    <div style="background-color: #9e9e9e; padding: 10px 10px 10px 10px;"><strong>IMPORTANT:</strong> Do NOT close this window without saving or cancelling.  Connectr is paused internally until this is dismissed!</div><br/>
    <h3>Connected Devices: <a href="#" style="width: 300px;" class="tooltip" data-tip="These are the devices that are currently logged in and online.  You can specify any device for your alarm, but be sure it is on and logged in at the right time.  Make sure your device doesn't become unavailable when idle.">eh?</a></h3>
    <table><tr><th style="width:300px;">Name</th><th>Device ID</th></tr>
"###,
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nCache-Control: no-cache, no-store, must-revalidate, max-age=0\r\n\r\n");
    if let Some(devices) = devices {
        for dev in devices {
            form.push_str(&format!(
                r###"
      <tr><td>{}</td><td>{}</td></tr>
"###, dev.name, dev.id.as_ref().unwrap_or(&"unknown".to_string())));
        }
    }
    form.push_str(&format!(
    r###"
    </table><br/>
    <h3>Alarm Schedule:</h3>
<form method="POST" action="#" accept-charset="UTF-8"><table>
<tr><th align="center">Time <a href="#" class="tooltip" data-tip="24-hour format">eh?</a></th><th align="center">Repeat</th><th>Spotify URI <a href="#" class="tooltip" data-tip="In Spotify app: right-click playlist, click 'Share', click 'URI'">eh?</a></th><th>Device ID <a href="#" class="tooltip" data-tip="Unique identifier of Spotify hardware.  Devices are listed above.">eh?</a></th></tr>
"###));
    for i in 0..5 {
        form.push_str(&format!(
            r###"
<tr><td align="center"><input style="width:40px;" type="number" name="hour_{}" min="0" max="23" size="3" maxlength="2" value="{}">:<input style="width:40px;" type="number" name="minute_{}" min="0" max="59" size="3" maxlength="2" value="{}"></td><td align="center"><select name="repeat_{}"><option value="daily" {}>Daily</option><option value="weekdays" {}>Weekdays</option><option value="weekends" {}>Weekends</option></select></td><td><input type="text" name="context_{}" style="width:350px;" value="{}"></td><td><input type="text" name="device_{}" size="42" value="{}"></td></tr>
"###,
            i,
            alarms.get(i).map_or(0, |a| {a.hour}).to_string(),
            i,
            alarms.get(i).map_or(0, |a| {a.minute}).to_string(),
            i,
            match alarms.get(i).map_or(false, |a| {a.repeat == AlarmRepeat::Daily   }) {
                true => "selected", _ => "" },
            match alarms.get(i).map_or(false, |a| {a.repeat == AlarmRepeat::Weekdays}) {
                true => "selected", _ => ""},
            match alarms.get(i).map_or(false, |a| {a.repeat == AlarmRepeat::Weekends}) {
                true => "selected", _ => ""},
            i,
            alarms.get(i).map_or("", |a| {&a.context}),
            i,
            alarms.get(i).map_or("", |a| {&a.device}),
        ));
    }

    form.push_str(&format!(
        r###"
<tr><td colspan="4"><br/><center><input type="submit" name="cancel" value="Cancel" style="height:50px; width: 300px; font-size:20px;"> &nbsp; <input type="submit" name="submit" value="Save Configuration" style="height:50px; width: 300px; font-size:20px;"></center></td></tr></br>
</table></form>
<br/><small>You can manually change or add more alarms by editing: <em>{}</em></br></br>
It is wise to have a backup alarm, in case your internet or Spotify is down.</small>
</body></html>
"###, inifile()));

    let reply = format!("{}Configuration saved.  You can close this window.",
                        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n");
    let mut config = BTreeMap::<String,String>::new();
    config.append(&mut http::config_request_local_webserver(WEB_PORT, form, reply));
    config
}

pub fn save_web_config(old_settings: Option<&Settings>, mut config: BTreeMap<String,String>) -> Ini {
    let mut c = match old_settings {
        Some(_) => Ini::load_from_file(&inifile()).unwrap_or(Ini::new()),
        None => Ini::new(),
    };
    if config.contains_key("cancel") {
        return c;
    }
    let port = config.remove("port").unwrap();
    c.with_section(Some("connectr".to_owned()))
        .set("port", port);
    let secret = config.remove("secret").unwrap_or("<PLACEHOLDER>".to_string());
    let client_id = config.remove("client_id").unwrap_or("<PLACEHOLDER>".to_string());
    let presets = config.remove("presets").unwrap_or(String::new());
    c.with_section(Some("application".to_owned()))
        .set("secret", secret.trim())
        .set("client_id", client_id.trim());
    if let Some(quicksave) = config.remove("quicksave_default") {
        c.with_section(Some("connectr".to_owned()))
            .set("quicksave_default", quicksave.trim());
    }
    {
        // TODO: INI uses HashMap, doesn't support maintaining order
        for preset in presets.split("\n") {
            let mut pair = preset.split("=");
            if pair.clone().count() == 2 {
                let key = pair.next().unwrap().trim();
                let value = pair.next().unwrap().trim();
                c.set_to(Some("presets"), key.to_string(), value.to_string());
            }
        }
    }
    let lastfm_enabled = match config.remove("lastfm_enabled") {
        Some(_) => true,
        None => false,
    };
    let key = config.remove("lastfm_key").unwrap_or("".to_string());
    let secret = config.remove("lastfm_secret").unwrap_or("".to_string());
    let username = config.remove("lastfm_username").unwrap_or("".to_string());
    let password = config.remove("lastfm_password").unwrap_or("".to_string());
    let session_key = match password.as_str() {
        "<UNCHANGED>" => match old_settings {
            Some(ref settings) => match settings.lastfm {
                Some(ref fm) => fm.session_key.clone(),
                None => String::new(),
            },
            None => String::new(),
        },
        _ => String::new(),
    };
    let ignore_pc = config.remove("lastfm_ignore_pc").unwrap_or("".to_string()) != "";
    let ignore_phone = config.remove("lastfm_ignore_phone").unwrap_or("".to_string()) != "";
    // Use existing session key if it exists, otherwise calculate a new one from
    // the password.
    let session_key = match session_key.len() {
        0 => {
            let mut scrob = Scrobbler::new(key.to_owned(), secret.to_owned());
            match scrob.authenticate_with_password(username.to_owned(), password.to_owned()) {
                Ok(_) => match scrob.session_key() {
                    Some(key) => key,
                    None => String::new(),
                },
                Err(_) => String::new(),
            }
        },
        _ => session_key,
    };
    c.with_section(Some("lastfm".to_owned()))
        .set("enabled", lastfm_enabled.to_string())
        .set("key", key.trim())
        .set("secret", secret.trim())
        .set("session_key", session_key)
        .set("username", username.trim())
        .set("ignore_pc", ignore_pc.to_string())
        .set("ignore_phone", ignore_phone.to_string());
    c.write_to_file(&default_inifile()).unwrap();
    c
}

pub fn save_web_alarm_config(config: BTreeMap<String,String>) -> Result<(), SettingsError> {
    let file = inifile();
    let mut conf: Ini;
    match Ini::load_from_file(&file) {
        Ok(c) => conf = c,
        Err(_) => return Err("Couldn't open configuration.".to_string()),
    }
    let mut entries: Vec<AlarmConfig> = Vec::with_capacity(5);
    for _ in 0..5 {
        entries.push(Default::default());
    }
    for pair in config.iter() {
        let key = pair.0;
        let value = pair.1;
        if key == "cancel" {
            return Err("Canceled by user.".to_string());
        }
        // Clear existing alarms
        conf.delete(Some("alarms".to_owned()));
        // are you kidding me??
        let idx = key.chars().rev().take(1).collect::<Vec<char>>()[0].to_digit(10).unwrap_or(0) as usize;
        let entry = entries.get_mut(idx).unwrap();
        match key.split("_").next().unwrap() {
            "hour" => {
                if let Ok(val) = value.parse() {
                    entry.hour = val;
                }
            },
            "minute" => {
                if let Ok(val) = value.parse() {
                    entry.minute = val;
                }
            },
            "repeat" => {
                entry.repeat = match value.as_ref() {
                    "weekdays" => AlarmRepeat::Weekdays,
                    "weekends" => AlarmRepeat::Weekends,
                    _ => AlarmRepeat::Daily,
                };
            },
            "context" => {
                entry.context = value.clone();
            },
            "device" => {
                entry.device = value.clone();
            },
            _ => {},
        }
    }
    let entries = entries.iter().filter(|e| {
        !e.context.is_empty() && !e.device.is_empty()
    }).collect::<Vec<&AlarmConfig>>();
    for (idx,entry) in entries.iter().enumerate() {
        conf.set_to(Some("alarms"), format!("alarm{}", idx+1), entry.to_string());
    }
    conf.write_to_file(&file).unwrap();
    Ok(())
}

pub fn read_settings(scopes_version: u32) -> Option<Settings> {
    info!("Attempting to read config file.");
    let conf = match Ini::load_from_file(&inifile()) {
        Ok(c) => c,
        Err(e) => {
            info!("Load file error: {}", e);
            info!("No config file found.");
            info!("Requesting settings via web form.");
            // Launch a local web server and open a browser to it.  Returns
            // the Spotify configuration.
            let web_config = request_web_config(None);
            save_web_config(None, web_config)
        }
    };

    let section = conf.section(Some("connectr".to_owned())).unwrap();
    let port = section.get("port").unwrap().parse().unwrap();
    let quicksave_default = match section.get("quicksave_default") {
        Some(uri) => Some(uri.to_string()),
        None => None,
    };

    let section = conf.section(Some("application".to_owned())).unwrap();
    let secret = section.get("secret").unwrap();
    let client_id = section.get("client_id").unwrap();
    if client_id.starts_with('<') || secret.starts_with('<') {
        error!("Invalid or missing configuration.  Cannot continue.");
        info!("");
        info!("ERROR: Spotify Client ID or Secret not set in connectr.ini!");
        info!("");
        info!("Create a Spotify application at https://developer.spotify.com/my-applications/ and");
        info!("add the client ID and secret to connectr.ini.");
        info!("");
        info!("Be sure to add a redirect URI of http://127.0.0.1:<PORT> to your Spotify application,");
        info!("and make sure the port matches in connectr.ini.");
        info!("");
        return None;
    }

    let mut access = None;
    let mut refresh = None;
    let mut expire_utc = None;
    if let Some(section) = conf.section(Some("tokens".to_owned())) {
        let saved_version = section.get("version");
        // Only accept saved tokens if the scopes version matches.  Otherwise
        // it will authenticate but some actions will be invalid.
        if saved_version.is_some() &&
            saved_version.unwrap().parse::<u32>().unwrap() == scopes_version {
            access = Some(section.get("access").unwrap().clone());
            refresh = Some(section.get("refresh").unwrap().clone());
            expire_utc = Some(section.get("expire").unwrap().parse().unwrap());
            info!("Read access token from INI!");
        }
    }

    let mut presets = Vec::<(String,String)>::new();
    let mut quicksave = BTreeMap::<String,String>::new();
    if let Some(section) = conf.section(Some("presets".to_owned())) {
        for (key, value) in section {
            let mut fields = value.split(",");
            let uri = fields.next().unwrap().trim(); // URI is required
            let save_uri = fields.next(); // quicksave is optional
            presets.push((key.to_owned(), uri.to_owned()));
            if let Some(save_uri) = save_uri {
                quicksave.insert(uri.to_owned(), save_uri.trim().to_owned());
            }
        }
    }
    let mut alarms = Vec::<AlarmConfig>::new();
    if let Some(section) = conf.section(Some("alarms".to_owned())) {
        for (_key, value) in section {
            match AlarmConfig::from_str(value) {
                Ok(a) => alarms.push(a),
                Err(_) => {},
            }
        }
    }

    let mut lastfm: Option<LastfmSettings> = None;
    let mut lastfm_enabled = false;
    if let Some(section) = conf.section(Some("lastfm".to_owned())) {
        let enabled = section.get("enabled").unwrap_or(&"false".to_string()).clone();
        let key = section.get("key").unwrap_or(&"".to_string()).clone();
        let secret = section.get("secret").unwrap_or(&"".to_string()).clone();
        let session_key = section.get("session_key").unwrap_or(&"".to_string()).clone();
        let username = section.get("username").unwrap_or(&"".to_string()).clone();
        let ignore_pc = section.get("ignore_pc").unwrap_or(&"false".to_string()).clone();
        let ignore_phone = section.get("ignore_phone").unwrap_or(&"false".to_string()).clone();
        lastfm_enabled = enabled == "true";
        lastfm = Some(LastfmSettings {
            key: key,
            secret: secret,
            session_key: session_key,
            username: username,
            ignore_pc: ignore_pc == "true",
            ignore_phone: ignore_phone == "true",
        });
    }

    Some(Settings { secret: secret.to_string(), client_id: client_id.to_string(), port: port,
                    access_token: access, refresh_token: refresh, expire_utc: expire_utc,
                    presets: presets,
                    default_quicksave: quicksave_default,
                    quicksave: quicksave,
                    alarms: alarms,
                    lastfm_enabled: lastfm_enabled,
                    lastfm: lastfm,
    })
}

pub type SettingsError = String;
pub fn save_tokens(version: u32, access: &str,
                   refresh: &str, expire_utc: u64) -> Result<(), SettingsError> {
    let mut conf = Ini::load_from_file(&inifile()).unwrap();
    conf.with_section(Some("tokens".to_owned()))
        .set("access", access)
        .set("refresh", refresh)
        .set("version", format!("{}",version))
        .set("expire", expire_utc.to_string());
    conf.write_to_file(&inifile()).unwrap();
    Ok(())
}
