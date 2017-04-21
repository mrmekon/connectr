extern crate connectr;
use connectr::SpotifyResponse;

use std::ptr;
use std::thread::sleep;
use std::time::Duration;
use std::sync::mpsc::channel;

extern crate time;

extern crate rustc_serialize;
use rustc_serialize::json;

#[derive(RustcDecodable, RustcEncodable, Debug)]
enum CallbackAction {
    SelectDevice,
    PlayPause,
    SkipNext,
    SkipPrev,
    Volume,
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
    device: Vec<(MenuItem, String)>,
    play: MenuItem,
    next: MenuItem,
    prev: MenuItem,
    preset: Vec<MenuItem>,
    volume: Vec<MenuItem>,
}
struct ConnectrApp {
    menu: MenuItems,
}

fn play_action_label(is_playing: bool) -> &'static str {
    match is_playing {
        true => "Pause",
        false => "Play",
    }
}

fn fill_menu<T: TStatusBar>(app: &mut ConnectrApp, spotify: &mut connectr::SpotifyConnectr, status: &mut T) {
    let device_list = spotify.request_device_list();
    let player_state = spotify.request_player_state();

    println!("Playback State:\n{}", player_state);
    let play_str = format!("{: ^50}\n{: ^50}\n{: ^50}",
                           &player_state.item.name,
                           &player_state.item.artists[0].name,
                           &player_state.item.album.name);
    status.set_tooltip(&play_str);

    status.add_label("Now Playing:");
    status.add_separator();
    //status.add_label(&player_state.item.name);
    status.add_label(&format!("{:<50}", &player_state.item.name));
    status.add_label(&format!("{:<50}", &player_state.item.artists[0].name));
    status.add_label(&format!("{:<50}", &player_state.item.album.name));
    let ms = player_state.item.duration_ms;
    let min = ms / 1000 / 60;
    let sec = (ms - (min * 60 * 1000)) / 1000;
    status.add_label(&format!("{:<50}", format!("{}:{:02}", min, sec)));

    status.add_label("");
    status.add_label("Actions:");
    status.add_separator();
    {
        let play_str = play_action_label(player_state.is_playing);
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

        let cb: osx::NSCallback = Box::new(move |sender, tx| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::SkipNext,
                sender: sender,
                data: String::new(),
            };
            let _ = tx.send(json::encode(&cmd).unwrap());
        });
        app.menu.next = status.add_item("Next", cb, false);

        let cb: osx::NSCallback = Box::new(move |sender, tx| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::SkipPrev,
                sender: sender,
                data: String::new(),
            };
            let _ = tx.send(json::encode(&cmd).unwrap());
        });
        app.menu.prev = status.add_item("Previous", cb, false);
    }

    status.add_label("");
    status.add_label("Presets:");
    status.add_separator();
    {
        let presets = spotify.get_presets();
        for preset in presets {
            let ref name = preset.0;
            let uri = preset.1.clone();
            let cb: osx::NSCallback = Box::new(move |sender, tx| {
                let cmd = MenuCallbackCommand {
                    action: CallbackAction::Preset,
                    sender: sender,
                    data: uri.to_owned(),
                };
                let _ = tx.send(json::encode(&cmd).unwrap());
            });
            let item = status.add_item(&name.clone(), cb, false);
            app.menu.preset.push(item);
        }
    }

    status.add_label("");
    status.add_label("Devices:");
    status.add_separator();
    println!("Visible Devices:");
    let mut cur_volume: u32 = 0;
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
        if dev.is_active {
            cur_volume = match dev.volume_percent {
                Some(v) => {
                    (v as f32 / 10.0).round() as u32 * 10
                },
                None => 100,
            }
        }
        app.menu.device.push((item, dev.id.clone()));
    }
    println!("");

    status.add_label("");
    status.add_label("Volume:");
    status.add_separator();
    {
        let mut i = 0;
        while i <= 100 {
            let vol_str = format!("{}%", i);
            let cb: osx::NSCallback = Box::new(move |sender, tx| {
                let cmd = MenuCallbackCommand {
                    action: CallbackAction::Volume,
                    sender: sender,
                    data: i.to_string(),
                };
                let _ = tx.send(json::encode(&cmd).unwrap());
            });
            let item = status.add_item(&vol_str, cb, i == cur_volume);
            app.menu.volume.push(item);
            i += 10;
        }
    }
    status.add_separator();
    status.add_quit("Exit");
}

fn clear_menu<T: TStatusBar>(app: &mut ConnectrApp, _: &mut connectr::SpotifyConnectr, status: &mut T) {
    app.menu = MenuItems {
        device: Vec::<(MenuItem, String)>::new(),
        play: ptr::null_mut(),
        next: ptr::null_mut(),
        prev: ptr::null_mut(),
        preset: Vec::<MenuItem>::new(),
        volume: Vec::<MenuItem>::new(),
    };
    status.clear_items();
}

fn main() {
    let mut app = ConnectrApp {
        menu: MenuItems {
            device: Vec::<(MenuItem, String)>::new(),
            play: ptr::null_mut(),
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            preset: Vec::<MenuItem>::new(),
            volume: Vec::<MenuItem>::new(),
        }
    };
    let mut refresh_time_utc = 0;
    let (tx,rx) = channel::<String>();
    let mut spotify = connectr::SpotifyConnectr::new();
    spotify.connect();
    spotify.set_target_device(None);
    let mut status = osx::OSXStatusBar::new(tx);

    loop {
        let now = time::now_utc().to_timespec().sec as i64;
        if now > refresh_time_utc {
            // Redraw the whole menu once every 60 seconds, or sooner if a
            // command is processed later.
            clear_menu(&mut app, &mut spotify, &mut status);
            fill_menu(&mut app, &mut spotify, &mut status);
            refresh_time_utc = now + 30;
        }

        spotify.await_once(false);
        if let Ok(s) = rx.try_recv() {
            println!("Received {}", s);
            let cmd: MenuCallbackCommand = json::decode(&s).unwrap();
            match cmd.action {
                CallbackAction::SelectDevice => {
                    let device = &app.menu.device;
                    for dev in device {
                        let &(ref item, _) = dev;
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
                    let play_str = play_action_label(!player_state.is_playing);
                    status.update_item(app.menu.play, play_str);
                },
                CallbackAction::Preset => {
                    play_uri(&mut spotify, None, Some(&cmd.data));
                }
                CallbackAction::SkipNext => {
                    require(spotify.next());
                }
                CallbackAction::SkipPrev => {
                    require(spotify.previous());
                }
                CallbackAction::Volume => {
                    let vol = cmd.data.parse::<u32>().unwrap();
                    require(spotify.volume(vol));
                    let volume = &app.menu.volume;
                    for item in volume {
                        status.unsel_item(*item as u64);
                    }
                    status.sel_item(cmd.sender);
                }
            }
            refresh_time_utc = now + 1;
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
