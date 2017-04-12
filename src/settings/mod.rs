extern crate ini;
use self::ini::Ini;

extern crate time;

pub struct Settings {
    pub port: u32,
    pub secret: String,
    pub client_id: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub expire_utc: Option<u64>,
}

pub fn read_settings() -> Option<Settings> {
    let conf = Ini::load_from_file("connectr.ini").unwrap();

    let section = conf.section(Some("connectr".to_owned())).unwrap();
    let port = section.get("port").unwrap().parse().unwrap();

    let section = conf.section(Some("application".to_owned())).unwrap();
    let secret = section.get("secret").unwrap();
    let client_id = section.get("client_id").unwrap();
    if client_id.starts_with('<') || secret.starts_with('<') {
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

    Some(Settings { secret: secret.to_string(), client_id: client_id.to_string(), port: port,
                    access_token: access, refresh_token: refresh, expire_utc: expire_utc})
}

pub type SettingsError = String;
pub fn save_tokens(access: &str, refresh: &str, expire: u64) -> Result<(), SettingsError> {
    let mut conf = Ini::load_from_file("connectr.ini").unwrap();
    let now = time::now_utc().to_timespec().sec as u64;
    let expire_utc = now + expire;
    conf.with_section(Some("tokens".to_owned()))
        .set("access", access)
        .set("refresh", refresh)
        .set("expire", expire_utc.to_string());
    conf.write_to_file("connectr.ini").unwrap();
    Ok(())
}
