extern crate systray;

pub use ::TStatusBar;
pub use ::NSCallback;

use std::sync::mpsc::Sender;

//pub type Object = objc::runtime::Object;
pub type Object = u32;

use std::cell::Cell;
pub struct WindowsStatusBar {
    app: systray::Application,
    idx: Cell<u32>,
}

impl TStatusBar for WindowsStatusBar {
    type S = WindowsStatusBar;
    fn new(tx: Sender<String>) -> WindowsStatusBar {
        let mut bar = WindowsStatusBar {
            app: systray::Application::new().unwrap(),
            idx: Cell::new(0),
        };
        {
            let ref mut win = &mut bar.app.window;
            win.set_icon_from_file(&"spotify.ico".to_string());
            win.add_menu_separator();
            win.add_menu_item(&"Menu Item1".to_string(), true, |window| {println!("hello")});
            win.add_menu_item(&"Menu Item2".to_string(), false, |window| {println!("hello")});
            let idx = win.add_menu_item(&"Menu Item3".to_string(), false, |window| {println!("hello")});
            let idx = idx.unwrap();
            win.select_menu_entry(idx);
            win.unselect_menu_entry(idx);
            win.clear_menu();
            win.add_menu_item(&"Menu Item4".to_string(), false, |window| {println!("hello")});
        }
        bar
    }
    fn clear_items(&mut self) {
    }
    fn set_tooltip(&mut self, text: &str) {
        let ref mut win = &mut self.app.window;
        win.set_tooltip(&text.to_string());
    }
    fn add_label(&mut self, label: &str) {
        let ref mut win = &mut self.app.window;
        win.add_menu_item(&label.to_string(), false, |window| {});
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
        win.add_menu_item(&item.to_string(), selected, move |window| {println!("rand: {}", idx);}).unwrap() as *mut Object
    }
    fn update_item(&mut self, item: *mut Object, label: &str) {
    }
    fn sel_item(&mut self, sender: u64) {
        let ref mut win = &mut self.app.window;
        win.select_menu_entry(sender as u32);
    }
    fn unsel_item(&mut self, sender: u64) {
        let ref mut win = &mut self.app.window;
        win.unselect_menu_entry(sender as u32);
    }
    fn run(&mut self, block: bool) {
        let ref mut win = &mut self.app.window;
        win.wait_for_message();
    }
}
