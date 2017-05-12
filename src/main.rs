extern crate connectr;
use connectr::SpotifyResponse;
use connectr::TStatusBar;
use connectr::MenuItem;
use connectr::NSCallback;

extern crate ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[macro_use]
extern crate log;
extern crate log4rs;

use std::env;
use std::ptr;
use std::thread::sleep;
use std::time::Duration;
use std::sync::mpsc::channel;

extern crate time;

extern crate rustc_serialize;
use rustc_serialize::json;

use std::process;

// How often to refresh Spotify state (if nothing triggers a refresh earlier).
pub const REFRESH_PERIOD: i64 = 30;

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
    device_list: Option<connectr::ConnectDeviceList>,
    player_state: Option<connectr::PlayerState>,
}

fn play_action_label(is_playing: bool) -> &'static str {
    match is_playing {
        true => "Pause",
        false => "Play",
    }
}

fn update_state(app: &mut ConnectrApp, spotify: &mut connectr::SpotifyConnectr) -> bool {
    let dev_list = spotify.request_device_list();
    let player_state = spotify.request_player_state();
    match dev_list {
        Some(_) => { app.device_list = dev_list },
        None => { return false },
    }
    match player_state {
        Some(_) => { app.player_state = player_state },
        None => { return false },
    }
    true
}

fn fill_menu<T: TStatusBar>(app: &mut ConnectrApp, spotify: &mut connectr::SpotifyConnectr, status: &mut T) {
    let ref device_list = app.device_list.as_ref().unwrap();
    let ref player_state = app.player_state.as_ref().unwrap();

    println!("Playback State:\n{}", player_state);
    let play_str = format!("{}\n{}\n{}",
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
        let is_playing = player_state.is_playing.clone();
        let cb: NSCallback = Box::new(move |sender, tx| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::PlayPause,
                sender: sender,
                data: is_playing.to_string(),
            };
            let _ = tx.send(json::encode(&cmd).unwrap());
        });
        app.menu.play = status.add_item(&play_str, cb, false);

        let cb: NSCallback = Box::new(move |sender, tx| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::SkipNext,
                sender: sender,
                data: String::new(),
            };
            let _ = tx.send(json::encode(&cmd).unwrap());
        });
        app.menu.next = status.add_item("Next", cb, false);

        let cb: NSCallback = Box::new(move |sender, tx| {
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
            let cb: NSCallback = Box::new(move |sender, tx| {
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
    for dev in *device_list {
        println!("{}", dev);
        let id = dev.id.clone();
        let cb: NSCallback = Box::new(move |sender, tx| {
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
            let cb: NSCallback = Box::new(move |sender, tx| {
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

fn create_logger() {
    use log::LogLevelFilter;
    use log4rs::append::console::ConsoleAppender;
    use log4rs::append::file::FileAppender;
    use log4rs::encode::pattern::PatternEncoder;
    use log4rs::config::{Appender, Config, Logger, Root};

    let log_path = format!("{}/{}", env::home_dir().unwrap().display(), ".connectr.log");
    let stdout = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{m}{n}")))
        .build();
    let requests = FileAppender::builder()
        .build(&log_path)
        .unwrap();

    let config = Config::builder()
        .appender(Appender::builder().build("stdout", Box::new(stdout)))
        .appender(Appender::builder().build("requests", Box::new(requests)))
        .logger(Logger::builder().build("app::backend::db", LogLevelFilter::Info))
        .logger(Logger::builder()
            .appender("requests")
            .additive(false)
            .build("app::requests", LogLevelFilter::Info))
        .build(Root::builder().appender("stdout").appender("requests").build(LogLevelFilter::Info))
        .unwrap();
    let _ = log4rs::init_config(config).unwrap();
}

fn handle_callback<T: TStatusBar>(app: &mut ConnectrApp, spotify: &mut connectr::SpotifyConnectr,
                   status: &mut T, cmd: &MenuCallbackCommand) {
    info!("Executed action: {:?}", cmd.action);
    match cmd.action {
        CallbackAction::SelectDevice => {
            let device = &app.menu.device;
            for dev in device {
                let &(ref item, _) = dev;
                status.unsel_item(*item as u64);
            }
            status.sel_item(cmd.sender);
            // Spotify is broken.  Must be 'true', always starts playing.
            require(spotify.transfer(cmd.data.clone(), true));
        },
        CallbackAction::PlayPause => {
            let fresh_player_state = spotify.request_player_state();
            let player_state = match fresh_player_state {
                Some(ref state) => state,
                None => app.player_state.as_ref().unwrap(),
            };
            match player_state.is_playing {
                true => {require(spotify.pause());},
                false => {require(spotify.play(None));},
            }
            let play_str = play_action_label(!player_state.is_playing);
            status.update_item(app.menu.play, play_str);
        },
        CallbackAction::Preset => {
            play_uri(spotify, None, Some(&cmd.data));
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
}

fn refresh_time(app: &mut ConnectrApp, now: i64) -> i64 {
    let refresh_offset = match app.player_state.as_ref() {
        Some(ref state) => {
            match state.is_playing {
                true => {
                    let track_end = match state.progress_ms {
                        Some(prog) => {
                            if prog < state.item.duration_ms {
                                state.item.duration_ms - prog
                            }
                            else {
                                0
                            }
                        },
                        None => state.item.duration_ms,
                    } as i64;
                    // Refresh 1 second after track ends
                    track_end/1000 + 1
                },
                false => REFRESH_PERIOD,
            }
        }
        None => REFRESH_PERIOD,
    };
    let refresh_offset = std::cmp::min(REFRESH_PERIOD, refresh_offset) as i64;
    info!("State refresh in {} seconds.", refresh_offset);
    now + refresh_offset
}

fn find_wine_path() -> Option<std::path::PathBuf> {
    let search_paths = connectr::search_paths();
    info!("Search paths: {:?}", search_paths);
    for search_path in search_paths {
        let path = std::path::PathBuf::from(search_path).join("wine");
        if path.exists() && path.is_dir() {
            return Some(path);
        }
    }
    None
}

fn main() {
    create_logger();
    info!("Started Connectr");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    match ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }) {
        Ok(_) => {},
        Err(_) => { error!("Failed to register Ctrl-C handler."); }
    }

    let mut app = ConnectrApp {
        menu: MenuItems {
            device: Vec::<(MenuItem, String)>::new(),
            play: ptr::null_mut(),
            next: ptr::null_mut(),
            prev: ptr::null_mut(),
            preset: Vec::<MenuItem>::new(),
            volume: Vec::<MenuItem>::new(),
        },
        device_list: None,
        player_state: None,
    };
    let mut refresh_time_utc = 0;
    let (tx,rx) = channel::<String>();
    let mut spotify = connectr::SpotifyConnectr::new();
    info!("Created Spotify controller.");
    spotify.connect();
    info!("Created Spotify connection.");
    spotify.set_target_device(None);
    let mut status = connectr::StatusBar::new(tx);
    info!("Created status bar.");

    let mut tiny: Option<process::Child> = None;
    if let Some(wine_dir) = find_wine_path() {
        info!("Found wine root: {:?}", wine_dir);
        let wine_exe = wine_dir.join("wine");
        let tiny_exe = wine_dir.join("tiny.exe");
        let config_dir = wine_dir.join("config");
        debug!("{:?} / {:?} / {:?} / {:?}", wine_dir, wine_exe, config_dir, tiny_exe);
        tiny = Some(process::Command::new(wine_exe)
                    .env("WINEPREFIX", config_dir)
                    .current_dir(wine_dir)
                    .args(&[tiny_exe])
                    .spawn().unwrap());
    }
    else {
        warn!("Didn't find Wine in search path.");
    }

    while running.load(Ordering::SeqCst) {
        let now = time::now_utc().to_timespec().sec as i64;
        if now > refresh_time_utc && status.can_redraw() && update_state(&mut app, &mut spotify) {
            // Redraw the whole menu once every 60 seconds, or sooner if a
            // command is processed later.
            clear_menu(&mut app, &mut spotify, &mut status);
            fill_menu(&mut app, &mut spotify, &mut status);
            refresh_time_utc = refresh_time(&mut app, now);
            info!("Refreshed Spotify state.");
        }

        spotify.await_once(false);
        if let Ok(s) = rx.try_recv() {
            println!("Received {}", s);
            let cmd: MenuCallbackCommand = json::decode(&s).unwrap();
            handle_callback(&mut app, &mut spotify, &mut status, &cmd);
            refresh_time_utc = now + 1;
        }
        status.run(false);
        sleep(Duration::from_millis(100));
    }
    info!("Exiting.\n");
    if let Some(mut tiny_proc) = tiny {
        let _ = tiny_proc.kill();
        let _ = tiny_proc.wait();
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
