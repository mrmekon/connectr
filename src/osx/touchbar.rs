// Copied from rustc-objc-foundation project by SSheldon, examples/custom_class.rs
// https://github.com/SSheldon/rust-objc-foundation/blob/master/examples/custom_class.rs
// Covered by MIT License: https://en.wikipedia.org/wiki/MIT_License

extern crate objc;
extern crate objc_foundation;
extern crate objc_id;
extern crate cocoa;

use std::cell::Cell;
use std::sync::{Once, ONCE_INIT};
use std::collections::BTreeMap;

use objc::Message;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use self::objc_foundation::{INSObject, NSObject, INSArray, NSArray, INSString};
use self::cocoa::base::{nil, YES, NO};
use self::cocoa::foundation::NSString;
use self::cocoa::foundation::{NSRect, NSPoint, NSSize};
use self::cocoa::appkit::NSApp;
use self::objc_id::Id;
use self::objc_id::Shared;

const IDENT_PREFIX: &'static str = "com.trevorbentley.";

#[link(name = "DFRFoundation", kind = "framework")]
extern {
    pub fn DFRSystemModalShowsCloseBoxWhenFrontMost(x: i8);
    pub fn DFRElementSetControlStripPresenceForIdentifier(n: *mut Object, x: i8);
}

extern crate libc;
use std::ffi::CStr;
fn print_nsstring(str: *mut Object) {
    unsafe {
        let cstr: *const libc::c_char = msg_send![str, UTF8String];
        let rstr = CStr::from_ptr(cstr).to_string_lossy().into_owned();
        info!("{}", rstr);
    }
}

pub trait TScrubberData {
    fn count(&self, item: ItemId) -> u32;
    fn text(&self, item: ItemId, idx: u32) -> String;
    fn width(&self, item: ItemId, idx: u32) -> u32;
    fn touch(&self, item: ItemId, idx: u32);
}

//use std::rc::Rc;

struct Scrubber {
    //struct Scrubber<T> where T: TScrubberData {
    //data: Rc<T>,
    count_cb: ScrubberCountFn,
    text_cb: ScrubberTextFn,
    width_cb: ScrubberWidthFn,
    touch_cb: ScrubberTouchFn,
    _item: ItemId,
    _scrubber: ItemId,
    _ident: Ident,
}

pub struct RustTouchbarDelegateWrapper {
    objc: Id<ObjcAppDelegate, Shared>,
    next_item_id: Cell<u64>,
    bar_obj_map: BTreeMap<ItemId, Ident>,
    control_obj_map: BTreeMap<ItemId, ItemId>,
    scrubber_obj_map: BTreeMap<ItemId, Scrubber>,
}

pub type Touchbar = Box<RustTouchbarDelegateWrapper>;

pub trait TouchbarTrait {
    fn alloc() -> Touchbar;
    fn set_icon(&self, icon: *mut Object);
    fn enable(&self);
    fn create_bar(&mut self) -> BarId;
    fn create_popover_item(&mut self, bar_id: BarId) -> BarId;
    fn add_items_to_bar(&mut self, bar_id: BarId, items: Vec<ItemId>);
    fn set_bar_as_root(&mut self, bar_id: BarId);
    fn create_label(&mut self) -> ItemId;
    fn update_label(&mut self, label_id: ItemId);
    fn create_text_scrubber(&mut self,
                            count_fn: ScrubberCountFn,
                            text_fn: ScrubberTextFn,
                            width_fn: ScrubberWidthFn,
                            touch_fn: ScrubberTouchFn) -> ItemId;
    fn select_scrubber_item(&mut self, scrub_id: ItemId, index: u32);
    fn refresh_scrubber(&mut self, scrub_id: ItemId);
    fn create_button(&mut self, image: *mut Object, text: *mut Object, cb: ButtonCb) -> ItemId;
}

//pub type ScrubberCountFn = fn(*const u32, ItemId) -> u32;
//pub type ScrubberTextFn = fn(*const u32, ItemId, u32) -> String;
//pub type ScrubberWidthFn = fn(*const u32, ItemId, u32) -> u32;
//pub type ScrubberTouchFn = fn(*const u32, ItemId, u32);

pub type ScrubberCountFn = Box<Fn(ItemId) -> u32>;
pub type ScrubberTextFn = Box<Fn(ItemId, u32) -> String>;
pub type ScrubberWidthFn = Box<Fn(ItemId, u32) -> u32>;
pub type ScrubberTouchFn = Box<Fn(ItemId, u32)>;


impl RustTouchbarDelegateWrapper {
    fn generate_ident(&mut self) -> u64 {
        unsafe {
            // Create string identifier
            let next_item_id = self.next_item_id.get();
            self.next_item_id.set(next_item_id + 1);
            let ident = format!("{}{}", IDENT_PREFIX, next_item_id);
            let objc_ident = NSString::alloc(nil).init_str(&ident);
            objc_ident as u64
        }
    }
}

impl TouchbarTrait for Touchbar {
    fn alloc() -> Touchbar {
        let objc = ObjcAppDelegate::new().share();
        let rust = Box::new(RustTouchbarDelegateWrapper {
            objc: objc.clone(),
            next_item_id: Cell::new(0),
            bar_obj_map: BTreeMap::<ItemId, Ident>::new(),
            control_obj_map: BTreeMap::<ItemId, ItemId>::new(),
            scrubber_obj_map: BTreeMap::<ItemId, Scrubber>::new(),
        });
        unsafe {
            let ptr: u64 = &*rust as *const RustTouchbarDelegateWrapper as u64;
            let _ = msg_send![rust.objc, setRustWrapper: ptr];
        }
        return rust
    }
    fn set_icon(&self, icon: *mut Object) {
        unsafe { let _:() = msg_send![self.objc, setIcon: icon]; }
    }
    fn enable(&self) {
        unsafe {
            let app = NSApp();
            let _: () = msg_send![app, setDelegate: self.objc.clone()];
            //let _: () = msg_send![self.objc, applicationDidFinishLaunching: 0];
        }
    }

//    pub fn add_button() {}
//    pub fn add_quit_button() {}
//    pub fn add_label() {}
    //    pub fn add_slider() {}
    fn create_bar(&mut self) -> BarId {
        unsafe {
            let ident = self.generate_ident();
            // Create touchbar
            let cls = Class::get("NSTouchBar").unwrap();
            let bar: *mut Object = msg_send![cls, alloc];
            let bar: *mut objc::runtime::Object = msg_send![bar, init];
            let _ : () = msg_send![bar, retain];
            let _ : () = msg_send![bar, setDelegate: self.objc.clone()];
            // Save tuple
            self.bar_obj_map.insert(bar as u64, ident as u64);
            info!("bar: {}", bar as u64);
            bar as u64
        }
    }
    fn create_popover_item(&mut self, bar_id: BarId) -> ItemId {
        unsafe {
            let bar = bar_id as *mut Object;
            let bar_ident = *self.bar_obj_map.get(&bar_id).unwrap() as *mut Object; 
            let ident = self.generate_ident();
            // Save tuple
            let cls = Class::get("NSPopoverTouchBarItem").unwrap();
            let item: *mut Object = msg_send![cls, alloc];
            let item: *mut Object = msg_send![item, initWithIdentifier: ident];
            let cls = Class::get("NSButton").unwrap();
            let text = NSString::alloc(nil).init_str("pop");
            let btn: *mut Object = msg_send![cls,
                                             buttonWithTitle:text
                                             target:self.objc.clone()
                                             action:sel!(popbar:)];
            let _:() = msg_send![item, setShowsCloseButton: YES];
            let gesture: *mut Object = msg_send![item, makeStandardActivatePopoverGestureRecognizer];
            let _:() = msg_send![btn, addGestureRecognizer: gesture];
            let _:() = msg_send![item, setCollapsedRepresentation: btn];
            
            //let idents = NSArray::from_vec(vec![b1_ident]);
            //let _ : () = msg_send![bar, setDefaultItemIdentifiers: idents];
            let _:() = msg_send![item, setPopoverTouchBar: bar];
            let _:() = msg_send![item, setPressAndHoldTouchBar: bar];

            self.bar_obj_map.insert(item as u64, ident as u64);
            self.control_obj_map.insert(btn as u64, item as u64);
            item as u64
        }
    }    
    fn add_items_to_bar(&mut self, bar_id: BarId, items: Vec<ItemId>) {
        unsafe {
            let cls = Class::get("NSMutableArray").unwrap();
            let idents: *mut Object = msg_send![cls, arrayWithCapacity: items.len()];
            for item in items {
                let ident = *self.bar_obj_map.get(&item).unwrap() as *mut Object;
                let _ : () = msg_send![idents, addObject: ident];
            }
            let bar = bar_id as *mut Object;
            let _ : () = msg_send![bar, setDefaultItemIdentifiers: idents];
        }
    }
    fn set_bar_as_root(&mut self, bar_id: BarId) {
        unsafe {
            let old_bar: *mut Object = msg_send![self.objc, groupTouchBar];
            if old_bar != nil {
                // TODO: store in temp place until it's not visible,
                // delete and replace when it is closed.  otherwise
                // it forces the bar to close on each update.
                let visible: bool = msg_send![old_bar, isVisible];
                info!("DELETING OLD BAR: {}", visible);
                let cls = Class::get("NSTouchBar").unwrap();
                msg_send![cls, dismissSystemModalFunctionBar: old_bar];
            }
            let _ : () = msg_send![self.objc, setGroupTouchBar: bar_id];
            let ident: *mut Object = *self.bar_obj_map.get(&bar_id).unwrap() as *mut Object;
            let _ : () = msg_send![self.objc, setGroupIdent: ident];
            let _: () = msg_send![self.objc, applicationDidFinishLaunching: 0];
        }
    }
    fn create_label(&mut self) -> ItemId {
        unsafe {
            let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(300., 44.));
            let cls = Class::get("NSTextField").unwrap();
            let label: *mut Object = msg_send![cls, alloc];
            let label: *mut Object = msg_send![label, initWithFrame: frame];
            let _:() = msg_send![label, setEditable: NO];
            let text = NSString::alloc(nil).init_str("froop doop poop\nsecond level");
            let _:() = msg_send![label, setStringValue: text];

            let ident = self.generate_ident();
            let cls = Class::get("NSCustomTouchBarItem").unwrap();
            let item: *mut Object = msg_send![cls, alloc];
            let item: *mut Object = msg_send![item, initWithIdentifier: ident];
            msg_send![item, setView: label];

            self.bar_obj_map.insert(item as u64, ident as u64);
            self.control_obj_map.insert(label as u64, item as u64);
            item as u64
        } 
    }
    fn update_label(&mut self, label_id: ItemId) {
        unsafe {
            let item: *mut Object = label_id as *mut Object;
            let label: *mut Object = msg_send![item, view];
            let text = NSString::alloc(nil).init_str("updated\nthis shit");
            let _:() = msg_send![label, setStringValue: text];
        }
    }
    fn create_text_scrubber(&mut self,
                            count_fn: ScrubberCountFn,
                            text_fn: ScrubberTextFn,
                            width_fn: ScrubberWidthFn,
                            touch_fn: ScrubberTouchFn) -> ItemId {
        unsafe {
            let ident = self.generate_ident();
            let cls = Class::get("NSCustomTouchBarItem").unwrap();
            let item: *mut Object = msg_send![cls, alloc];
            let item: *mut Object = msg_send![item, initWithIdentifier: ident];
            
            // note: frame is ignored, but must be provided.
            let frame = NSRect::new(NSPoint::new(0., 0.), NSSize::new(0., 30.));
            let cls = Class::get("NSScrubber").unwrap();
            let scrubber: *mut Object = msg_send![cls, alloc];
            let scrubber: *mut Object = msg_send![scrubber, initWithFrame: frame];

            let cls = Class::get("NSScrubberSelectionStyle").unwrap();
            let style: *mut Object = msg_send![cls, outlineOverlayStyle];
            
            let cls = Class::get("NSScrubberTextItemView").unwrap();
            let _:() = msg_send![scrubber, registerClass: cls forItemIdentifier: ident];
            let _:() = msg_send![scrubber, setDelegate: self.objc.clone()];
            let _:() = msg_send![scrubber, setDataSource: self.objc.clone()];
            let _:() = msg_send![scrubber, setSelectionOverlayStyle: style];
            let _:() = msg_send![scrubber, setMode: 1]; // NSScrubberModeFree
            //(*scrubber).set_ivar("selectedIndex", 3);
            //let _:() = msg_send![scrubber, ];
            let _:() = msg_send![item, setView: scrubber];

            self.bar_obj_map.insert(item as u64, ident as u64);
            let scrub_struct = Scrubber {
                text_cb: text_fn,
                count_cb: count_fn,
                width_cb: width_fn,
                touch_cb: touch_fn,
                _ident: ident as u64,
                _item: item as u64,
                _scrubber: scrubber as u64,
            };
            self.scrubber_obj_map.insert(scrubber as u64, scrub_struct);
            item as u64
        }
    } 
    fn select_scrubber_item(&mut self, scrub_id: ItemId, index: u32) {
        unsafe {
            let item = scrub_id as *mut Object;
            let scrubber: *mut Object = msg_send![item, view];
            let _:() = msg_send![scrubber, setSelectedIndex: index];
        }
    }
    fn refresh_scrubber(&mut self, scrub_id: ItemId) {
        unsafe {
            let item = scrub_id as *mut Object;
            let scrubber: *mut Object = msg_send![item, view];
            //let layout: *mut Object = msg_send![scrubber, scrubberLayout];
            //let _:() = msg_send![layout, invalidateLayout];
            let _:() = msg_send![scrubber, reloadData];
        }
    }
    fn create_button(&mut self, image: *mut Object, text: *mut Object, cb: ButtonCb) -> ItemId {
        unsafe {
            let ident = self.generate_ident();
            let cls = Class::get("NSButton").unwrap();
            let btn: *mut Object;
            // Match on (image, text) as booleans.   false == null.
            match ((image as u64) != 0, (text as u64) != 0) {
                (false,true) => {
                    btn = msg_send![cls,
                                    buttonWithTitle: text
                                    target:self.objc.clone()
                                    action:sel!(button:)];
                }
                (true,false) => {
                    btn = msg_send![cls,
                                    buttonWithImage: image
                                    target:self.objc.clone()
                                    action:sel!(button:)];
                }
                (true,true) => {
                    btn = msg_send![cls,
                                    buttonWithTitle: text
                                    image:image
                                    target:self.objc.clone()
                                    action:sel!(button:)];
                }
                _ => { return 0 }
            }

            let cls = Class::get("NSCustomTouchBarItem").unwrap();
            let item: *mut Object = msg_send![cls, alloc];
            let item: *mut Object = msg_send![item, initWithIdentifier: ident];
            msg_send![item, setView: btn];

            self.bar_obj_map.insert(item as u64, ident as u64);
            self.control_obj_map.insert(btn as u64, item as u64);
            item as u64
        }
    }
}

pub type BarId = u64;
pub type ItemId = u64;
pub type Ident = u64;
//pub type ButtonCb = Box<Fn(u64, &Sender<String>)>;
pub type ButtonCb = Box<Fn(u64)>;

pub enum ObjcAppDelegate {}
impl ObjcAppDelegate {}

unsafe impl Message for ObjcAppDelegate { }

static OBJC_SUBCLASS_REGISTER_CLASS: Once = ONCE_INIT;

impl INSObject for ObjcAppDelegate {
    fn class() -> &'static Class {
        OBJC_SUBCLASS_REGISTER_CLASS.call_once(|| {
            let superclass = NSObject::class();
            let mut decl = ClassDecl::new("ObjcAppDelegate", superclass).unwrap();
            decl.add_ivar::<u64>("_rust_wrapper");
            decl.add_ivar::<u64>("_groupbar");
            decl.add_ivar::<u64>("_groupId");
            decl.add_ivar::<u64>("_icon");
            decl.add_ivar::<u64>("_popbar");
            decl.add_ivar::<u64>("_popover");

            extern fn objc_set_rust_wrapper(this: &mut Object, _cmd: Sel, ptr: u64) {
                unsafe {this.set_ivar("_rust_wrapper", ptr);}
            }
            extern fn objc_group_touch_bar(this: &mut Object, _cmd: Sel) -> u64 {
                unsafe {*this.get_ivar("_groupbar")}
            }
            extern fn objc_set_group_touch_bar(this: &mut Object, _cmd: Sel, bar: u64) {
                unsafe {this.set_ivar("_groupbar", bar);}
            }
            extern fn objc_set_group_ident(this: &mut Object, _cmd: Sel, bar: u64) {
                unsafe {this.set_ivar("_groupId", bar);}
            }
            extern fn objc_set_icon(this: &mut Object, _cmd: Sel, icon: u64) {
                unsafe {this.set_ivar("_icon", icon);}
            }
            extern fn objc_number_of_items_for_scrubber(this: &mut Object, _cmd: Sel,
                                                        scrub: u64) -> u32 {
                info!("scrubber item count");
                unsafe {
                    let ptr: u64 = *this.get_ivar("_rust_wrapper");
                    let wrapper = &mut *(ptr as *mut RustTouchbarDelegateWrapper);
                    let scrubber = scrub as *mut Object;
                    let scrub_struct = wrapper.scrubber_obj_map.get(&scrub).unwrap();
                    let item = scrub_struct._item;
                    (scrub_struct.count_cb)(item)
                }
            }
            extern fn objc_scrubber_view_for_item_at_index(this: &mut Object, _cmd: Sel,
                                                           scrub: u64, idx: u32) -> u64 {
                info!("scrubber item view");
                unsafe {
                    let ptr: u64 = *this.get_ivar("_rust_wrapper");
                    let wrapper = &mut *(ptr as *mut RustTouchbarDelegateWrapper);
                    let scrubber = scrub as *mut Object;
                    let scrub_struct = wrapper.scrubber_obj_map.get(&scrub).unwrap();
                    let item = scrub_struct._item;
                    let ident = scrub_struct._ident as *mut Object;
                    let view: *mut Object = msg_send![scrubber,
                                                      makeItemWithIdentifier:ident owner:nil];
                    let text = (scrub_struct.text_cb)(item, idx);
                    let text_field: *mut Object = msg_send![view, textField];
                    let objc_text: *mut Object = NSString::alloc(nil).init_str(&text);
                    let _:() = msg_send![text_field, setStringValue: objc_text];
                    view as u64
                }
            }
            extern fn objc_scrubber_layout_size_for_item_at_index(this: &mut Object, _cmd: Sel,
                                                                  scrub: u64,
                                                                  _layout: u64, idx: u32) -> NSSize {
                info!("scrubber item size");
                unsafe {
                    let ptr: u64 = *this.get_ivar("_rust_wrapper");
                    let wrapper = &mut *(ptr as *mut RustTouchbarDelegateWrapper);
                    let scrubber = scrub as *mut Object;
                    let scrub_struct = wrapper.scrubber_obj_map.get(&scrub).unwrap();
                    let item = scrub_struct._item;
                    info!("scrubber item size call CB");
                    let width = (scrub_struct.width_cb)(item, idx);
                    NSSize::new(width as f64, 30.)
                }
            }
            extern fn objc_scrubber_did_select_item_at_index(this: &mut Object, _cmd: Sel,
                                                             scrub: u64, idx: u32) {
                info!("scrubber selected");
                unsafe {
                    let ptr: u64 = *this.get_ivar("_rust_wrapper");
                    let wrapper = &mut *(ptr as *mut RustTouchbarDelegateWrapper);
                    let scrubber = scrub as *mut Object;
                    let scrub_struct = wrapper.scrubber_obj_map.get(&scrub).unwrap();
                    let item = scrub_struct._item;
                    (scrub_struct.touch_cb)(item, idx);
                }
            }
            extern fn objc_popbar(this: &mut Object, _cmd: Sel, sender: u64) {
                info!("Popbar push: {}", sender as u64);
                unsafe {
                    let ptr: u64 = *this.get_ivar("_rust_wrapper");
                    let wrapper = &mut *(ptr as *mut RustTouchbarDelegateWrapper);
                    
                    info!("got wrapper");
                    let btn = sender as *mut Object;
                    info!("got button");
                    let item = *wrapper.control_obj_map.get(&sender).unwrap() as *mut Object;
                    info!("got item");
                    let bar: *mut Object = msg_send![item, popoverTouchBar];
                    info!("got bar");
                    let ident = *wrapper.bar_obj_map.get(&(bar as u64)).unwrap() as *mut Object;
                    info!("got ident");

                    let cls = Class::get("NSTouchBar").unwrap();
                    msg_send![cls,
                              presentSystemModalFunctionBar: bar
                              systemTrayItemIdentifier: ident];
                    let app = NSApp();
                    let _:() = msg_send![app, setTouchBar: nil];
                }

                //unsafe {
                //    // Present the request popover.  This must be done instead of
                //    // using the popover's built-in showPopover because that pops
                //    // _under_ a system function bar.
                //    let ptr: u64 = *this.get_ivar("_popbar");
                //    let bar = ptr as *mut Object;
                //    let ptr: u64 = *this.get_ivar("_popover");
                //    let item = ptr as *mut Object;
                //    msg_send![item, showPopover: bar];
                //    let pop_ident = objc_foundation::NSString::from_str("com.trevorbentley.pop");
                //    let cls = Class::get("NSTouchBar").unwrap();
                //    msg_send![cls,
                //              presentSystemModalFunctionBar: bar
                //              systemTrayItemIdentifier: pop_ident];
                //
                //    // Alternative: popup, and close the main bar
                //    //let bar: u64 = *this.get_ivar("_groupbar");
                //    //let cls = Class::get("NSTouchBar").unwrap();
                //    //msg_send![cls, minimizeSystemModalFunctionBar: bar];
                //    //msg_send![item, showPopover: bar];
                //
                //}
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
                info!("present");
                unsafe {
                    let ident_int: u64 = *this.get_ivar("_groupId");
                    let bar_int: u64 = *this.get_ivar("_groupbar");
                    let ident = ident_int as *mut Object;
                    print_nsstring(ident);
                    let bar = bar_int as *mut Object;
                    let cls = Class::get("NSTouchBar").unwrap();
                    info!("present msg_send");
                    let len: u64 = msg_send![ident, length];
                    info!("set _groupId len: {}", len);
                    info!("bar: {}", bar_int);
                    msg_send![cls,
                              presentSystemModalFunctionBar: bar
                              systemTrayItemIdentifier: ident];
                    info!("present sent");
                }
            }
            extern fn objc_touch_bar_make_item_for_identifier(this: &mut Object, _cmd: Sel,
                                                              _bar: u64, id_ptr: u64) -> u64 {
                info!("MAKE");
                unsafe {
                    // Find the touchbar item matching this identifier in the
                    // Objective-C object map of the Rust wrapper class, and
                    // return it if found.
                    let id = id_ptr as *mut Object;
                    print_nsstring(id);
                    let ptr: u64 = *this.get_ivar("_rust_wrapper");
                    let wrapper = &mut *(ptr as *mut RustTouchbarDelegateWrapper);
                    for (obj_ref, ident_ref) in &wrapper.bar_obj_map {
                        let ident = *ident_ref as *mut Object;
                        let obj = *obj_ref as *mut Object;
                        if msg_send![id, isEqualToString: ident] {
                            return obj as u64;
                        }
                    }

                    let b1_ident = objc_foundation::NSString::from_str("com.trevorbentley.b1");
                    let b2_ident = objc_foundation::NSString::from_str("com.trevorbentley.b2");
                    let b3_ident = objc_foundation::NSString::from_str("com.trevorbentley.b3");
                    let slide_ident = objc_foundation::NSString::from_str("com.trevorbentley.slide");
                    let pop_ident = objc_foundation::NSString::from_str("com.trevorbentley.pop");
                    if msg_send![b1_ident, isEqualToString: id] {
                        let cls = Class::get("NSButton").unwrap();
                        let icon_ptr: u64 = *this.get_ivar("_icon");
                        let btn: *mut Object = msg_send![cls,
                                                         buttonWithImage:icon_ptr
                                                         target:this
                                                         action:sel!(button:)];
                        let cls = Class::get("NSCustomTouchBarItem").unwrap();
                        let item: *mut Object = msg_send![cls, alloc];
                        let item: *mut Object = msg_send![item, initWithIdentifier: b1_ident];
                        msg_send![item, setView: btn];
                        return item as u64;
                    }
                    else if msg_send![b2_ident, isEqualToString: id] {
                        let cls = Class::get("NSButton").unwrap();
                        let icon_ptr: u64 = *this.get_ivar("_icon");
                        let btn: *mut Object = msg_send![cls,
                                                         buttonWithImage:icon_ptr
                                                         target:this
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
                        msg_send![item, setTarget: this];
                        msg_send![item, setAction: sel!(button:)];
                        return item as u64;
                    }
                    else if msg_send![pop_ident, isEqualToString: id] {
                        let cls = Class::get("NSPopoverTouchBarItem").unwrap();
                        let item: *mut Object = msg_send![cls, alloc];
                        let item: *mut Object = msg_send![item, initWithIdentifier: pop_ident];
                        let cls = Class::get("NSButton").unwrap();
                        let icon_ptr: u64 = *this.get_ivar("_icon");
                        let this_copy = this as *mut Object as u64;
                        let btn: *mut Object = msg_send![cls,
                                                         buttonWithImage:icon_ptr
                                                         // TODO: instead of showPopover, use a
                                                         // handler that minimizes the group bar
                                                         // before doing the pop.
                                                         //
                                                         // Alternative: present on top of current
                                                         // one.
                                                         target:this_copy
                                                         action:sel!(popbar:)];
                        let _:() = msg_send![item, setShowsCloseButton: YES];
                        let gesture: *mut Object = msg_send![item, makeStandardActivatePopoverGestureRecognizer];
                        let _:() = msg_send![btn, addGestureRecognizer: gesture];
                        let _:() = msg_send![item, setCollapsedRepresentation: btn];
                        let cls = Class::get("NSTouchBar").unwrap();
                        let bar: *mut Object = msg_send![cls, alloc];
                        let bar: *mut objc::runtime::Object = msg_send![bar, init];
                        let this_copy = this as *mut Object as u64;
                        let _ : () = msg_send![bar, setDelegate: this_copy as *mut Object];
                        let idents = NSArray::from_vec(vec![b1_ident]);
                        let _ : () = msg_send![bar, setDefaultItemIdentifiers: idents];
                        let _:() = msg_send![item, setPopoverTouchBar: bar];
                        let _:() = msg_send![item, setPressAndHoldTouchBar: bar];
                        this.set_ivar("_popbar", bar as u64);
                        this.set_ivar("_popover", item as u64);
                        
                        return item as u64;
                    }
                }
                0
            }
            extern fn objc_application_did_finish_launching(this: &mut Object, _cmd: Sel, _notification: u64) {
                unsafe {
                    DFRSystemModalShowsCloseBoxWhenFrontMost(YES);

//                    // Initialize touchbar singleton with button layout
//                    // TODO: break out into function.  this needs to be runtime reconfigured
//                    let b1_ident = objc_foundation::NSString::from_str("com.trevorbentley.b1");
//                    let b2_ident = objc_foundation::NSString::from_str("com.trevorbentley.b2");
//                    let b3_ident = objc_foundation::NSString::from_str("com.trevorbentley.b3");
//                    let slide_ident = objc_foundation::NSString::from_str("com.trevorbentley.slide");
//                    let pop_ident = objc_foundation::NSString::from_str("com.trevorbentley.pop");
//                    let cls = Class::get("NSTouchBar").unwrap();
//                    let bar: *mut Object = msg_send![cls, alloc];
//                    let bar: *mut objc::runtime::Object = msg_send![bar, init];
//                    let idents = NSArray::from_vec(vec![b1_ident, b2_ident, b3_ident,
//                                                        slide_ident, pop_ident]);
//                    let _ : () = msg_send![bar, setDefaultItemIdentifiers: idents];
//                    let this_copy = this as *mut Object as u64;
//                    let _ : () = msg_send![bar, setDelegate: this_copy as *mut Object];
//                    let _ : () = msg_send![this, setGroupTouchBar: bar];

                    // Add icon to touchbar's Control Strip.
                    //let ident = NSString::alloc(nil).init_str("com.trevorbentley.group");
                    //info!("set _groupId");
                    //this.set_ivar("_groupId", ident as u64);
                    //let len: u64 = msg_send![ident, length];
                    //info!("set _groupId len: {}", len);

                    let ident_int: u64 = *this.get_ivar("_groupId");
                    let ident = ident_int as *mut Object;
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

                let f: extern fn(&mut Object, Sel, u64) = objc_popbar;
                decl.add_method(sel!(popbar:), f);

                let f: extern fn(&mut Object, Sel) -> u64 = objc_group_touch_bar;
                decl.add_method(sel!(groupTouchBar), f);

                let f: extern fn(&mut Object, Sel, u64) = objc_set_group_touch_bar;
                decl.add_method(sel!(setGroupTouchBar:), f);

                let f: extern fn(&mut Object, Sel, u64) = objc_set_group_ident;
                decl.add_method(sel!(setGroupIdent:), f);

                let f: extern fn(&mut Object, Sel, u64) = objc_set_icon;
                decl.add_method(sel!(setIcon:), f);

                let f: extern fn(&mut Object, Sel, u64) = objc_set_rust_wrapper;
                decl.add_method(sel!(setRustWrapper:), f);

                // Scrubber delegates
                let f: extern fn(&mut Object, Sel, u64) -> u32 = objc_number_of_items_for_scrubber;
                decl.add_method(sel!(numberOfItemsForScrubber:), f);
                let f: extern fn(&mut Object, Sel, u64, u32) -> u64 = objc_scrubber_view_for_item_at_index;
                decl.add_method(sel!(scrubber:viewForItemAtIndex:), f);
                let f: extern fn(&mut Object, Sel, u64, u64, u32) -> NSSize = objc_scrubber_layout_size_for_item_at_index;
                decl.add_method(sel!(scrubber:layout:sizeForItemAtIndex:), f);
                let f: extern fn(&mut Object, Sel, u64, u32) = objc_scrubber_did_select_item_at_index;
                decl.add_method(sel!(scrubber:didSelectItemAtIndex:), f);
            }

            decl.register();
        });

        Class::get("ObjcAppDelegate").unwrap()
    }
}
