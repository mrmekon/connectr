extern crate ini;
use self::ini::Ini;

extern crate time;

#[cfg(target_os = "macos")]
use super::osx;

use std::env;
use std::fs;
use std::path;

const INIFILE: &'static str = "connectr.ini";

pub struct Settings {
    pub port: u32,
    pub secret: String,
    pub client_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expire_utc: Option<u64>,
    pub presets: Vec<(String,String)>,
}

#[cfg(target_os = "macos")]
fn bundled_ini() -> String {
    match osx::bundled_resource_path("connectr", "ini") {
        Some(path) => path,
        None => String::new(),
    }
}

#[cfg(not(target_os = "macos"))]
fn bundled_ini() -> String {
    String::new()
}

fn inifile() -> String {
    // Try to load INI file from home directory
    let path = format!("{}/.{}", env::home_dir().unwrap().display(), INIFILE);
    if path::Path::new(&path).exists() {
        info!("Found config: {}", path);
        return path;
    }

    // If it doesn't exist, try to copy the template from the app bundle, if
    // such a thing exists.
    let bundle_ini = bundled_ini();
    if path::Path::new(&bundle_ini).exists() {
        info!("Copied config: {}", bundle_ini);
        let _ = fs::copy(bundle_ini, path.clone());
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

pub fn read_settings() -> Option<Settings> {
    info!("Attempting to read config file.");
    let conf = match Ini::load_from_file(&inifile()) {
        Ok(c) => c,
        Err(e) => {
            info!("Load file error: {}", e);
            // No connectr.ini found.  Generate a junk one in-memory, which
            // will fail shortly after with the nice error message.
            let mut c = Ini::new();
            info!("No config file found.");
            c.with_section(Some("connectr".to_owned()))
                .set("port", 5657.to_string());
            c.with_section(Some("application".to_owned()))
                .set("secret", "<PLACEHOLDER>".to_string())
                .set("client_id", "<PLACEHOLDER>".to_string());
            c
        }
    };

    let section = conf.section(Some("connectr".to_owned())).unwrap();
    let port = section.get("port").unwrap().parse().unwrap();

    let section = conf.section(Some("application".to_owned())).unwrap();
    let secret = section.get("secret").unwrap();
    let client_id = section.get("client_id").unwrap();
    if client_id.starts_with('<') || secret.starts_with('<') {
        error!("Invalid or missing configuration.  Cannot continue.");
        println!("");
        println!("ERROR: Spotify Client ID or Secret not set in connectr.ini!");
        println!("");
        println!("Create a Spotify application at https://developer.spotify.com/my-applications/ and");
        println!("add the client ID and secret to connectr.ini.");
        println!("");
        println!("Be sure to add a redirect URI of http://127.0.0.1:<PORT> to your Spotify application,");
        println!("and make sure the port matches in connectr.ini.");
        println!("");
        return None;
    }

    let mut access = None;
    let mut refresh = None;
    let mut expire_utc = None;
    if let Some(section) = conf.section(Some("tokens".to_owned())) {
        access = Some(section.get("access").unwrap().clone());
        refresh = Some(section.get("refresh").unwrap().clone());
        expire_utc = Some(section.get("expire").unwrap().parse().unwrap());
        println!("Read access token from INI!");
    }

    let mut presets = Vec::<(String,String)>::new();
    if let Some(section) = conf.section(Some("presets".to_owned())) {
        for (key, value) in section {
            presets.push((key.to_owned(), value.to_owned()));
        }
    }

    Some(Settings { secret: secret.to_string(), client_id: client_id.to_string(), port: port,
                    access_token: access, refresh_token: refresh, expire_utc: expire_utc,
                    presets: presets})
}

pub type SettingsError = String;
pub fn save_tokens(access: &str, refresh: &str, expire_utc: u64) -> Result<(), SettingsError> {
    let mut conf = Ini::load_from_file(&inifile()).unwrap();
    conf.with_section(Some("tokens".to_owned()))
        .set("access", access)
        .set("refresh", refresh)
        .set("expire", expire_utc.to_string());
    conf.write_to_file(&inifile()).unwrap();
    Ok(())
}
