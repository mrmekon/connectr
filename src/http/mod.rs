use std::fmt;
use std::error::Error;
use std::str;
use std::io::{Read, Write, BufReader, BufRead};
use std::net::{TcpListener};

extern crate regex;
use self::regex::Regex;

extern crate curl;
use self::curl::easy::{Easy, List};

extern crate open;
extern crate url;
use self::url::percent_encoding;

use super::settings;
use super::spotify_api;

#[derive(PartialEq)]
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

pub enum AccessToken<'a> {
    Bearer(&'a str),
    Basic(&'a str),
    None,
}

pub fn http(url: &str, query: &str, body: &str,
            method: HttpMethod, access_token: AccessToken) -> HttpResponse {
    let enc_query = percent_encoding::utf8_percent_encode(&query, percent_encoding::QUERY_ENCODE_SET).collect::<String>();
    let mut data = match method {
        HttpMethod::POST => { enc_query.as_bytes() },
        _ => { body.as_bytes() },
        //_ => { query.as_bytes() }
    };
    let query_url = &format!("{}?{}", url, query);
    let url = match method {
        HttpMethod::GET | HttpMethod::PUT => match query.len() {
            0 => url,
            _ => query_url,
        },
        _ => url

    };
    let mut response = None;
    let mut json_bytes = Vec::<u8>::new();
    {
        let mut easy = Easy::new();
        easy.url(url).unwrap();
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
                let mut list = List::new();
                let header = format!("Authorization: {} {}", request.0, request.1);
                list.append(&header).unwrap();
                easy.http_headers(list).unwrap();
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
    println!("HTTP response: {}", result.clone().unwrap());
    HttpResponse {code: response, data: result }
}

pub fn authenticate(settings: &settings::Settings) -> String {
    let scopes = spotify_api::SCOPES.join(" ");
    let host = format!("http://127.0.0.1:{}", settings.port);
    let url = format!("{}?client_id={}&response_type=code&scope={}&redirect_uri={}",
                      spotify_api::AUTHORIZE,settings.client_id, scopes, host);
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
