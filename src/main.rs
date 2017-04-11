
extern crate connectr;
use connectr::settings;

use std::process;

//extern crate systray;

//#[cfg(target_os = "windows")]
//fn systray(player_state: PlayerState) {
//    let mut app;
//    match systray::Application::new() {
//        Ok(w) => app = w,
//        Err(e) => panic!("Can't create systray window.")
//    }
//    let mut w = &mut app.window;
//    let _ = w.set_icon_from_file(&"spotify.ico".to_string());
//    let _ = w.set_tooltip(&"Whatever".to_string());
//    let _ = w.add_menu_item(&"Print a thing".to_string(), |window| {
//        println!("Printing a thing!");
//    });
//    println!("Waiting on message!");
//    w.wait_for_message();
//}

fn main() {
    let settings = match settings::read_settings() {
        Some(s) => s,
        None => process::exit(0),
    };
    let mut spotify = connectr::SpotifyConnectr::new(settings);
    spotify.connect();

    let device_list = spotify.request_device_list();
    let player_state = spotify.request_player_state();

    for dev in device_list.devices {
        println!("{:?}", dev);
    }
    println!("State: {:?}", player_state);

    let offset = connectr::PlayContextOffset {
        position: Some(5), uri: Some("blah".to_string()),
    };
    let ctx = connectr::PlayContext {
        context_uri: Some("spotify:user:mrmekon:playlist:4XqYlbPdDUsranzjicPCgf".to_string()),
        uris: Some(vec!["one".to_string(), "two".to_string()]),
        offset: Some(offset),
    };
    spotify.play(Some("deviceid".to_string()), Some(&ctx));
    spotify.pause(Some("deviceid".to_string()));
    spotify.play(None, Some(&ctx));
    spotify.pause(None);
    spotify.play(None, None);

    //systray(player_state);
    //loop {}
}
