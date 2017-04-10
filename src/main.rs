
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
    spotify.go();

    //systray(player_state);
    //loop {}
}
