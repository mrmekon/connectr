// Copied from rustc-objc-foundation project by SSheldon, examples/custom_class.rs
// https://github.com/SSheldon/rust-objc-foundation/blob/master/examples/custom_class.rs
// Covered by MIT License: https://en.wikipedia.org/wiki/MIT_License

#[macro_use]
extern crate objc;
extern crate objc_foundation;

use std::sync::{Once, ONCE_INIT};

use objc::Message;
use objc::declare::ClassDecl;
use objc::runtime::{Class, Object, Sel};
use objc_foundation::{INSObject, NSObject};

pub enum RustNSObject { }

impl RustNSObject {
    fn number(&self) -> u32 {
        unsafe {
            let obj = &*(self as *const _ as *const Object);
            *obj.get_ivar("_number")
        }
    }

    fn set_number(&mut self, number: u32) {
        unsafe {
            let obj =  &mut *(self as *mut _ as *mut Object);
            obj.set_ivar("_number", number);
        }
    }
}

unsafe impl Message for RustNSObject { }

static RUSTNSOBJECT_REGISTER_CLASS: Once = ONCE_INIT;

impl INSObject for RustNSObject {
    fn class() -> &'static Class {
        RUSTNSOBJECT_REGISTER_CLASS.call_once(|| {
            let superclass = NSObject::class();
            let mut decl = ClassDecl::new("RustNSObject", superclass).unwrap();
            decl.add_ivar::<u32>("_number");

            // Add ObjC methods for getting and setting the number
            extern fn my_object_set_number(this: &mut Object, _cmd: Sel, number: u32) {
                unsafe { this.set_ivar("_number", number); }
            }

            extern fn my_object_get_number(this: &Object, _cmd: Sel) -> u32 {
                unsafe { *this.get_ivar("_number") }
            }

            unsafe {
                let set_number: extern fn(&mut Object, Sel, u32) = my_object_set_number;
                decl.add_method(sel!(setNumber:), set_number);
                let get_number: extern fn(&Object, Sel) -> u32 = my_object_get_number;
                decl.add_method(sel!(number), get_number);
            }

            decl.register();
        });

        Class::get("RustNSObject").unwrap()
    }
}

fn main() {
    let mut obj = RustNSObject::new();

    obj.set_number(7);
    println!("Number: {}", unsafe {
        let number: u32 = msg_send![obj, number];
        number
    });

    unsafe {
        let _: () = msg_send![obj, setNumber:12u32];
    }
    println!("Number: {}", obj.number());
}
