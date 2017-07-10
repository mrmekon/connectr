mod rustnsobject;

#[cfg(feature = "mac_touchbar")]
mod touchbar;
#[cfg(feature = "mac_touchbar")]
use self::touchbar::{Touchbar, TouchbarTrait, TScrubberData};
#[cfg(not(feature = "mac_touchbar"))]
struct Touchbar {}
#[cfg(not(feature = "mac_touchbar"))]
impl Touchbar {
    fn alloc() -> Touchbar { Touchbar {} }
    fn set_icon(&self, _: *mut Object) {}
    fn enable(&self) {}
}

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
use std::sync::mpsc::channel;

use std::ptr;
use std::rc::Rc;
use std::cell::Cell;
use std::cell::RefCell;
use std::borrow::BorrowMut;
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
    touchbar: Touchbar,

    // Run loop state
    // Keeping these in persistent state instead of recalculating saves quite a
    // bit of CPU during idle.
    pool: Cell<*mut objc::runtime::Object>,
    run_count: Cell<u64>,
    run_mode: *mut objc::runtime::Object,
    run_date: *mut objc::runtime::Object,

    label: Cell<u64>,
    devices: Vec<String>,
    scrubber: Rc<Scrubber>,
}

const ITEMS: &'static [&'static str] = &["a rather longer first one", "one","two","much longer than two","three", "seventeen", "A speaker with a very long name is not an impossible thing."];

struct Scrubber {
    devices: RefCell<Vec<String>>,
    touch_tx: Sender<(touchbar::ItemId,u32)>,
}

impl TScrubberData for Scrubber {
    fn count(&self, item: touchbar::ItemId) -> u32 {
        let dev = self.devices.borrow();
        let len = (*dev).len();
        info!("MOD GOT SCRUBBER COUNT REQUEST {}", len);
        self.devices.borrow().len() as u32
    }
    fn text(&self, item: touchbar::ItemId, idx: u32) -> String {
        info!("MOD GOT SCRUBBER TEXT REQUEST {}", idx);
        self.devices.borrow()[idx as usize].to_string()
    }
    fn width(&self, item: touchbar::ItemId, idx: u32) -> u32 {
        info!("scrub_width {}", idx);
        // 10px per character + some padding seems to work nicely for the default
        // font.  no idea what it's like on other machines.  does the touchbar
        // font change? ¯\_(ツ)_/¯
        let len = self.devices.borrow()[idx as usize].len() as u32;
        let width = len * 8 + 20;
        info!("Width for {}: {}", len, width);
        width
    }
    fn touch(&self, item: touchbar::ItemId, idx: u32) {
        info!("scrub touch: {}", idx);
        self.touch_tx.send((item, idx));
    }
}

impl TStatusBar for OSXStatusBar {
    type S = OSXStatusBar;
    fn new(tx: Sender<String>) -> OSXStatusBar {
        let mut bar;
        unsafe {
            let app = NSApp();
            let (touch_tx,touch_rx) = channel::<(touchbar::ItemId,u32)>();
            let status_bar = NSStatusBar::systemStatusBar(nil);
            let date_cls = Class::get("NSDate").unwrap();
            let scrubber = Rc::new(Scrubber {
                devices: RefCell::new(vec!["one".to_string(), "two".to_string(),
                                           "a little bit longer one".to_string(),
                                           "three".to_string(),
                                           "this one is really quite a bit longer than the others".to_string()]),
                //Vec::<String>::new(),
                touch_tx: touch_tx,
            });
            bar = OSXStatusBar {
                app: app,
                status_bar_item: status_bar.statusItemWithLength_(NSVariableStatusItemLength),
                menu_bar: NSMenu::new(nil),
                object: NSObj::alloc(tx).setup(),
                touchbar: Touchbar::alloc(),
                pool: Cell::new(nil),
                run_count: Cell::new(0),
                run_mode: NSString::alloc(nil).init_str("kCFRunLoopDefaultMode"),
                run_date: msg_send![date_cls, distantPast],
                label: Cell::new(0),
                devices: Vec::new(),
                scrubber: scrubber.clone(),
            };
            // Don't become foreground app on launch
            bar.app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);

            // Default mode for menu bar items: blue highlight when selected
            msg_send![bar.status_bar_item, setHighlightMode:YES];

            // Set title.  Only displayed if image fails to load.
            let title = NSString::alloc(nil).init_str("connectr");
            NSButton::setTitle_(bar.status_bar_item, title);
            let _ = msg_send![title, release];

            // Look for icon in OS X bundle if there is one, otherwise current dir.
            // See docs/icons.md for explanation of icon files.
            // TODO: Use the full list of search paths.
            let icon_name = "connectr_80px_300dpi";
            let img_path = match bundled_resource_path(icon_name, "png") {
                Some(path) => path,
                None => format!("{}.png", icon_name),
            };

            // Set the status bar image.  Switching on setTemplate switches it to
            // using OS X system-style icons that are masked to all white.  I
            // prefer color, but that should maybe be configurable.
            let img = NSString::alloc(nil).init_str(&img_path);
            let icon = NSImage::alloc(nil).initWithContentsOfFile_(img);
            #[cfg(feature = "mac_white_icon")]
            let _ = msg_send![icon, setTemplate: YES]; // enable to make icon white
            bar.status_bar_item.button().setImage_(icon);
            bar.touchbar.set_icon(icon);
            let _ = msg_send![img, release];
            let _ = msg_send![icon, release];

            // Add the same image again as an alternate image.  I'm not sure how the
            // blending is performed, but it behaves differently and better if an
            // alt image is specified.  Without an alt image, the icon darkens too
            // much in 'dark mode' when selected, and is too light in 'light mode'.
            let img = NSString::alloc(nil).init_str(&img_path);
            let icon = NSImage::alloc(nil).initWithContentsOfFile_(img);
            let _ = msg_send![bar.status_bar_item.button(), setAlternateImage: icon];
            let _ = msg_send![img, release];
            let _ = msg_send![icon, release];

            bar.status_bar_item.setMenu_(bar.menu_bar);
            bar.object.cb_fn = Some(Box::new(
                move |s, sender| {
                    let cb = s.get_value(sender);
                    cb(sender, &s.tx);
                }
            ));

            let barid = bar.touchbar.create_bar();
            let text = NSString::alloc(nil).init_str("hi1");
            let b1id = bar.touchbar.create_button(nil, text, Box::new(move |_| {}));
            let text = NSString::alloc(nil).init_str("hi2");
            let b2id = bar.touchbar.create_button(nil, text, Box::new(move |_| {}));

            let popid = bar.touchbar.create_bar();
            let p1id = bar.touchbar.create_popover_item(popid);
            let text = NSString::alloc(nil).init_str("hi3");
            let b3id = bar.touchbar.create_button(nil, text, Box::new(move |_| {}));
            bar.touchbar.add_items_to_bar(popid, vec![b3id]);

            for item in ITEMS {
                bar.devices.push(item.to_string());
            }
            info!("devices: {:?}", bar.devices);
            info!("devices: {:?}", (&bar.devices) as *const Vec<String>);
            let bar_ptr = &bar as *const OSXStatusBar as *const u32;
            info!("bar ptr: {:?}", bar_ptr);
            //let testfn: Box<FnMut()> = Box::new(bar.test);
            let scrubber1 = scrubber.clone();
            let scrubber2 = scrubber.clone();
            let scrubber3 = scrubber.clone();
            let scrubber4 = scrubber.clone();
            let s1id = bar.touchbar.create_text_scrubber(
                Box::new(move |s|   { scrubber1.count(s) }),
                Box::new(move |s,i| { scrubber2.text(s,i) }),
                Box::new(move |s,i| { scrubber3.width(s,i) }),
                Box::new(move |s,i| { scrubber4.touch(s,i) }),
            );
            bar.touchbar.select_scrubber_item(s1id, 1);

            let l1id = bar.touchbar.create_label();
            bar.label.set(s1id);

            //bar.touchbar.add_items_to_bar(barid, vec![b1id, b2id, p1id]);
            bar.touchbar.add_items_to_bar(barid, vec![b1id, b2id, p1id, l1id, s1id]);
            bar.touchbar.set_bar_as_root(barid);

            let _: () = msg_send![app, finishLaunching];
            bar.touchbar.enable();
        }
        bar
    }    
    fn touchbar(&mut self) {
        info!("Touchbar fucker!");

        self.scrubber.devices.borrow_mut().push("a new device!".to_string());
        let l1id = self.label.get();
        //self.touchbar.select_scrubber_item(l1id, 0);
        self.touchbar.refresh_scrubber(l1id);
        //self.touchbar.update_label(l1id);
        unsafe {

            //let barid = self.touchbar.create_bar();
            //let text = NSString::alloc(nil).init_str("hi1");
            //let b1id = self.touchbar.create_button(nil, text, Box::new(move |_| {}));
            //let text = NSString::alloc(nil).init_str("hi2");
            //let b2id = self.touchbar.create_button(nil, text, Box::new(move |_| {}));
            //
            //let popid = self.touchbar.create_bar();
            //let p1id = self.touchbar.create_popover_item(popid);
            //let text = NSString::alloc(nil).init_str("hi3");
            //let b3id = self.touchbar.create_button(nil, text, Box::new(move |_| {}));
            //self.touchbar.add_items_to_bar(popid, vec![b3id]);
            //
            ////bar.touchbar.add_items_to_bar(barid, vec![b1id, b2id, p1id]);
            //self.touchbar.add_items_to_bar(barid, vec![b1id]);
            //self.touchbar.set_bar_as_root(barid);
        }
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
