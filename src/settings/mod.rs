extern crate ini;
use self::ini::Ini;
use super::http;
use super::AlarmRepeat;
use super::AlarmConfig;

extern crate time;
extern crate fruitbasket;

use std::env;
use std::path;
use std::str::FromStr;
use std::collections::BTreeMap;

const INIFILE: &'static str = "connectr.ini";
const PORT: u32 = 5432;
const WEB_PORT: u32 = 5676;

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
    format!("{}/.{}", env::home_dir().unwrap().display(), INIFILE)
}

fn inifile() -> String {
    // Try to load INI file from home directory
    let path = format!("{}/.{}", env::home_dir().unwrap().display(), INIFILE);
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

pub fn request_web_config() -> BTreeMap<String,String> {
    let form = format!(r###"
{}
<!DOCTYPE HTML>
<html>
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
<li> Enter a name (perhaps "Connectr") and description ("Use Connectr app with my account.")
<li> Add a Redirect URI: <em>http://127.0.0.1:{}</em>
<li> Copy your <em>Client ID</em> and <em>Client Secret</em> to the fields below.
<li> Press the <em>SAVE</em> button at the bottom of Spotify's webpage
<li> Submit this configuration form
</ul></p>
<form method="POST" action="#" accept-charset="UTF-8"><table>
<tr><td colspan=2><h3>Spotify Credentials (all fields required):</h3></td></tr>
<tr><td>Client ID:</td><td><input type="text" name="client_id" style="width:400px;"></td></tr>
<tr><td>Client Secret:</td><td><input type="text" name="secret" style="width:400px;"></td></tr>
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
<tr><td>Presets:</br>(one per line)</td><td><textarea rows="10" cols="100"  name="presets" placeholder="First Preset Name = spotify:user:spotify:playlist:37i9dQZEVXboyJ0IJdpcuT"></textarea></td></tr>
<tr><td style="width:150px;">Quick-Save URI:</br>(playlist URI)</td><td>
    <input type="text" name="quicksave_default" style="width:400px;"></td></tr>
<tr><td colspan=2></br></br></tr></tr>
<tr><td colspan=2><center><input type="submit" value="Save Configuration" style="height:50px; width: 300px; font-size:20px;"></center></td></tr>
</br>
</table></form>
</br>
<small>Config will be saved as: <em>{}</em></br>
If something goes wrong or changes, edit or delete that file.</small>
</body></html>
"###,
                       "HTTP/1.1 200 OK\r\n\r\n",
                       PORT,
                       default_inifile());
    let reply = format!("{}Configuration saved.  You can close this window.",
                        "HTTP/1.1 200 OK\r\n\r\n");
    let mut config = BTreeMap::<String,String>::new();
    config.insert("port".to_string(), PORT.to_string());
    config.append(&mut http::config_request_local_webserver(WEB_PORT, form, reply));
    config
}

pub fn request_web_alarm_config() -> BTreeMap<String,String> {
    let form = format!(r###"
{}
<!DOCTYPE HTML>
<html><head><title>Connectr Alarm Clocks</title><style> tr:nth-child(even) {{ background: #f2f2f2; }} </style></head>
<body><h2>Connectr Alarm Clocks</h2> Input in 24-hour time format:
<form method="POST" action="#" accept-charset="UTF-8"><table>
<tr><th style="width:100px; border-bottom: 1px solid #ddd;" align="center">Time</th><th align="center" style="width:100px;border-bottom: 1px solid #ddd;">Repeat</th></tr>

<tr><td align="center"><input style="width:40px;" type="number" name="hour_0" min="0" max="23" size="3" maxlength="2">:<input style="width:40px;" type="number" name="minute_0" min="0" max="59" size="3" maxlength="2"></td><td align="center"><select name="repeat_0"><option value="daily">Daily</option><option value="weekdays">Weekdays</option><option value="weekends">Weekends</option></select></td></tr>

<tr><td align="center"><input style="width:40px;" type="number" name="hour_1" min="0" max="23" size="3" maxlength="2">:<input style="width:40px;" type="number" name="minute_1" min="0" max="59" size="3" maxlength="2"></td><td align="center"><select name="repeat_1"><option value="daily">Daily</option><option value="weekdays">Weekdays</option><option value="weekends">Weekends</option></select></td></tr>

<tr><td align="center"><input style="width:40px;" type="number" name="hour_2" min="0" max="23" size="3" maxlength="2">:<input style="width:40px;" type="number" name="minute_2" min="0" max="59" size="3" maxlength="2"></td><td align="center"><select name="repeat_2"><option value="daily">Daily</option><option value="weekdays">Weekdays</option><option value="weekends">Weekends</option></select></td></tr>

<tr><td align="center"><input style="width:40px;" type="number" name="hour_3" min="0" max="23" size="3" maxlength="2">:<input style="width:40px;" type="number" name="minute_3" min="0" max="59" size="3" maxlength="2"></td><td align="center"><select name="repeat_3"><option value="daily">Daily</option><option value="weekdays">Weekdays</option><option value="weekends">Weekends</option></select></td></tr>

<tr><td align="center"><input style="width:40px;" type="number" name="hour_4" min="0" max="23" size="3" maxlength="2">:<input style="width:40px;" type="number" name="minute_4" min="0" max="59" size="3" maxlength="2"></td><td align="center"><select name="repeat_4"><option value="daily">Daily</option><option value="weekdays">Weekdays</option><option value="weekends">Weekends</option></select></td></tr>

<tr><td colspan=2><center><input type="submit" value="Save Configuration" style="height:50px; width: 300px; font-size:20px;"></center></td></tr></br>
</table></form>
</body></html>
"###, "HTTP/1.1 200 OK\r\n\r\n");
    let reply = format!("{}Configuration saved.  You can close this window.",
                        "HTTP/1.1 200 OK\r\n\r\n");
    let mut config = BTreeMap::<String,String>::new();
    config.append(&mut http::config_request_local_webserver(WEB_PORT, form, reply));
    config
}

pub fn save_web_config(mut config: BTreeMap<String,String>) -> Ini {
    let mut c = Ini::new();
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
        // are you kidding me??
        let idx = key.chars().rev().take(1).collect::<Vec<char>>()[0].to_digit(10).unwrap() as usize;
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
            _ => {},
        }
    }
    for i in 0..5 {
        info!("Entry {}: {:?}", i, entries.get(i).unwrap());
    }
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
            let web_config = request_web_config();
            save_web_config(web_config)
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

    Some(Settings { secret: secret.to_string(), client_id: client_id.to_string(), port: port,
                    access_token: access, refresh_token: refresh, expire_utc: expire_utc,
                    presets: presets,
                    default_quicksave: quicksave_default,
                    quicksave: quicksave,
                    alarms: alarms,
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
