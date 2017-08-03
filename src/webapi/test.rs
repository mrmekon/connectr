#[cfg(test)]
mod tests {
    extern crate futures;
    extern crate hyper;
    extern crate fruitbasket;
    extern crate time;

    use super::super::*;
    use super::super::super::SpotifyEndpoints;

    use std;
    use std::thread;
    use std::thread::sleep;
    use std::sync::atomic::AtomicBool;
    use std::sync::atomic::Ordering;
    use std::time::Duration;
    use std::sync::{Once, ONCE_INIT};

    use self::hyper::{Post, StatusCode};
    use self::hyper::server::{Service, Request, Response};
    use self::hyper::server::Http;
    use self::futures::Stream;
    use self::futures::Future;

    static START: Once = ONCE_INIT;

    lazy_static! {
        static ref WEBSERVER_STARTED: AtomicBool = AtomicBool::new(false);
    }

    pub const TEST_API: SpotifyEndpoints = SpotifyEndpoints {
        scopes: "user-read-private streaming user-read-playback-state",
        authorize: "http://127.0.0.1:9799/en/authorize",
        token: "http://127.0.0.1:9799/api/token",
        devices: "http://127.0.0.1:9799/v1/me/player/devices",
        player_state: "http://127.0.0.1:9799/v1/me/player",
        play: "http://127.0.0.1:9799/v1/me/player/play",
        pause: "http://127.0.0.1:9799/v1/me/player/pause",
        next: "http://127.0.0.1:9799/v1/me/player/next",
        previous: "http://127.0.0.1:9799/v1/me/player/previous",
        seek: "http://127.0.0.1:9799/v1/me/player/seek",
        volume: "http://127.0.0.1:9799/v1/me/player/volume",
        shuffle: "http://127.0.0.1:9799/v1/me/player/shuffle",
        repeat: "http://127.0.0.1:9799/v1/me/player/repeat",
        player: "http://127.0.0.1:9799/v1/me/player",
    };

    /// Macro to parse the body of a POST request and send a response.
    ///
    /// There's probably a "body.to_string()" function somewhere.  I didn't find it.
    /// So instead there's this unreadable, overly complicated bullshit.
    ///
    /// $body_in: a POST body (hyper::Body) from a received POST request
    /// $pairs_out: the name of the key/value pair variable provided to the $block_in
    /// $block_in: a block of code to be executed, with $pairs_out in scope, that evaluates
    ///            to tuple (status_code: StatusCode, body: &str) to send as a response.
    macro_rules! post {
    ($body_in:ident, $pairs_out:ident, $block_in:block) => {
        {
            // Read chunks from user provided body var $body_in
            $body_in.fold(vec![], |mut acc, chunk| {
                acc.extend(chunk);
                Ok::<_, hyper::Error>(acc)
            }).and_then(move |bytes| {
                // [u8] -> String
                let post_data: String = std::str::from_utf8(&bytes).unwrap().to_string();;
                // Split on & to get ["key=value"...]
                let pairs = post_data.split("&");
                // Split on = to get [[key,value]...], put in user provided var name $pairs_out
                let $pairs_out = pairs.map(|pair| pair.split("=").collect::<Vec<&str>>()).collect::<Vec<Vec<&str>>>();
                // User provided block takes $pairs_out and returns response string
                let (code, response) = $block_in;
                let res = Response::new();
                Ok(res.with_status(code).with_body(response))
            }).boxed()
        }
    };
    }

    fn token_response(pairs: &Vec<Vec<&str>>) -> (StatusCode, String) {
        let mut resp = String::new();
        let mut code = StatusCode::Ok;
        resp.push_str("{");
        resp.push_str(r#""access_token": "valid_access_code","#);
        resp.push_str(r#""token_type": "Bearer","#);
        resp.push_str(r#""scope": "user-read-private user-read-email","#);
        resp.push_str(r#""expires_in": 3600"#);
        resp.push_str("}");
        for pair in pairs {
            let (key,value) = (pair[0], pair[1]);
            if key == "refresh_token" && value == "error" {
                code = StatusCode::Forbidden;
            }
        }
        (code, resp)
    }

    fn init() {
        while !WEBSERVER_STARTED.load(Ordering::Relaxed) {
            sleep(Duration::from_millis(100));
        }
        START.call_once(|| {
            #[derive(Clone, Copy)]
            struct Webapi;
            impl Service for Webapi {
                type Request = Request;
                type Response = Response;
                type Error = hyper::Error;
                type Future = futures::BoxFuture<Response, hyper::Error>;
                fn call(&self, req: Request) -> Self::Future {
                    let (method, uri, _, _headers, body) = req.deconstruct();
                    match(method, uri.path()) {
                        (Post, "/api/token") => post!(body, pairs, { token_response(&pairs) }),
                        _ => futures::future::ok(Response::new().with_status(StatusCode::NotFound)).boxed(),
                    }
                }
            }
            thread::spawn(move || {
                let addr = "127.0.0.1:9799".parse().unwrap();
                let server = Http::new().bind(&addr, || Ok(Webapi)).unwrap();
                server.run().unwrap();
                WEBSERVER_STARTED.store(true, Ordering::Relaxed);
            });
        });
    }

    #[test]
    fn test_refresh_oauth_tokens_no_connection() {
        let now = time::now_utc().to_timespec().sec as u64;
        let spotify = SpotifyConnectr::new()
            .with_api(TEST_API)
            .with_oauth_tokens("access", "refresh", now + 3600)
            .build()
            .unwrap();
        let res = spotify.refresh_oauth_tokens();
        // Unlock webserver init so all other tests can run
        WEBSERVER_STARTED.store(true, Ordering::Relaxed);
        assert!(res.is_none());
    }

    #[test]
    fn test_refresh_oauth_tokens_pass() {
        init();
        let now = time::now_utc().to_timespec().sec as u64;
        let spotify = SpotifyConnectr::new()
            .with_api(TEST_API)
            .with_oauth_tokens("access", "refresh", now + 3600)
            .build()
            .unwrap();
        match spotify.refresh_oauth_tokens() {
            Some((access,expires)) => {
                assert_eq!(access, "valid_access_code");
                assert_eq!(expires, 3600);
            },
            None => { assert!(false) },
        }
    }

    #[test]
    fn test_refresh_oauth_tokens_error_status() {
        init();
        let now = time::now_utc().to_timespec().sec as u64;
        let mut spotify = SpotifyConnectr::new()
            .with_api(TEST_API)
            .with_oauth_tokens("access", "refresh", now + 3600)
            .build()
            .unwrap();
        spotify.refresh_token = Some("error".to_string());
        match spotify.refresh_oauth_tokens() {
            Some(_) => { assert!(false) },
            None => { },
        }
    }

}
