mod rustnsobject;

extern crate objc;
extern crate objc_foundation;
extern crate cocoa;

extern crate fruitbasket;
use self::fruitbasket::FruitApp;

pub use ::TStatusBar;
pub use ::NSCallback;

use objc::runtime::Class;

use self::cocoa::base::{nil, YES};
use self::cocoa::appkit::NSStatusBar;
use self::cocoa::foundation::NSString;
use self::cocoa::appkit::{NSMenu,
                          NSMenuItem,
                          NSImage,
                          NSVariableStatusItemLength,
                          NSStatusItem,
                          NSButton};

use self::rustnsobject::{NSObj, NSObjTrait, NSObjCallbackTrait};

use std::sync::mpsc::Sender;
use std::ptr;
use std::ffi::CStr;

pub type Object = objc::runtime::Object;

pub struct OSXStatusBar {
    object: NSObj,
    app: FruitApp,
    status_bar_item: *mut objc::runtime::Object,
    menu_bar: *mut objc::runtime::Object,
}

impl TStatusBar for OSXStatusBar {
    type S = OSXStatusBar;
    fn new(tx: Sender<String>) -> OSXStatusBar {
        let mut bar;
        unsafe {
            let nsapp = FruitApp::new();
            nsapp.set_activation_policy(fruitbasket::ActivationPolicy::Prohibited);
            let status_bar = NSStatusBar::systemStatusBar(nil);
            bar = OSXStatusBar {
                app: nsapp,
                status_bar_item: status_bar.statusItemWithLength_(NSVariableStatusItemLength),
                menu_bar: NSMenu::new(nil),
                object: NSObj::alloc(tx),
            };

            // Default mode for menu bar items: blue highlight when selected
            let _: () = msg_send![bar.status_bar_item, setHighlightMode:YES];

            // Set title.  Only displayed if image fails to load.
            let title = NSString::alloc(nil).init_str("connectr");
            NSButton::setTitle_(bar.status_bar_item, title);
            let _: () = msg_send![title, release];

            // Look for icon in OS X bundle if there is one, otherwise current dir.
            // See docs/icons.md for explanation of icon files.
            // TODO: Use the full list of search paths.
            let icon_name = "connectr_80px_300dpi";
            let img_path = match fruitbasket::FruitApp::bundled_resource_path(icon_name, "png") {
                Some(path) => path,
                None => format!("{}.png", icon_name),
            };

            // Set the status bar image.  Switching on setTemplate switches it to
            // using OS X system-style icons that are masked to all white.  I
            // prefer color, but that should maybe be configurable.
            let img = NSString::alloc(nil).init_str(&img_path);
            let icon = NSImage::alloc(nil).initWithContentsOfFile_(img);
            #[cfg(feature = "mac_white_icon")]
            let _: () = msg_send![icon, setTemplate: YES]; // enable to make icon white
            bar.status_bar_item.button().setImage_(icon);
            let _: () = msg_send![img, release];
            let _: () = msg_send![icon, release];

            // Add the same image again as an alternate image.  I'm not sure how the
            // blending is performed, but it behaves differently and better if an
            // alt image is specified.  Without an alt image, the icon darkens too
            // much in 'dark mode' when selected, and is too light in 'light mode'.
            let img = NSString::alloc(nil).init_str(&img_path);
            let icon = NSImage::alloc(nil).initWithContentsOfFile_(img);
            let _: () = msg_send![bar.status_bar_item.button(), setAlternateImage: icon];
            let _: () = msg_send![img, release];
            let _: () = msg_send![icon, release];

            bar.status_bar_item.setMenu_(bar.menu_bar);
            bar.object.cb_fn = Some(Box::new(
                move |s, sender| {
                    let cb = s.get_value(sender);
                    cb(sender, &s.tx);
                }
            ));
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
            let _: () = msg_send![old_menu, removeAllItems];
            let _: () = msg_send![old_menu, release];
        }
    }
    fn set_tooltip(&mut self, text: &str) {
        unsafe {
            let img = NSString::alloc(nil).init_str(text);
            let _: () = msg_send![self.status_bar_item.button(), setToolTip: img];
            let _: () = msg_send![img, release];
        }
    }
    fn add_label(&mut self, label: &str) {
        unsafe {
            let txt = NSString::alloc(nil).init_str(label);
            let quit_key = NSString::alloc(nil).init_str("");
            let app_menu_item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(txt, self.object.selector(), quit_key);
            let _: () = msg_send![txt, release];
            let _: () = msg_send![quit_key, release];
            self.menu_bar.addItem_(app_menu_item);
            let _: () = msg_send![app_menu_item, release];
        }
    }
    fn add_quit(&mut self, label: &str) {
        unsafe {
            let txt = NSString::alloc(nil).init_str(label);
            let quit_key = NSString::alloc(nil).init_str("");
            let app_menu_item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(txt, sel!(terminate:), quit_key);
            let _: () = msg_send![txt, release];
            let _: () = msg_send![quit_key, release];
            self.menu_bar.addItem_(app_menu_item);
            let _: () = msg_send![app_menu_item, release];
        }
    }
    fn add_separator(&mut self) {
        unsafe {
            let cls = Class::get("NSMenuItem").unwrap();
            let sep: *mut Object = msg_send![cls, separatorItem];
            self.menu_bar.addItem_(sep);
        }
    }
    // TODO: whole API should accept menu option.  this whole thing should
    // be split out into its own recursive menu-builder trait.  this is
    // horrible.
    fn add_item(&mut self, menu: Option<*mut Object>,item: &str, callback: NSCallback, selected: bool) -> *mut Object {
        unsafe {
            let txt = NSString::alloc(nil).init_str(item);
            let quit_key = NSString::alloc(nil).init_str("");
            let app_menu_item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(txt, self.object.selector(), quit_key);
            let _: () = msg_send![txt, release];
            let _: () = msg_send![quit_key, release];
            self.object.add_callback(app_menu_item, callback);
            let objc = self.object.take_objc();
            let _: () = msg_send![app_menu_item, setTarget: objc];
            if selected {
                let _: () = msg_send![app_menu_item, setState: 1];
            }
            let item: *mut Object = app_menu_item;
            match menu {
                Some(menu) => { menu.addItem_(app_menu_item); },
                None => { self.menu_bar.addItem_(app_menu_item); }
            }
            let _: () = msg_send![app_menu_item, release];
            item
        }
    }
    fn add_submenu(&mut self, label: &str, callback: NSCallback) -> *mut Object {
        unsafe {
            let submenu = NSMenu::new(nil);
            let txt = NSString::alloc(nil).init_str(label);
            let quit_key = NSString::alloc(nil).init_str("");
            let app_menu_item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(txt,
                                                     self.object.selector(),
                                                     quit_key);
            self.object.add_callback(app_menu_item, callback);
            let objc = self.object.take_objc();
            let _: () = msg_send![app_menu_item, setTarget: objc];
            let _: () = msg_send![app_menu_item, setSubmenu: submenu];
            let _: () = msg_send![txt, release];
            let _: () = msg_send![quit_key, release];
            self.menu_bar.addItem_(app_menu_item);
            let _: () = msg_send![app_menu_item, release];
            let _: () = msg_send![submenu, release];
            submenu
        }
    }
    fn update_item(&mut self, item: *mut Object, label: &str) {
        unsafe {
            let ns_label = NSString::alloc(nil).init_str(label);
            let _: () = msg_send![item, setTitle: ns_label];
            let _: () = msg_send![ns_label, release];
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
    fn register_url_handler(&mut self) {
        unsafe {
            let cls = Class::get("NSAppleEventManager").unwrap();
            let manager: *mut Object = msg_send![cls, sharedAppleEventManager];
            let objc = self.object.take_objc();
            let _: () = msg_send![objc, handleURLEvent: 0 withReplyEvent: 0];
            let _: () = msg_send![manager, setEventHandler: objc
                                  andSelector: sel!(handleURLEvent:withReplyEvent:)
                                  forEventClass: 0x4755524c
                                  andEventID: 0x4755524c];
            info!("Registered URL handler");
            //let cb: NSCallback = Box::new(move |_sender, _tx| {
            //    info!("URL callback");
            //});
            //self.object.add_callback(objc, cb);
        }
    }
    fn run(&mut self, block: bool) {
        let period = match block {
            true => fruitbasket::RunPeriod::Forever,
            _ => fruitbasket::RunPeriod::Once,
        };
        let _ = self.app.run(period);
    }
}

//pub fn osx_alert(text: &str) {
//    unsafe {
//        let ns_text = NSString::alloc(nil).init_str(text);
//        let button = NSString::alloc(nil).init_str("ok");
//        let cls = Class::get("NSAlert").unwrap();
//        let alert: *mut Object = msg_send![cls, alloc];
//        let _: () = msg_send![alert, init];
//        let _: () = msg_send![alert, setMessageText: ns_text];
//        let _: () = msg_send![alert, addButtonWithTitle: button];
//        let _: () = msg_send![alert, runModal];
//        let _: () = msg_send![ns_text, release];
//        let _: () = msg_send![button, release];
//        let _: () = msg_send![alert, release];
//    }
//}

pub fn resource_dir() -> Option<String> {
    unsafe {
        let cls = Class::get("NSBundle").unwrap();
        let bundle: *mut Object = msg_send![cls, mainBundle];
        let path: *mut Object = msg_send![bundle, resourcePath];
        let cstr: *const i8 = msg_send![path, UTF8String];
        if cstr != ptr::null() {
            let rstr = CStr::from_ptr(cstr).to_string_lossy().into_owned();
            return Some(rstr);
        }
        None
    }
}
