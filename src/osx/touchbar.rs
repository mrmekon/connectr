// Copied from rustc-objc-foundation project by SSheldon, examples/custom_class.rs
// https://github.com/SSheldon/rust-objc-foundation/blob/master/examples/custom_class.rs
// Covered by MIT License: https://en.wikipedia.org/wiki/MIT_License

extern crate objc;
extern crate objc_foundation;
extern crate objc_id;
extern crate cocoa;

pub use ::NSCallback;

use std::sync::{Once, ONCE_INIT};

use objc::Message;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use self::objc_foundation::{INSObject, NSObject, INSArray, NSArray, INSString};
use self::cocoa::base::{nil, YES};
use self::cocoa::foundation::NSString;

//use self::objc_id::Id;
//use self::objc_id::Shared;

#[link(name = "DFRFoundation", kind = "framework")]
extern {
    pub fn DFRSystemModalShowsCloseBoxWhenFrontMost(x: i8);
    pub fn DFRElementSetControlStripPresenceForIdentifier(n: *mut Object, x: i8);
}

//pub struct RustTouchbarDelegate {
//    pub objc: Id<ObjcAppDelegate, Shared>,
//}
//
//impl RustTouchbarDelegate {
//    pub fn add_button() {}
//    pub fn add_quit_button() {}
//    pub fn add_label() {}
//    pub fn add_slider() {}
//}

pub enum ObjcAppDelegate {}
impl ObjcAppDelegate {}

unsafe impl Message for ObjcAppDelegate { }

static OBJC_SUBCLASS_REGISTER_CLASS: Once = ONCE_INIT;

impl INSObject for ObjcAppDelegate {
    fn class() -> &'static Class {
        OBJC_SUBCLASS_REGISTER_CLASS.call_once(|| {
            let superclass = NSObject::class();
            let mut decl = ClassDecl::new("ObjcAppDelegate", superclass).unwrap();
            decl.add_ivar::<u64>("_groupbar");
            decl.add_ivar::<u64>("_groupId");
            decl.add_ivar::<u64>("_icon");

            extern fn objc_group_touch_bar(this: &mut Object, _cmd: Sel) -> u64 {
                unsafe {*this.get_ivar("_groupbar")}
            }
            extern fn objc_set_group_touch_bar(this: &mut Object, _cmd: Sel, bar: u64) {
                unsafe {this.set_ivar("_groupbar", bar);}
            }
            extern fn objc_set_icon(this: &mut Object, _cmd: Sel, icon: u64) {
                unsafe {this.set_ivar("_icon", icon);}
            }
            extern fn objc_button(_this: &mut Object, _cmd: Sel, _sender: u64) {
                let sender = _sender as *mut Object;
                info!("Button push: {}", sender as u64);
                //unsafe {
                    //let slider: *mut Object = msg_send![sender, slider];
                    //let val: u32 = msg_send![slider, intValue];
                    //info!("Slider val: {}", val);
                //}
            }
            extern fn objc_present(this: &mut Object, _cmd: Sel, _sender: u64) {
                unsafe {
                    let ident: u64 = *this.get_ivar("_groupId");
                    let bar: u64 = *this.get_ivar("_groupbar");
                    let cls = Class::get("NSTouchBar").unwrap();
                    msg_send![cls,
                              presentSystemModalFunctionBar: bar
                              systemTrayItemIdentifier: ident];
                }
            }
            extern fn objc_touch_bar_make_item_for_identifier(_this: &mut Object, _cmd: Sel, _bar: u64, _id: u64) -> u64 {
                unsafe {
                    let id = _id as *mut Object;
                    let b1_ident = objc_foundation::NSString::from_str("com.trevorbentley.b1");
                    let b2_ident = objc_foundation::NSString::from_str("com.trevorbentley.b2");
                    let b3_ident = objc_foundation::NSString::from_str("com.trevorbentley.b3");
                    let slide_ident = objc_foundation::NSString::from_str("com.trevorbentley.slide");
                    if msg_send![b1_ident, isEqualToString: id] {
                        let cls = Class::get("NSButton").unwrap();
                        let icon_ptr: u64 = *_this.get_ivar("_icon");
                        let btn: *mut Object = msg_send![cls,
                                                         buttonWithImage:icon_ptr
                                                         target:_this
                                                         action:sel!(button:)];
                        let cls = Class::get("NSCustomTouchBarItem").unwrap();
                        let item: *mut Object = msg_send![cls, alloc];
                        let item: *mut Object = msg_send![item, initWithIdentifier: b1_ident];
                        msg_send![item, setView: btn];
                        return item as u64;
                    }
                    else if msg_send![b2_ident, isEqualToString: id] {
                        let cls = Class::get("NSButton").unwrap();
                        let icon_ptr: u64 = *_this.get_ivar("_icon");
                        let btn: *mut Object = msg_send![cls,
                                                         buttonWithImage:icon_ptr
                                                         target:_this
                                                         action:sel!(button:)];
                        let cls = Class::get("NSCustomTouchBarItem").unwrap();
                        let item: *mut Object = msg_send![cls, alloc];
                        let item: *mut Object = msg_send![item, initWithIdentifier: b2_ident];
                        msg_send![item, setView: btn];
                        return item as u64;
                    }
                    else if msg_send![b3_ident, isEqualToString: id] {
                        let label = objc_foundation::NSString::from_str("Quit");
                        let cls = Class::get("NSButton").unwrap();
                        let btn: *mut Object = msg_send![cls,
                                                         buttonWithTitle:label
                                                         target: nil
                                                         action:sel!(terminate:)];
                        let cls = Class::get("NSCustomTouchBarItem").unwrap();
                        let item: *mut Object = msg_send![cls, alloc];
                        let item: *mut Object = msg_send![item, initWithIdentifier: b3_ident];
                        msg_send![item, setView: btn];
                        return item as u64;
                    }
                    else if msg_send![slide_ident, isEqualToString: id] {
                        let cls = Class::get("NSSliderTouchBarItem").unwrap();
                        let item: *mut Object = msg_send![cls, alloc];
                        let item: *mut Object = msg_send![item, initWithIdentifier: slide_ident];
                        let slider: *mut Object = msg_send![item, slider];
                        msg_send![slider, setMinValue: 0f32];
                        msg_send![slider, setMaxValue: 100.0];
                        msg_send![slider, setContinuous: YES];
                        msg_send![item, setTarget: _this];
                        msg_send![item, setAction: sel!(button:)];
                        return item as u64;
                    }
                }
                0
            }
            extern fn objc_application_did_finish_launching(this: &mut Object, _cmd: Sel, _notification: u64) {
                unsafe {
                    DFRSystemModalShowsCloseBoxWhenFrontMost(YES);

                    // Initialize touchbar singleton with button layout
                    // TODO: break out into function.  this needs to be runtime reconfigured
                    let b1_ident = objc_foundation::NSString::from_str("com.trevorbentley.b1");
                    let b2_ident = objc_foundation::NSString::from_str("com.trevorbentley.b2");
                    let b3_ident = objc_foundation::NSString::from_str("com.trevorbentley.b3");
                    let slide_ident = objc_foundation::NSString::from_str("com.trevorbentley.slide");
                    let cls = Class::get("NSTouchBar").unwrap();
                    let bar: *mut Object = msg_send![cls, alloc];
                    let bar: *mut objc::runtime::Object = msg_send![bar, init];
                    let idents = NSArray::from_vec(vec![b1_ident, b2_ident, b3_ident, slide_ident]);
                    let _ : () = msg_send![bar, setDefaultItemIdentifiers: idents];
                    let this_copy = this as *mut Object as u64;
                    let _ : () = msg_send![bar, setDelegate: this_copy as *mut Object];
                    let _ : () = msg_send![this, setGroupTouchBar: bar];

                    // Add icon to touchbar's Control Strip.
                    let ident = NSString::alloc(nil).init_str("com.trevorbentley.group");
                    this.set_ivar("_groupId", ident as u64);
                    let cls = Class::get("NSCustomTouchBarItem").unwrap();
                    let item: *mut Object = msg_send![cls, alloc];
                    msg_send![item, initWithIdentifier:ident];

                    let cls = Class::get("NSButton").unwrap();
                    let icon_ptr: u64 = *this.get_ivar("_icon");
                    let btn: *mut Object = msg_send![cls,
                                                     buttonWithImage:icon_ptr
                                                     target:this
                                                     action:sel!(present:)];
                    msg_send![item, setView:btn];

                    let cls = Class::get("NSTouchBarItem").unwrap();
                    msg_send![cls, addSystemTrayItem: item];
                    DFRElementSetControlStripPresenceForIdentifier(ident, YES);
                }
            }

            unsafe {
                let f: extern fn(&mut Object, Sel, u64, u64) -> u64 = objc_touch_bar_make_item_for_identifier;
                decl.add_method(sel!(touchBar:makeItemForIdentifier:), f);

                let f: extern fn(&mut Object, Sel, u64) = objc_application_did_finish_launching;
                decl.add_method(sel!(applicationDidFinishLaunching:), f);

                let f: extern fn(&mut Object, Sel, u64) = objc_present;
                decl.add_method(sel!(present:), f);

                let f: extern fn(&mut Object, Sel, u64) = objc_button;
                decl.add_method(sel!(button:), f);

                let f: extern fn(&mut Object, Sel) -> u64 = objc_group_touch_bar;
                decl.add_method(sel!(groupTouchBar), f);

                let f: extern fn(&mut Object, Sel, u64) = objc_set_group_touch_bar;
                decl.add_method(sel!(setGroupTouchBar:), f);

                let f: extern fn(&mut Object, Sel, u64) = objc_set_icon;
                decl.add_method(sel!(setIcon:), f);
            }

            decl.register();
        });

        Class::get("ObjcAppDelegate").unwrap()
    }
}
