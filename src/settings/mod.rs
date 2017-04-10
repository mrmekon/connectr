extern crate ini;
use self::ini::Ini;

pub struct Settings {
    pub port: u32,
    pub secret: String,
    pub client_id: String,
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
    Some(Settings { secret: secret.to_string(), client_id: client_id.to_string(), port: port })
}
