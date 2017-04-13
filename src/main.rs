extern crate connectr;
use connectr::SpotifyResponse;

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
    let mut spotify = connectr::SpotifyConnectr::new();
    spotify.connect();

    let device_list = spotify.request_device_list();
    let player_state = spotify.request_player_state();

    println!("Visible Devices:");
    for dev in device_list {
        println!("{}", dev);
    }
    println!("");

    println!("Playback State:\n{}", player_state);

    let ctx = connectr::PlayContext::new()
        .context_uri("spotify:user:mrmekon:playlist:4XqYlbPdDUsranzjicPCgf")
        .offset_position(2)
        .build();

    spotify.set_target_device(None);
    require(spotify.play(Some(&ctx)));
    require(spotify.pause());
    require(spotify.next());
    require(spotify.previous());
    require(spotify.seek(5000));
    require(spotify.volume(10));
    require(spotify.shuffle(true));
    require(spotify.repeat(connectr::SpotifyRepeat::Context));
    require(spotify.transfer_multi(vec!["1a793f2a23989a1c35d05b2fd1ff00e9a67e7134".to_string()], false));
    require(spotify.transfer("1a793f2a23989a1c35d05b2fd1ff00e9a67e7134".to_string(), false));

    let player_state = spotify.request_player_state();
    println!("Final state:\n{}", player_state);

    loop {
        spotify.await_once(true);
    }
}
