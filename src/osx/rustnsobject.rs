// Copied from rustc-objc-foundation project by SSheldon, examples/custom_class.rs
// https://github.com/SSheldon/rust-objc-foundation/blob/master/examples/custom_class.rs
// Covered by MIT License: https://en.wikipedia.org/wiki/MIT_License

extern crate objc;
extern crate objc_foundation;
extern crate objc_id;

pub use ::NSCallback;

use std::sync::Once;

use objc::Message;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use self::objc_foundation::{INSObject, NSObject};

use std::collections::BTreeMap;

use self::objc_id::Id;
use self::objc_id::WeakId;
use self::objc_id::Shared;

use std::sync::mpsc::Sender;

pub struct RustWrapperClass {
    pub objc: Id<ObjcSubclass, Shared>,
    pub cb_fn: Option<Box<dyn Fn(&mut RustWrapperClass, u64)>>,
    pub map: BTreeMap<u64, NSCallback>,
    pub tx: Sender<String>,
}

pub type NSObj = Box<RustWrapperClass>;
pub type NSObjc = Id<ObjcSubclass, Shared>;

pub trait NSObjCallbackTrait {
    fn set_value(&mut self, u64, NSCallback);
    fn get_value(&self, u64) -> &NSCallback;
}

impl NSObjCallbackTrait for RustWrapperClass {
    fn set_value(&mut self, key: u64, val: NSCallback) {
        self.map.insert(key, val);
    }
    fn get_value(&self, key: u64) -> &NSCallback {
        self.map.get(&key).unwrap()
    }
}

pub trait NSObjTrait {
    fn alloc(tx: Sender<String>) -> NSObj;
    fn selector(&self) -> Sel;
    fn take_objc(&mut self) -> NSObjc;
    fn add_callback(&mut self, *const Object, NSCallback);
}

impl NSObjTrait for NSObj {
    fn add_callback(&mut self, item: *const Object, cb: NSCallback) {
        let sender: u64 = item as u64;
        self.set_value(sender, cb);
    }
    fn alloc(tx: Sender<String>) -> NSObj {
        let objc = ObjcSubclass::new().share();
        let rust = Box::new(RustWrapperClass {
            objc: objc,
            map: BTreeMap::<u64,NSCallback>::new(),
            cb_fn: None,
            tx: tx,
        });
        unsafe {
            let ptr: u64 = &*rust as *const RustWrapperClass as u64;
            let _:() = msg_send![rust.objc, setRustData: ptr];
        }
        return rust
    }
    fn selector(&self) -> Sel {
        sel!(cb:)
    }
    fn take_objc(&mut self) -> NSObjc {
        let weak = WeakId::new(&self.objc);
        weak.load().unwrap()
    }
}

impl NSObjCallbackTrait for NSObj {
    fn set_value(&mut self, key: u64, val: NSCallback) {
        self.map.insert(key, val);
    }
    fn get_value(&self, key: u64) -> &NSCallback {
        self.map.get(&key).unwrap()
    }
}

// ObjcSubclass is a subclass of the objective-c NSObject base class.
// This is registered with the objc runtime, so instances of this class
// are "owned" by objc, and have no associated Rust data.
//
// This can be wrapped with a RustWrapperClass, which is a proper Rust struct
// with its own storage, and holds an instance of ObjcSubclass.
//
// An ObjcSubclass can "talk" to its Rust wrapper class through function
// pointers, as long as the storage is on the heap with a Box and the underlying
// memory address doesn't change.  The NSObj type wraps RustWrapperClass up
// in a Box.  The functions in the NSObjTrait trait operate on the boxed struct,
// while keeping its storage location on the heap persistent.
//
pub enum ObjcSubclass {}
impl ObjcSubclass {}

unsafe impl Message for ObjcSubclass { }

static OBJC_SUBCLASS_REGISTER_CLASS: Once = Once::new();

impl INSObject for ObjcSubclass {
    fn class() -> &'static Class {
        OBJC_SUBCLASS_REGISTER_CLASS.call_once(|| {
            let superclass = NSObject::class();
            let mut decl = ClassDecl::new("ConnectrObjcSubclass", superclass).expect("Failed to create custom ObjC class.");
            decl.add_ivar::<u64>("_rustdata");

            extern fn objc_cb(this: &mut Object, _cmd: Sel, sender: u64) {
                unsafe {
                    let ptr: u64 = *this.get_ivar("_rustdata");
                    let rustdata: &mut RustWrapperClass = &mut *(ptr as *mut RustWrapperClass);
                    if let Some(ref cb) = rustdata.cb_fn {
                        // Ownership?  Fuck ownership!
                        let rustdata: &mut RustWrapperClass = &mut *(ptr as *mut RustWrapperClass);
                        cb(rustdata, sender);
                    }
                }
            }
            extern fn objc_set_rust_data(this: &mut Object, _cmd: Sel, ptr: u64) {
                unsafe {this.set_ivar("_rustdata", ptr);}
            }
            extern fn objc_get_rust_data(this: &Object, _cmd: Sel) -> u64 {
                unsafe {*this.get_ivar("_rustdata")}
            }

            extern fn objc_url(_this: &Object, _cmd: Sel, _event: u64, _reply: u64) {
                info!("connectr URL support not implemented yet.");
            }

            unsafe {
                let f: extern fn(&mut Object, Sel, u64) = objc_cb;
                decl.add_method(sel!(cb:), f);
                let f: extern fn(&mut Object, Sel, u64) = objc_set_rust_data;
                decl.add_method(sel!(setRustData:), f);
                let f: extern fn(&Object, Sel) -> u64 = objc_get_rust_data;
                decl.add_method(sel!(rustData), f);
                let f: extern fn(&Object, Sel, u64, u64) = objc_url;
                decl.add_method(sel!(handleURLEvent:withReplyEvent:), f);
            }

            decl.register();
        });

        Class::get("ConnectrObjcSubclass").unwrap()
    }
}
