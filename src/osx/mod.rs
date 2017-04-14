pub mod rustnsobject;

extern crate objc;
extern crate objc_foundation;
extern crate cocoa;

pub use self::rustnsobject::NSCallback;

use objc::runtime::{Class, Object};

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

use std::thread::sleep;
use std::time::Duration;

extern crate objc_id;
use self::objc_id::Id;

pub type MenuItem = *mut Object;

pub struct OSXStatusBar {
    object: NSObj,
    app: *mut objc::runtime::Object,
    status_bar_item: *mut objc::runtime::Object,
    menu_bar: *mut objc::runtime::Object,
}
pub trait TStatusBar {
    type S: TStatusBar;
    fn new(tx: Sender<String>) -> Self::S;
    fn add_separator(&mut self);
    fn add_label(&mut self, label: &str);
    fn add_item(&mut self, item: &str, callback: NSCallback, selected: bool) -> *mut Object;
    fn update_item(&mut self, item: *mut Object, label: &str);
    fn sel_item(&mut self, sender: u64);
    fn unsel_item(&mut self, sender: u64);
    fn set_tooltip(&self, text: &str);
    fn run(&mut self, block: bool);
}

impl TStatusBar for OSXStatusBar {
    type S = OSXStatusBar;
    fn new(tx: Sender<String>) -> OSXStatusBar {
        let mut bar;
        unsafe {
            let _ = NSAutoreleasePool::new(nil);
            let app = NSApp();
            let status_bar = NSStatusBar::systemStatusBar(nil);
            bar = OSXStatusBar {
                app: app,
                //status_bar_item: status_bar.statusItemWithLength_(NSSquareStatusItemLength),
                status_bar_item: status_bar.statusItemWithLength_(NSVariableStatusItemLength),
                menu_bar: NSMenu::new(nil).autorelease(),
                object: NSObj::alloc(tx).setup(),
            };
            bar.app.setActivationPolicy_(NSApplicationActivationPolicyAccessory);
            msg_send![bar.status_bar_item, setHighlightMode:YES];
            let img = NSString::alloc(nil).init_str("spotify.png");
            let icon = NSImage::alloc(nil).initWithContentsOfFile_(img);
            NSButton::setTitle_(bar.status_bar_item, NSString::alloc(nil).init_str("connectr"));
            bar.status_bar_item.button().setImage_(icon);
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
    fn set_tooltip(&self, text: &str) {
        unsafe {
            let img = NSString::alloc(nil).init_str(text);
            let _ = msg_send![self.status_bar_item.button(), setToolTip: img];
        }
    }
    fn add_label(&mut self, label: &str) {
        unsafe {
            let txt = NSString::alloc(nil).init_str(label);
            let quit_key = NSString::alloc(nil).init_str("");
            let app_menu_item = NSMenuItem::alloc(nil)
                .initWithTitle_action_keyEquivalent_(txt, self.object.selector(), quit_key)
                .autorelease();
            self.menu_bar.addItem_(app_menu_item);
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
                .initWithTitle_action_keyEquivalent_(txt, self.object.selector(), quit_key)
                .autorelease();
            self.object.add_callback(app_menu_item, callback);
            let objc = self.object.take_objc();
            let _: () = msg_send![app_menu_item, setTarget: objc];
            if selected {
                let _: () = msg_send![app_menu_item, setState: 1];
            }
            let item: *mut Object = app_menu_item;
            self.menu_bar.addItem_(app_menu_item);
            item
        }
    }
    fn update_item(&mut self, item: *mut Object, label: &str) {
        unsafe {
            let ns_label = NSString::alloc(nil).init_str(label);
            let _: () = msg_send![item, setTitle: ns_label];
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
        //unsafe {
            //self.app.run();
        //}
        let _ = unsafe {NSAutoreleasePool::new(nil)};
        unsafe { let _: () = msg_send![self.app, finishLaunching]; }
        loop {
            sleep(Duration::from_millis(50));
            unsafe {
                let _ = NSAutoreleasePool::new(nil);
                let cls = Class::get("NSDate").unwrap();
                let date: Id<Object> = msg_send![cls, distantPast];
                let mode = NSString::alloc(nil).init_str("kCFRunLoopDefaultMode");
                let event: Id<Object> = msg_send![self.app, nextEventMatchingMask: -1
                                                  untilDate: date inMode:mode dequeue: YES];
                let _ = msg_send![self.app, sendEvent: event];
                let _ = msg_send![self.app, updateWindows];
            }
            if !block { break; }
        }
    }
}
