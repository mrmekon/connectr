pub mod rustnsobject;

extern crate objc;
extern crate objc_foundation;
extern crate cocoa;
extern crate libc;

pub use ::TStatusBar;
pub use ::NSCallback;

use objc::runtime::Class;

use self::cocoa::base::{nil, YES};
use self::cocoa::appkit::NSStatusBar;
use self::cocoa::foundation::{NSAutoreleasePool,NSString};
use self::cocoa::appkit::{NSApp,
                          NSApplication,
                          NSApplicationActivationPolicyAccessory,
                          NSMenu,
                          NSMenuItem,
                          NSImage,
                          NSVariableStatusItemLength,
                          NSStatusItem,
                          NSButton};

use self::rustnsobject::{NSObj, NSObjTrait, NSObjCallbackTrait};

use std::sync::mpsc::Sender;

use std::ptr;
use std::cell::Cell;
use std::ffi::CStr;
use std::thread::sleep;
use std::time::Duration;

extern crate objc_id;
use self::objc_id::Id;

pub type Object = objc::runtime::Object;

pub struct OSXStatusBar {
    object: NSObj,
    app: *mut objc::runtime::Object,
    status_bar_item: *mut objc::runtime::Object,
    menu_bar: *mut objc::runtime::Object,

    // Run loop state
    // Keeping these in persistent state instead of recalculating saves quite a
    // bit of CPU during idle.
    pool: Cell<*mut objc::runtime::Object>,
    run_count: Cell<u64>,
    run_mode: *mut objc::runtime::Object,
    run_date: *mut objc::runtime::Object,
}

impl TStatusBar for OSXStatusBar {
    type S = OSXStatusBar;
    fn new(tx: Sender<String>) -> OSXStatusBar {
        let mut bar;
        unsafe {
            let app = NSApp();
            let status_bar = NSStatusBar::systemStatusBar(nil);
            let date_cls = Class::get("NSDate").unwrap();
            bar = OSXStatusBar {
                app: app,
                status_bar_item: status_bar.statusItemWithLength_(NSVariableStatusItemLength),
                menu_bar: NSMenu::new(nil),
                object: NSObj::alloc(tx).setup(),
                pool: Cell::new(nil),
                run_count: Cell::new(0),
                run_mode: NSString::alloc(nil).init_str("kCFRunLoopDefaultMode"),
                run_date: msg_send![date_cls, distantPast],
            };
            bar.app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
            msg_send![bar.status_bar_item, setHighlightMode:YES];
            let img_path = match bundled_resource_path("spotify", "png") {
                Some(path) => path,
                None => "spotify.png".to_string(),
            };
            let img = NSString::alloc(nil).init_str(&img_path);
            let icon = NSImage::alloc(nil).initWithContentsOfFile_(img);
            let _ = msg_send![img, release];
            //let icon = NSImage::alloc(nil).initWithContentsOfFile_(img);
            //let icon = NSImage::imageNamed_(img, img);
            let title = NSString::alloc(nil).init_str("connectr");
            NSButton::setTitle_(bar.status_bar_item, title);
            let _ = msg_send![title, release];
            bar.status_bar_item.button().setImage_(icon);
            let _ = msg_send![icon, release];
            bar.status_bar_item.setMenu_(bar.menu_bar);
            bar.object.cb_fn = Some(Box::new(
                move |s, sender| {
                    let cb = s.get_value(sender);
                    cb(sender, &s.tx);
                }
            ));
            let _: () = msg_send![app, finishLaunching];
        }
        bar
    }
    fn can_redraw(&mut self) -> bool {
        true
    }
    fn clear_items(&mut self) {
        unsafe {
            let old_menu = self.menu_bar;
            self.menu_bar = NSMenu::new(nil);
            self.status_bar_item.setMenu_(self.menu_bar);
            let _ = msg_send![old_menu, removeAllItems];
            let _ = msg_send![old_menu, release];
        }
    }
    fn set_tooltip(&mut self, text: &str) {
        unsafe {
            let img = NSString::alloc(nil).init_str(text);
            let _ = msg_send![self.status_bar_item.button(), setToolTip: img];
            let _ = msg_send![img, release];
        }
    }
    fn add_label(&mut self, label: &str) {
        unsafe {
            let txt = NSString::alloc(nil).init_str(label);
            let quit_key = NSString::alloc(nil).init_str("");
            let app_menu_item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(txt, self.object.selector(), quit_key);
            let _ = msg_send![txt, release];
            let _ = msg_send![quit_key, release];
            self.menu_bar.addItem_(app_menu_item);
            let _ = msg_send![app_menu_item, release];
        }
    }
    fn add_quit(&mut self, label: &str) {
        unsafe {
            let txt = NSString::alloc(nil).init_str(label);
            let quit_key = NSString::alloc(nil).init_str("");
            let app_menu_item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(txt, sel!(terminate:), quit_key);
            let _ = msg_send![txt, release];
            let _ = msg_send![quit_key, release];
            self.menu_bar.addItem_(app_menu_item);
            let _ = msg_send![app_menu_item, release];
        }
    }
    fn add_separator(&mut self) {
        unsafe {
            let cls = Class::get("NSMenuItem").unwrap();
            let sep: *mut Object = msg_send![cls, separatorItem];
            self.menu_bar.addItem_(sep);
        }
    }
    fn add_item(&mut self, item: &str, callback: NSCallback, selected: bool) -> *mut Object {
        unsafe {
            let txt = NSString::alloc(nil).init_str(item);
            let quit_key = NSString::alloc(nil).init_str("");
            let app_menu_item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(txt, self.object.selector(), quit_key);
            let _ = msg_send![txt, release];
            let _ = msg_send![quit_key, release];
            self.object.add_callback(app_menu_item, callback);
            let objc = self.object.take_objc();
            let _: () = msg_send![app_menu_item, setTarget: objc];
            if selected {
                let _: () = msg_send![app_menu_item, setState: 1];
            }
            let item: *mut Object = app_menu_item;
            self.menu_bar.addItem_(app_menu_item);
            let _ = msg_send![app_menu_item, release];
            item
        }
    }
    fn update_item(&mut self, item: *mut Object, label: &str) {
        unsafe {
            let ns_label = NSString::alloc(nil).init_str(label);
            let _: () = msg_send![item, setTitle: ns_label];
            let _ = msg_send![ns_label, release];
        }
    }
    fn sel_item(&mut self, sender: u64) {
        let target: *mut Object = sender as *mut Object;
        unsafe {
            let _: () = msg_send![target, setState: 1];
        }
    }
    fn unsel_item(&mut self, sender: u64) {
        let target: *mut Object = sender as *mut Object;
        unsafe {
            let _: () = msg_send![target, setState: 0];
        }
    }
    fn run(&mut self, block: bool) {
        loop {
            unsafe {
                let run_count = self.run_count.get();
                // Create a new release pool every once in a while, draining the old one
                if run_count % 100 == 0 {
                    let old_pool = self.pool.get();
                    if run_count != 0 {
                        let _ = msg_send![old_pool, drain];
                    }
                    self.pool.set(NSAutoreleasePool::new(nil));
                }
                let mode = self.run_mode;
                let event: Id<Object> = msg_send![self.app, nextEventMatchingMask: -1
                                                  untilDate: self.run_date inMode:mode dequeue: YES];
                let _ = msg_send![self.app, sendEvent: event];
                let _ = msg_send![self.app, updateWindows];
                self.run_count.set(run_count + 1);
            }
            if !block { break; }
            sleep(Duration::from_millis(50));
        }
    }
}

//pub fn osx_alert(text: &str) {
//    unsafe {
//        let ns_text = NSString::alloc(nil).init_str(text);
//        let button = NSString::alloc(nil).init_str("ok");
//        let cls = Class::get("NSAlert").unwrap();
//        let alert: *mut Object = msg_send![cls, alloc];
//        let _ = msg_send![alert, init];
//        let _ = msg_send![alert, setMessageText: ns_text];
//        let _ = msg_send![alert, addButtonWithTitle: button];
//        let _ = msg_send![alert, runModal];
//        let _ = msg_send![ns_text, release];
//        let _ = msg_send![button, release];
//        let _ = msg_send![alert, release];
//    }
//}

pub fn resource_dir() -> Option<String> {
    unsafe {
        let cls = Class::get("NSBundle").unwrap();
        let bundle: *mut Object = msg_send![cls, mainBundle];
        let path: *mut Object = msg_send![bundle, resourcePath];
        let cstr: *const libc::c_char = msg_send![path, UTF8String];
        if cstr != ptr::null() {
            let rstr = CStr::from_ptr(cstr).to_string_lossy().into_owned();
            return Some(rstr);
        }
        None
    }
}

pub fn bundled_resource_path(name: &str, extension: &str) -> Option<String> {
    unsafe {
        let cls = Class::get("NSBundle").unwrap();
        let bundle: *mut Object = msg_send![cls, mainBundle];
        let res = NSString::alloc(nil).init_str(name);
        let ext = NSString::alloc(nil).init_str(extension);
        let ini: *mut Object = msg_send![bundle, pathForResource:res ofType:ext];
        let _ = msg_send![res, release];
        let _ = msg_send![ext, release];
        let cstr: *const libc::c_char = msg_send![ini, UTF8String];
        if cstr != ptr::null() {
            let rstr = CStr::from_ptr(cstr).to_string_lossy().into_owned();
            return Some(rstr);
        }
        None
    }
}
