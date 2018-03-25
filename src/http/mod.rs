use std::fmt;
use std::error::Error;
use std::str;
use std::io::{Read, Write, BufReader, BufRead};
use std::net::{TcpListener};
use std::thread;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::collections::BTreeMap;

extern crate regex;
use self::regex::Regex;

extern crate curl;
use self::curl::easy::{Easy, List};

extern crate time;
extern crate open;
extern crate url;
use self::url::percent_encoding;

use super::settings;

#[derive(PartialEq, Debug)]
pub enum HttpMethod {
    GET,
    POST,
    PUT,
}

pub type HttpErrorString = String;
pub struct HttpResponse {
    pub code: Option<u32>,
    pub data: Result<String, HttpErrorString>,
}

impl HttpResponse {
    pub fn unwrap(self) -> String { self.data.unwrap() }
    pub fn print(&self) {
        let code: i32 = match self.code {
            Some(x) => { x as i32 }
            None => -1
        };
        println!("Code: {}", code);
        match self.data {
            Ok(ref s) => {println!("{}", s)}
            Err(ref s) => {println!("ERROR: {}", s)}
        }
    }
}

impl fmt::Display for HttpResponse {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let code: i32 = match self.code {
            Some(x) => { x as i32 }
            None => -1
        };
        let _ = write!(f, "Code: {}\n", code);
        match self.data {
            Ok(ref s) => {write!(f, "Response: {}", s)}
            Err(ref s) => {write!(f, "ERROR: {}", s)}
        }
    }
}

#[derive(Debug)]
pub enum AccessToken<'a> {
    Bearer(&'a str),
    Basic(&'a str),
    None,
}

pub fn http(url: &str, query: Option<&str>, body: Option<&str>,
            method: HttpMethod, access_token: AccessToken) -> HttpResponse {
    let mut headers = List::new();
    #[cfg(feature = "verbose_http")]
    info!("HTTP URL: {:?} {}\nQuery: {:?}\nBody: {:?}\nToken: {:?}", method, url, query, body, access_token);
    let data = match method {
        HttpMethod::POST => {
            match query {
                Some(q) => {
                    let enc_query = percent_encoding::utf8_percent_encode(&q, percent_encoding::QUERY_ENCODE_SET).collect::<String>();
                    enc_query
                },
                None => {
                    let header = format!("Content-Type: application/json");
                    headers.append(&header).unwrap();
                    body.unwrap_or("").to_string()
                }
            }
        },
        _ => { body.unwrap_or("").to_string() },
    };
    let mut data = data.as_bytes();

    let url = match method {
        HttpMethod::GET | HttpMethod::PUT => match query {
            None => url.to_string(),
            Some(q) => format!("{}?{}", url, q),
        },
        _ => url.to_string()

    };
    let mut response = None;
    let mut json_bytes = Vec::<u8>::new();
    {
        let mut easy = Easy::new();
        easy.url(&url).unwrap();
        match method {
            HttpMethod::POST => {
                easy.post(true).unwrap();
                easy.post_field_size(data.len() as u64).unwrap();
            }
            HttpMethod::PUT => {
                easy.put(true).unwrap();
                easy.post_field_size(data.len() as u64).unwrap();
            }
            _ => {}
        }

        match access_token {
            AccessToken::None => {},
            access_token => {
                let request = match access_token {
                    AccessToken::Bearer(token) => ("Bearer", token),
                    AccessToken::Basic(token) => ("Basic", token),
                    _ => ("",""),
                };
                let header = format!("Authorization: {} {}", request.0, request.1);
                headers.append(&header).unwrap();
                easy.http_headers(headers).unwrap();
            }
        }

        {
            let mut transfer = easy.transfer();
            if method == HttpMethod::POST || method == HttpMethod::PUT {
                transfer.read_function(|buf| {
                    Ok(data.read(buf).unwrap_or(0))
                }).unwrap();
            }
            transfer.write_function(|x| {
                json_bytes.extend(x);
                Ok(x.len())
            }).unwrap();
            match transfer.perform() {
                Err(x) => {
                    let result: Result<String,String> = Err(x.description().to_string());
                    #[cfg(feature = "verbose_http")]
                    warn!("HTTP response: err: {}", x.description().to_string());
                    return HttpResponse {code: response, data: result }
                }
                _ => {}
            };
        }
        response = match easy.response_code() {
            Ok(code) => { Some(code) }
            _ => { None }
        };
    }
    let result: Result<String,String> = match String::from_utf8(json_bytes) {
        Ok(x) => { Ok(x) }
        Err(x) => { Err(x.utf8_error().description().to_string()) }
    };
    #[cfg(feature = "verbose_http")]
    info!("HTTP response: {}", result.clone().unwrap());
    HttpResponse {code: response, data: result }
}

pub fn authenticate(scopes: &str, url: &str, settings: &settings::Settings) -> String {
    let host = format!("http://127.0.0.1:{}", settings.port);
    let url = format!("{}?client_id={}&response_type=code&scope={}&redirect_uri={}",
                      url,settings.client_id, scopes, host);
    let query = percent_encoding::utf8_percent_encode(&url, percent_encoding::QUERY_ENCODE_SET).collect::<String>();
    let response = "HTTP/1.1 200 OK\r\n\r\n<html><body>
Authenticated with Spotify.<br/><br/>
You can close this window.<br/><br/>
<button type=\"button\" onclick=\"window.open('', '_self', ''); window.close();\">Close</button><br/>
</body></html>";
    let auth_lines = oauth_request_with_local_webserver(settings.port, &query, response);
    let auth_code = spotify_auth_code(auth_lines);
    auth_code
}

fn oauth_request_with_local_webserver(port: u32, url: &str, reply: &str) -> Vec<String> {
    if !open::that(url).is_ok() {
        return Vec::<String>::new()
    }
    let start = time::now_utc().to_timespec().sec as i64;
    let host = format!("127.0.0.1:{}", port);
    let listener = TcpListener::bind(host);
    if listener.is_err() {
        return Vec::<String>::new();
    }
    let timeout_sec = 20;
    let listener = listener.unwrap();
    let _ = listener.set_nonblocking(true);
    loop {
        let conn = listener.accept();
        if conn.is_err() {
            let now = time::now_utc().to_timespec().sec as i64;
            if now >= start + timeout_sec {
                warn!("Spotify OAuth request timed out.");
                break;
            }
            thread::sleep(Duration::from_millis(100));
            continue;
        }
        let stream = conn.unwrap().0;
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
        return response;
    }
    Vec::<String>::new()
}

pub fn config_request_local_webserver(port: u32, form: String, reply: String) -> BTreeMap<String,String> {
    let mut config = BTreeMap::<String,String>::new();
    let (tx,rx) = channel::<Option<(String,String)>>();
    let (tx_kill,rx_kill) = channel::<()>();
    thread::spawn(move || {
        // Implement a custom HTTP server, because YOU'RE NOT MY MOM.
        let host = format!("127.0.0.1:{}", port);
        let listener = TcpListener::bind(host);
        if listener.is_err() {
            let _ = tx.send(None); // Start data transfer
            let _ = tx.send(None); // End data transfer
            return;
        }
        let listener = listener.unwrap();
        let _ = listener.set_nonblocking(true);
        loop {
            let conn = listener.accept();
            if conn.is_err() {
                match rx_kill.try_recv() {
                    Ok(_) => { break;},
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    },
                }
            }
            let stream = conn.unwrap().0;
            let mut reader = BufReader::new(stream);
            let mut response = Vec::<String>::new();
            let mut post_bytes: u32 = 0;
            let re = Regex::new(r"Content-Length: ([0-9 ]+)").unwrap();
            for line in reader.by_ref().lines() {
                if let Ok(line_str) = line {
                    if re.is_match(line_str.as_str()) {
                        post_bytes = re.captures(line_str.as_str()).unwrap()[1].parse::<u32>().unwrap();
                    }
                    response.push(line_str.clone());
                    if line_str == "" {
                        break;
                    }
                }
            }
            match post_bytes {
                x if x > 0  => {
                    // Tell parent thread data is coming.  Cancels timeout mechanism.
                    let _ = tx.send(None);
                    {
                        let mut post_reader = reader.by_ref().take(post_bytes as u64);
                        let mut post_data = Vec::<u8>::new();
                        let _ = post_reader.read_to_end(&mut post_data);
                        let post_data = String::from_utf8(post_data).unwrap();
                        for post_pair in post_data.split("&") {
                        let mut key_value = post_pair.split("=");
                            let key = key_value.next().unwrap();
                            let value = key_value.next().unwrap();
                            let _ = tx.send(Some((key.to_string(),value.to_string())));
                        }
                    }
                    let _ = reader.into_inner().write(reply.as_bytes());
                    // Tell parent thread that data is finished.
                    let _ = tx.send(None);
                    break;
                },
                _ => {
                    let _ = reader.into_inner().write(form.as_bytes());
                }
            }
        }
    });
    if !open::that(format!("http://127.0.0.1:{}", port)).is_ok() {
        return config;
    }
    // Run web server for an hour.
    // In a proper world, this would be an async 'future'.  The whole Spotify
    // thread is blocked until the user saves.
    let timeout = Duration::from_secs(60*60);
    match rx.recv_timeout(timeout) {
        Ok(_) => {
            while let Some(pair) = rx.recv().unwrap() {
                let key = pair.0.replace("+"," ").trim().to_string();
                let key = percent_encoding::percent_decode(key.as_bytes()).decode_utf8_lossy();
                let value = pair.1.replace("+"," ").trim().to_string();
                let value = percent_encoding::percent_decode(value.trim().as_bytes()).decode_utf8_lossy();
                config.insert(key.to_string(), value.to_string());
            }
        }
        _ => {
            warn!("Web configuration timed out.");
            let _ = tx_kill.send(());
        }
    }
    config
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
