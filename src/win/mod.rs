extern crate systray;

pub use ::TStatusBar;
pub use ::NSCallback;

use self::systray::api::api::MenuEnableFlag;

use std::collections::BTreeMap;
use std::sync::mpsc::Sender;

//pub type Object = objc::runtime::Object;
pub type Object = u32;

use std::cell::Cell;
pub struct WindowsStatusBar {
    app: systray::Application,
    idx: Cell<u32>,
    tx: Sender<String>,
    items: BTreeMap<u64, u32>,
}

impl TStatusBar for WindowsStatusBar {
    type S = WindowsStatusBar;
    fn new(tx: Sender<String>) -> WindowsStatusBar {
        let mut bar = WindowsStatusBar {
            app: systray::Application::new().unwrap(),
            idx: Cell::new(0),
            tx: tx,
            items: BTreeMap::<u64, u32>::new(),
        };
        {
            let ref mut win = &mut bar.app.window;
            win.set_icon_from_file(&"spotify.ico".to_string());
        }
        bar
    }
    fn can_redraw(&mut self) -> bool {
        let ref mut win = &mut self.app.window;
        !win.menu_displayed()
    }
    fn clear_items(&mut self) {
        let ref mut win = &mut self.app.window;
        win.clear_menu();
        self.items.clear();
    }
    fn set_tooltip(&mut self, text: &str) {
        let ref mut win = &mut self.app.window;
        win.set_tooltip(&text.to_string());
    }
    fn add_label(&mut self, label: &str) {
        let ref mut win = &mut self.app.window;
        let idx = win.add_menu_item(&label.to_string(), false, |window| {});
        win.enable_menu_item(idx.unwrap(), MenuEnableFlag::Disabled);
    }
    fn add_quit(&mut self, label: &str) {
        let ref mut win = &mut self.app.window;
        win.add_menu_item(&"Quit".to_string(), false, |window| { window.quit(); panic!("goodness."); });
    }
    fn add_separator(&mut self) {
        let ref mut win = &mut self.app.window;
        win.add_menu_separator();
    }
    fn add_item(&mut self, item: &str, callback: NSCallback, selected: bool) -> *mut Object {
        let ref mut win = &mut self.app.window;
        let idx = self.idx.get();
        self.idx.set(idx+1);
        let tx = self.tx.clone();
        let item = win.add_menu_item(&item.to_string(), selected, move |window| {
            callback(idx as u64, &tx);
        }).unwrap();
        self.items.insert(idx as u64, item);
        idx as *mut Object
    }
    fn update_item(&mut self, item: *mut Object, label: &str) {
    }
    fn sel_item(&mut self, sender: u64) {
        let ref mut win = &mut self.app.window;
        let obj = self.items.get(&sender).unwrap();
        win.select_menu_item(*obj);
    }
    fn unsel_item(&mut self, sender: u64) {
        let ref mut win = &mut self.app.window;
        let obj = self.items.get(&sender).unwrap();
        win.unselect_menu_item(*obj);
    }
    fn run(&mut self, block: bool) {
        let ref mut win = &mut self.app.window;
        win.wait_for_message(block);
    }
}
