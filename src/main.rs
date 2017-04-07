mod spotify_api {
    pub const SCOPES: &'static [&'static str] = &["user-read-private", "streaming", "user-read-playback-state"];
    pub const AUTHORIZE: &'static str = "https://accounts.spotify.com/en/authorize";
    pub const TOKEN: &'static str = "https://accounts.spotify.com/api/token";
    pub const DEVICES: &'static str = "https://api.spotify.com/v1/me/player/devices";
}

#[derive(PartialEq)]
enum HttpMethod {
    GET,
    POST,
}

extern crate curl;
extern crate open;
extern crate regex;
extern crate rustc_serialize;
extern crate url;
extern crate ini;

use std::net::{TcpListener};
use std::io::{Read, Write, BufReader, BufRead};

use curl::easy::{Easy, List};
use regex::Regex;
use rustc_serialize::json::Json;
use rustc_serialize::json;
use rustc_serialize::Decodable;
use rustc_serialize::Decoder;
use url::percent_encoding;
use ini::Ini;

fn oauth_request_with_local_webserver(port: u32, url: &str, reply: &str) -> Vec<String> {
    if !open::that(url).is_ok() {
        return Vec::<String>::new()
    }
    let host = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(host).unwrap();
    let stream = listener.accept().unwrap().0;
    let mut reader = BufReader::new(stream);
    let mut response = Vec::<String>::new();
    for line in reader.by_ref().lines() {
        let line_str = line.unwrap();
        response.push(line_str.clone());
        if line_str == "" {
            break;
        }
    }
    let _ = reader.into_inner().write(reply.as_bytes());
    response
}

fn spotify_auth_code(lines: Vec<String>) -> String {
    let mut auth_code = String::new();
    for line in lines {
        let line_str = line;
        let re = Regex::new(r"code=([^?& ]+)").unwrap();
        let ismatch = re.is_match(line_str.as_str());
        if ismatch {
            let cap = re.captures(line_str.as_str()).unwrap();
            auth_code = auth_code + &cap[1];
        }
    }
    auth_code
}

macro_rules! vec_of_strings {
    ($($x:expr),*) => (vec![$($x.to_string()),*]);
}

fn authenticate(settings: &Settings) -> String {
    let scopes = spotify_api::SCOPES.join(" ");
    let host = format!("http://127.0.0.1:{}", settings.port);
    let url = format!("{}?client_id={}&response_type=code&scope={}&redirect_uri={}",
                      spotify_api::AUTHORIZE,settings.client_id, scopes, host);
    let query = percent_encoding::utf8_percent_encode(&url, percent_encoding::QUERY_ENCODE_SET).collect::<String>();
    let response = "HTTP/1.1 200 OK\r\n\r\nAuthenticated with Spotify.\r\n\r\nYou can close this window.\r\n";
    let auth_lines = oauth_request_with_local_webserver(settings.port, &query, response);
    let auth_code = spotify_auth_code(auth_lines);
    auth_code
}

fn http(url: &str, query: &str, method: HttpMethod, access_token: Option<&str>) -> String {
    let mut data = query.as_bytes();
    let mut json_bytes = Vec::<u8>::new();
    {
        let mut easy = Easy::new();
        match method {
            HttpMethod::POST => {
                easy.url(url).unwrap();
                easy.post(true).unwrap();
                easy.post_field_size(data.len() as u64).unwrap();
            }
            _ => {
                let get_url = format!("{}?{}", url, query);
                easy.url(&get_url).unwrap();
            }
        }

        match access_token {
            Some(access_token) => {
                let mut list = List::new();
                let header = format!("Authorization: Bearer {}", access_token);
                list.append(&header).unwrap();
                easy.http_headers(list).unwrap();
            }
            None => {}
        }

        {
            let mut transfer = easy.transfer();
            if method == HttpMethod::POST {
                transfer.read_function(|buf| {
                    Ok(data.read(buf).unwrap_or(0))
                }).unwrap();
            }
            transfer.write_function(|x| {
                json_bytes.extend(x);
                Ok(x.len())
            }).unwrap();
            transfer.perform().unwrap();
        }
    }
    String::from_utf8(json_bytes).unwrap()
}

fn parse_spotify_token(json: &str) -> (String, String) {
    let json_data = Json::from_str(&json).unwrap();
    let obj = json_data.as_object().unwrap();
    let access_token = obj.get("access_token").unwrap().as_string().unwrap();
    let refresh_token = obj.get("refresh_token").unwrap().as_string().unwrap();
    (String::from(access_token),String::from(refresh_token))
}

//#[derive(RustcDecodable, RustcEncodable, Debug)]
#[derive(RustcEncodable, Debug)]
struct ConnectDevice {
    id: String,
    is_active: bool,
    is_restricted: bool,
    name: String,
    device_type: String,
    volume_percent: u32
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
struct ConnectDeviceList {
    devices: Vec<ConnectDevice>
}

struct Settings {
    port: u32,
    secret: String,
    client_id: String,
}

fn read_settings() -> Settings {
    let conf = Ini::load_from_file("connectr.ini").unwrap();

    let section = conf.section(Some("connectr".to_owned())).unwrap();
    let port = section.get("port").unwrap().parse().unwrap();

    let section = conf.section(Some("application".to_owned())).unwrap();
    let secret = section.get("secret").unwrap();
    let client_id = section.get("client_id").unwrap();
    Settings { secret: secret.to_string(), client_id: client_id.to_string(), port: port }
}

fn main() {
    let settings = read_settings();
    let auth_code = authenticate(&settings);
    let query = format!("grant_type=authorization_code&code={}&redirect_uri=http://127.0.0.1:{}&client_id={}&client_secret={}",
                        auth_code, settings.port, settings.client_id, settings.secret);
    let query = percent_encoding::utf8_percent_encode(&query, percent_encoding::QUERY_ENCODE_SET).collect::<String>();
    let json_response = http(spotify_api::TOKEN, &query, HttpMethod::POST, None);
    let (access_token, refresh_token) = parse_spotify_token(&json_response);

    let json_response = http(spotify_api::DEVICES, "", HttpMethod::GET, Some(&access_token));

    let decoded: ConnectDeviceList = json::decode(&json_response).unwrap();

    println!("Auth Code: {}...", &auth_code[0..5]);
    println!("Access: {}... / Refresh: {}...", &access_token[0..5], &refresh_token[0..5]);
    for dev in decoded.devices {
        println!("{:?}", dev);
    }
}
