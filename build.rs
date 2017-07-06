use std::fs;
use std::path::Path;
fn main() {
    // Copy connectr.ini.in to connectr.ini if connectr.ini does not exist.
    //
    // The local changes in connectr.ini are always preserved, so you can
    // set private keys without worrying about git.
    let ini_file = Path::new("connectr.ini");
    if !ini_file.exists() {
        let _ = fs::copy("connectr.ini.in", "connectr.ini");
    }

    // Try again on re-build if either INI file has changed.
    println!("cargo:rerun-if-changed=connectr.ini");
    println!("cargo:rerun-if-changed=connectr.ini.in");

    // Mac touchbar support depends on a private framework to inject itself
    // into the persistent Control Strip area.  This adds it to the linker
    // include path.
    #[cfg(feature = "mac_touchbar")]
    println!("cargo:rustc-link-search=framework={}", "/System/Library/PrivateFrameworks");
}
