extern crate connectr;
use connectr::SpotifyResponse;

use std::ptr;
use std::thread::sleep;
use std::time::Duration;
use std::sync::mpsc::channel;

extern crate rustc_serialize;
use rustc_serialize::json;

#[derive(RustcDecodable, RustcEncodable, Debug)]
enum CallbackAction {
    SelectDevice,
    PlayPause,
    Preset,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
struct MenuCallbackCommand {
    action: CallbackAction,
    sender: u64,
    data: String,
}

#[cfg(target_os = "macos")]
use connectr::osx;
#[cfg(target_os = "macos")]
use connectr::osx::TStatusBar;
#[cfg(target_os = "macos")]
use connectr::osx::MenuItem;

struct MenuItems {
    device: Vec<MenuItem>,
    play: MenuItem,
    preset: Vec<MenuItem>,
}
struct ConnectrApp {
    menu: MenuItems,
}

fn main() {
    let mut app = ConnectrApp {
        menu: MenuItems {
            device: Vec::<MenuItem>::new(),
            play: ptr::null_mut(),
            preset: Vec::<MenuItem>::new(),
        }
    };
    let (tx,rx) = channel::<String>();
    let mut spotify = connectr::SpotifyConnectr::new();
    spotify.connect();
    spotify.set_target_device(None);
    let mut status = osx::OSXStatusBar::new(tx);

    let device_list = spotify.request_device_list();

    status.add_label("DEVICES:");
    status.add_separator();

    println!("Visible Devices:");
    for dev in device_list {
        println!("{}", dev);
        let id = dev.id.clone();
        let cb: osx::NSCallback = Box::new(move |sender, tx| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::SelectDevice,
                sender: sender,
                data: id.to_owned(),
            };
            let _ = tx.send(json::encode(&cmd).unwrap());
        });
        let item = status.add_item(&dev.name, cb, dev.is_active);
        app.menu.device.push(item);
    }
    println!("");

    let player_state = spotify.request_player_state();
    println!("Playback State:\n{}", player_state);
    let play_str = format!("{: ^50}\n{: ^50}\n{: ^50}",
                           &player_state.item.name,
                           &player_state.item.artists[0].name,
                           &player_state.item.album.name);
    status.set_tooltip(&play_str);

    status.add_label("");
    status.add_label("ACTIONS:");
    status.add_separator();
    {
        let play_str = match player_state.is_playing {
            true => "PAUSE",
            false => "PLAY",
        };
        let cb: osx::NSCallback = Box::new(move |sender, tx| {
            let is_playing = &player_state.is_playing;
            let cmd = MenuCallbackCommand {
                action: CallbackAction::PlayPause,
                sender: sender,
                data: is_playing.to_string(),
            };
            let _ = tx.send(json::encode(&cmd).unwrap());
        });
        app.menu.play = status.add_item(&play_str, cb, false);
    }

    status.add_label("");
    status.add_label("PRESETS:");
    status.add_separator();
    {
        for uri in vec!["spotify:user:mrmekon:playlist:4c8eKK6kKrcdt1HToEX7Jc",
                        "spotify:user:spotify:playlist:37i9dQZEVXcOmDhsenkuCu"] {
            let cb: osx::NSCallback = Box::new(move |sender, tx| {
                let cmd = MenuCallbackCommand {
                    action: CallbackAction::Preset,
                    sender: sender,
                    data: uri.to_owned(),
                };
                let _ = tx.send(json::encode(&cmd).unwrap());
            });
            let item = status.add_item(uri, cb, false);
            app.menu.preset.push(item);
        }
    }

    loop {
        spotify.await_once(false);
        if let Ok(s) = rx.try_recv() {
            println!("Received {}", s);
            let cmd: MenuCallbackCommand = json::decode(&s).unwrap();
            match cmd.action {
                CallbackAction::SelectDevice => {
                    let device = &app.menu.device;
                    for item in device {
                        status.unsel_item(*item as u64);
                    }
                    status.sel_item(cmd.sender);
                    // Spotify is broken.  Must be 'true', always starts playing.
                    require(spotify.transfer(cmd.data, true));
                },
                CallbackAction::PlayPause => {
                    let player_state = spotify.request_player_state();
                    match player_state.is_playing {
                        true => {require(spotify.pause());},
                        false => {require(spotify.play(None));},
                    }
                    //let player_state = spotify.request_player_state();
                    let play_str = match player_state.is_playing {
                        false => "PAUSE",
                        true => "PLAY",
                    };
                    status.update_item(app.menu.play, play_str);
                },
                CallbackAction::Preset => {
                    play_uri(&mut spotify, None, Some(&cmd.data));
                }
            }
        }
        status.run(false);
        sleep(Duration::from_millis(10));
    }
}

fn require(response: SpotifyResponse) {
    match response.code.unwrap() {
        200 ... 299 => (),
        _ => panic!("{}", response)
    }
    println!("Response: {}", response.code.unwrap());
}

fn play_uri(spotify: &mut connectr::SpotifyConnectr, device: Option<&str>, uri: Option<&str>) {
    match device {
        Some(dev) => { spotify.set_target_device(Some(dev.to_string())); },
        None => { spotify.set_target_device(None); },
    }
    match uri {
        Some(s) => {
            let ctx = connectr::PlayContext::new()
                .context_uri(s)
                .offset_position(0)
                .build();
            require(spotify.play(Some(&ctx)));
        }
        None => {
            println!("Transfer!");
            require(spotify.play(None));
        }
    };
}

//    spotify.set_target_device(None);
//    require(spotify.pause());
//    require(spotify.next());
//    require(spotify.previous());
//    require(spotify.seek(5000));
//    require(spotify.volume(10));
//    require(spotify.shuffle(true));
//    require(spotify.repeat(connectr::SpotifyRepeat::Context));
//    require(spotify.transfer_multi(vec!["1a793f2a23989a1c35d05b2fd1ff00e9a67e7134".to_string()], false));
//    require(spotify.transfer("1a793f2a23989a1c35d05b2fd1ff00e9a67e7134".to_string(), false));
