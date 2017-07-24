extern crate connectr;
use connectr::SpotifyResponse;
use connectr::TStatusBar;
use connectr::MenuItem;
use connectr::NSCallback;

extern crate rubrail;
use rubrail::Touchbar;
use rubrail::TTouchbar;
use rubrail::TScrubberData;
use rubrail::ImageTemplate;
use rubrail::SpacerType;

extern crate ctrlc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::RwLock;

#[macro_use]
extern crate log;
extern crate log4rs;

use std::env;
use std::ptr;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use std::sync::mpsc::channel;
use std::sync::mpsc::{Sender, Receiver};
use std::rc::Rc;
use std::cell::RefCell;

extern crate time;

#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::process;

// How often to refresh Spotify state (if nothing triggers a refresh earlier).
pub const REFRESH_PERIOD: i64 = 30;

#[allow(dead_code)]
enum RefreshTime {
    Now,   // immediately
    Soon,  // after ~1 sec
    Later, // don't change whatever the current schedule is
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
enum CallbackAction {
    SelectDevice,
    PlayPause,
    SkipNext,
    SkipPrev,
    Volume,
    Preset,
}

#[derive(Serialize, Deserialize, Debug)]
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
    // TODO: move touchbar in
}

struct TouchbarScrubberData {
    action: CallbackAction,
    entries: RefCell<Vec<(String,String)>>,
    tx: Sender<String>,
}
impl TouchbarScrubberData {
    fn new(action: CallbackAction,
           tx: Sender<String>) -> Rc<TouchbarScrubberData> {
        Rc::new(TouchbarScrubberData {
            action: action,
            entries: RefCell::new(Vec::<(String,String)>::new()),
            tx: tx,
        })
    }
    fn fill(&self, items: Vec<(String,String)>) {
        let mut entries = self.entries.borrow_mut();
        entries.clear();
        for item in items {
            entries.push(item);
        }
    }
}
impl TScrubberData for TouchbarScrubberData {
    fn count(&self, _item: rubrail::ItemId) -> u32 {
        self.entries.borrow().len() as u32
    }
    fn text(&self, _item: rubrail::ItemId, idx: u32) -> String {
        self.entries.borrow()[idx as usize].0.to_string()
    }
    fn width(&self, _item: rubrail::ItemId, idx: u32) -> u32 {
        // 10px per character + some padding seems to work nicely for the default
        // font.  no idea what it's like on other machines.  does the touchbar
        // font change? ¯\_(ツ)_/¯
        let len = self.entries.borrow()[idx as usize].0.len() as u32;
        let width = len * 8 + 20;
        width
    }
    fn touch(&self, ui_item: rubrail::ItemId, idx: u32) {
        info!("scrub touch: {}", idx);
        if let Some(item) = self.entries.borrow().get(idx as usize) {
            let cmd = MenuCallbackCommand {
                action: self.action,
                sender: ui_item,
                data: item.1.clone(),
            };
            let _ = self.tx.send(serde_json::to_string(&cmd).unwrap());
        }
    }
}

#[allow(dead_code)]
struct TouchbarUI {
    touchbar: Touchbar,

    root_bar: rubrail::BarId,
    playing_label: rubrail::ItemId,
    prev_button: rubrail::ItemId,
    play_pause_button: rubrail::ItemId,
    next_button: rubrail::ItemId,

    preset_bar: rubrail::BarId,
    preset_popover: rubrail::ItemId,
    preset_data: Rc<TouchbarScrubberData>,
    preset_scrubber: rubrail::ItemId,

    device_bar: rubrail::BarId,
    device_popover: rubrail::ItemId,
    device_data: Rc<TouchbarScrubberData>,
    device_scrubber: rubrail::ItemId,

    volume_bar: rubrail::BarId,
    volume_popover: rubrail::ItemId,
    volume_slider: rubrail::ItemId,

    submenu_bar: rubrail::BarId,
    submenu_popover: rubrail::ItemId,
}

impl TouchbarUI {
    fn init(tx: Sender<String>) -> TouchbarUI {
        let mut touchbar = Touchbar::alloc("cnr");
        let icon = rubrail::util::bundled_resource_path("connectr_80px_300dpi", "png");
        if let Some(path) = icon {
            touchbar.set_icon(&path);
        }

        let playing_label = touchbar.create_label(
            "Now Playing Long Track                                  \n\
             Now Playing Long Artist                                 ");
        let image = touchbar.create_image_from_template(ImageTemplate::RewindTemplate);
        let tx_clone = tx.clone();
        let prev_button = touchbar.create_button(Some(&image), None, Box::new(move |s| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::SkipPrev,
                sender: s,
                data: String::new(),
            };
            let _ = tx_clone.send(serde_json::to_string(&cmd).unwrap());
        }));
        let image = touchbar.create_image_from_template(ImageTemplate::PlayPauseTemplate);
        let tx_clone = tx.clone();
        let play_pause_button = touchbar.create_button(Some(&image), None, Box::new(move |s| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::PlayPause,
                sender: s,
                data: String::new(),
            };
            let _ = tx_clone.send(serde_json::to_string(&cmd).unwrap());
        }));
        let image = touchbar.create_image_from_template(ImageTemplate::FastForwardTemplate);
        let tx_clone = tx.clone();
        let next_button = touchbar.create_button(Some(&image), None, Box::new(move |s| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::SkipNext,
                sender: s,
                data: String::new(),
            };
            let _ = tx_clone.send(serde_json::to_string(&cmd).unwrap());
        }));

        let preset_scrubber_data = TouchbarScrubberData::new(CallbackAction::Preset,
                                                             tx.clone());
        let preset_scrubber = touchbar.create_text_scrubber(preset_scrubber_data.clone());
        let preset_bar = touchbar.create_bar();
        touchbar.add_items_to_bar(&preset_bar, vec![preset_scrubber]);
        let preset_popover = touchbar.create_popover_item(
            None,
            Some(&format!("{}", "Presets")),
            &preset_bar);
        touchbar.update_button_width(&preset_popover, 200);

        let device_scrubber_data = TouchbarScrubberData::new(CallbackAction::SelectDevice,
                                                             tx.clone());
        let device_scrubber = touchbar.create_text_scrubber(device_scrubber_data.clone());
        let device_bar = touchbar.create_bar();
        touchbar.add_items_to_bar(&device_bar, vec![device_scrubber]);
        let device_popover = touchbar.create_popover_item(
            None,
            Some(&format!("{}", "Devices")),
            &device_bar);
        touchbar.update_button_width(&device_popover, 200);

        let tx_clone = tx.clone();
        let volume_slider = touchbar.create_slider(0., 100., false, Box::new(move |s,v| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::Volume,
                sender: s,
                data: (v as u32).to_string(),
            };
            let _ = tx_clone.send(serde_json::to_string(&cmd).unwrap());
        }));
        let volume_bar = touchbar.create_bar();
        touchbar.add_items_to_bar(&volume_bar, vec![volume_slider]);
        let image = touchbar.create_image_from_template(ImageTemplate::AudioOutputVolumeMediumTemplate);
        let volume_popover = touchbar.create_popover_item(
            Some(&image),
            None,
            &volume_bar);

        let submenu_bar = touchbar.create_bar();
        touchbar.add_items_to_bar(&submenu_bar, vec![
            preset_popover,
            device_popover,
        ]);
        let image = touchbar.create_image_from_template(ImageTemplate::GoUpTemplate);
        let submenu_popover = touchbar.create_popover_item(
            Some(&image),
            None,
            &submenu_bar);

        let flexible_space = touchbar.create_spacer(SpacerType::Flexible);
        let flexible_space2 = touchbar.create_spacer(SpacerType::Flexible);
        let root_bar = touchbar.create_bar();
        touchbar.add_items_to_bar(&root_bar, vec![
            playing_label,
            prev_button,
            play_pause_button,
            next_button,
            flexible_space,
            volume_popover,
            flexible_space2,
            submenu_popover,
        ]);
        touchbar.set_bar_as_root(root_bar);

        TouchbarUI {
            touchbar: touchbar,

            root_bar: root_bar,
            playing_label: playing_label,
            prev_button: prev_button,
            play_pause_button: play_pause_button,
            next_button: next_button,

            preset_bar: preset_bar,
            preset_popover: preset_popover,
            preset_data: preset_scrubber_data,
            preset_scrubber: preset_scrubber,

            device_bar: device_bar,
            device_popover: device_popover,
            device_data: device_scrubber_data,
            device_scrubber: device_scrubber,

            volume_bar: volume_bar,
            volume_popover: volume_popover,
            volume_slider: volume_slider,

            submenu_bar: submenu_bar,
            submenu_popover: submenu_popover,
        }
    }
    fn update_now_playing(&mut self, track: &str, artist: &str) {
        let text = format!("{}\n{}", track, artist);
        self.touchbar.update_label(&self.playing_label, &text);
        self.touchbar.update_label_width(&self.playing_label, 200)
    }
    fn update_volume(&mut self, volume: u32) {
        self.touchbar.update_slider(&self.volume_slider, volume as f64);
    }
    fn update_scrubbers(&mut self) {
        self.touchbar.refresh_scrubber(&self.device_scrubber);
        self.touchbar.refresh_scrubber(&self.preset_scrubber);
    }
    fn set_selected_device(&mut self, selected: u32) {
        self.touchbar.select_scrubber_item(&self.device_scrubber, selected);
    }
}

fn play_action_label(is_playing: bool) -> &'static str {
    match is_playing {
        true => "Pause",
        false => "Play",
    }
}

fn fill_menu<T: TStatusBar>(app: &mut ConnectrApp,
                            spotify: &SpotifyThread,
                            status: &mut T,
                            touchbar: &mut TouchbarUI) {
    let device_list = spotify.device_list.read().unwrap();
    let player_state = spotify.player_state.read().unwrap();
    let presets = spotify.presets.read().unwrap();
    if device_list.is_none() ||
        player_state.is_none() {
            // TODO: handle empty groups
            return;
        }
    let device_list = device_list.as_ref().unwrap();
    let player_state = player_state.as_ref().unwrap();

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
    touchbar.update_now_playing(&player_state.item.name,
                                &player_state.item.artists[0].name);

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
            let _ = tx.send(serde_json::to_string(&cmd).unwrap());
        });
        app.menu.play = status.add_item(&play_str, cb, false);

        let cb: NSCallback = Box::new(move |sender, tx| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::SkipNext,
                sender: sender,
                data: String::new(),
            };
            let _ = tx.send(serde_json::to_string(&cmd).unwrap());
        });
        app.menu.next = status.add_item("Next", cb, false);

        let cb: NSCallback = Box::new(move |sender, tx| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::SkipPrev,
                sender: sender,
                data: String::new(),
            };
            let _ = tx.send(serde_json::to_string(&cmd).unwrap());
        });
        app.menu.prev = status.add_item("Previous", cb, false);
    }

    status.add_label("");
    status.add_label("Presets:");
    status.add_separator();
    {
        let preset_tuples: Vec<(String,String)> = presets.iter().map(|p| {
            (p.0.clone(), p.1.clone())
        }).collect();
        touchbar.preset_data.fill(preset_tuples);
        for preset in presets.iter() {
            let ref name = preset.0;
            let uri = preset.1.clone();
            let cb: NSCallback = Box::new(move |sender, tx| {
                let cmd = MenuCallbackCommand {
                    action: CallbackAction::Preset,
                    sender: sender,
                    data: uri.to_owned(),
                };
                let _ = tx.send(serde_json::to_string(&cmd).unwrap());
            });
            let item = status.add_item(&name.clone(), cb, false);
            app.menu.preset.push(item);
        }
    }

    status.add_label("");
    status.add_label("Devices:");
    status.add_separator();
    println!("Visible Devices:");

    let devices: Vec<(String,String)> = device_list.into_iter().map(|d| {
        (d.name.clone(), d.id.clone().unwrap_or(String::new()))
    }).collect();
    touchbar.device_data.fill(devices);

    let selected_arr: Vec<bool> = device_list.into_iter().map(|d| {d.is_active}).collect();
    if let Ok(selected) = selected_arr.binary_search(&true) {
        touchbar.set_selected_device(selected as u32);
    }
    touchbar.update_scrubbers();

    let mut cur_volume: u32 = 0;
    let mut cur_volume_exact: u32 = 0;
    for dev in device_list {
        println!("{}", dev);
        let id = match dev.id {
            Some(ref id) => id.clone(),
            None => "".to_string(),
        };
        let cb_id = id.clone();
        let cb: NSCallback = Box::new(move |sender, tx| {
            let cmd = MenuCallbackCommand {
                action: CallbackAction::SelectDevice,
                sender: sender,
                data: cb_id.to_owned(),
            };
            let _ = tx.send(serde_json::to_string(&cmd).unwrap());
        });
        let item = status.add_item(&dev.name, cb, dev.is_active);
        if dev.is_active {
            cur_volume_exact = dev.volume_percent.unwrap_or(0);
            cur_volume = match dev.volume_percent {
                Some(v) => {
                    (v as f32 / 10.0).round() as u32 * 10
                },
                None => 100,
            };
        }
        app.menu.device.push((item, id));
        touchbar.update_volume(cur_volume_exact);
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
                let _ = tx.send(serde_json::to_string(&cmd).unwrap());
            });
            let item = status.add_item(&vol_str, cb, i == cur_volume);
            app.menu.volume.push(item);
            i += 10;
        }
    }
    status.add_separator();
    status.add_quit("Exit");
}

fn clear_menu<T: TStatusBar>(app: &mut ConnectrApp, status: &mut T) {
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

fn handle_callback(player_state: Option<&connectr::PlayerState>,
                   spotify: &mut connectr::SpotifyConnectr,
                   cmd: &MenuCallbackCommand) -> RefreshTime {
    info!("Executed action: {:?}", cmd.action);
    let refresh = RefreshTime::Soon;
    match cmd.action {
        CallbackAction::SelectDevice => {
            require(spotify.transfer(cmd.data.clone(), true));
        },
        CallbackAction::PlayPause => {
            if let Some(player_state) = player_state {
                match player_state.is_playing {
                    true => {require(spotify.pause());},
                    false => {require(spotify.play(None));},
                }
            }
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
        }
    }
    refresh
}

fn refresh_time(player_state: Option<&connectr::PlayerState>, now: i64) -> i64 {
    let refresh_offset = match player_state {
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

struct SpotifyThread {
    #[allow(dead_code)]
    handle: std::thread::JoinHandle<()>,
    #[allow(dead_code)]
    tx: Sender<String>,
    rx: Receiver<String>,
    device_list: Arc<RwLock<Option<connectr::ConnectDeviceList>>>,
    player_state: Arc<RwLock<Option<connectr::PlayerState>>>,
    presets: Arc<RwLock<Vec<(String,String)>>>,
}

fn create_spotify_thread(rx_cmd: Receiver<String>) -> SpotifyThread {
    let (tx_in,rx_in) = channel::<String>();
    let (tx_out,rx_out) = channel::<String>();
    let device_list = Arc::new(RwLock::new(None));
    let player_state = Arc::new(RwLock::new(None));
    let presets = Arc::new(RwLock::new(vec![]));
    let thread_device_list = device_list.clone();
    let thread_player_state = player_state.clone();
    let thread_presets = presets.clone();
    let thread = thread::spawn(move || {
        let tx = tx_out;
        let rx = rx_in;
        let rx_cmd = rx_cmd;
        let mut refresh_time_utc = 0;
        let mut spotify = connectr::SpotifyConnectr::new();
        let device_list = thread_device_list;
        let player_state = thread_player_state;
        let presets = thread_presets;
        info!("Created Spotify controller.");
        spotify.connect();
        info!("Created Spotify connection.");
        spotify.set_target_device(None);
        {
            let mut preset_writer = presets.write().unwrap();
            *preset_writer = spotify.get_presets().clone();
            let _ = tx.send(String::new());
        }
        loop {
            if rx.try_recv().is_ok() {
                // Main thread tells us to shutdown
                break;
            }
            let now = time::now_utc().to_timespec().sec as i64;
            spotify.await_once(false);
            // Block for 200ms while waiting for UI input.  This throttles the
            // thread CPU usage, at the expense of slight delays for metadata
            // updates.  Optimizes for UI response.
            if let Ok(s) = rx_cmd.recv_timeout(Duration::from_millis(200)) {
                info!("Received {}", s);
                let cmd: MenuCallbackCommand = serde_json::from_str(&s).unwrap();
                let refresh_strategy =  handle_callback(player_state.read().unwrap().as_ref(),
                                                        &mut spotify, &cmd);
                refresh_time_utc = match refresh_strategy {
                    RefreshTime::Now => now - 1,
                    RefreshTime::Soon => now + 1,
                    RefreshTime::Later => refresh_time_utc,
                }
            }

            if now > refresh_time_utc {
                info!("Request update");
                let dev_list = spotify.request_device_list();
                {
                    let mut dev_writer = device_list.write().unwrap();
                    *dev_writer = dev_list;
                }
                let play_state = spotify.request_player_state();
                {
                    let mut player_writer = player_state.write().unwrap();
                    *player_writer = play_state;
                }
                refresh_time_utc = refresh_time(player_state.read().unwrap().as_ref(), now);
                info!("Refreshed Spotify state.");
                let _ = tx.send(String::new()); // inform main thread
            }
        }
    });
    SpotifyThread {
        handle: thread,
        tx: tx_in,
        rx: rx_out,
        device_list: device_list,
        player_state: player_state,
        presets: presets,
    }
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
    };
    let (tx,rx) = channel::<String>();
    let spotify_thread = create_spotify_thread(rx);

    let mut status = connectr::StatusBar::new(tx.clone());
    info!("Created status bar.");
    let mut touchbar = TouchbarUI::init(tx);
    info!("Created touchbar.");

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
        if spotify_thread.rx.recv_timeout(Duration::from_millis(100)).is_ok() {
            // TODO: && status.can_redraw()
            clear_menu(&mut app, &mut status);
            fill_menu(&mut app, &spotify_thread, &mut status, &mut touchbar);
        }
        status.run(false);
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
        // TODO: Don't panic
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
