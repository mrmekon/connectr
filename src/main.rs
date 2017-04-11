
extern crate connectr;
use connectr::settings;
use connectr::SpotifyResponse;

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

fn require(response: SpotifyResponse) {
    match response.code.unwrap() {
        200 ... 299 => (),
        _ => panic!("{}", response)
    }
}

fn main() {
    let settings = match settings::read_settings() {
        Some(s) => s,
        None => process::exit(0),
    };
    let mut spotify = connectr::SpotifyConnectr::new(settings);
    spotify.connect();

    let device_list = spotify.request_device_list();
    let player_state = spotify.request_player_state();

    println!("Devices:\n{}", device_list);
    println!("State:\n{}", player_state);

    let ctx = connectr::PlayContext::new()
        .context_uri("spotify:user:mrmekon:playlist:4XqYlbPdDUsranzjicPCgf")
        .offset_position(2)
        .build();

    spotify.set_target_device(None);
    require(spotify.play(Some(&ctx)));
    require(spotify.pause());
    require(spotify.seek(5000));
}
