//! Build script.
//!
//! Windows: the app manifest (Common Controls v6, which the file dialog needs) is embedded
//! through the linker instead of Tauri's resource file. The resource only reaches the app
//! binary, so the unit-test binary of the library (it links the dialog plugin) failed to start
//! with `STATUS_ENTRYPOINT_NOT_FOUND`. The linker arguments reach every binary, tests included.
//! The manifest file is Tauri's own default, kept here unchanged.

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").is_ok_and(|os| os == "windows") {
        println!("cargo:rerun-if-changed=windows-manifest.xml");
        let manifest =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("windows-manifest.xml");
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
        let windows = tauri_build::WindowsAttributes::new_without_app_manifest();
        if tauri_build::try_build(tauri_build::Attributes::new().windows_attributes(windows))
            .is_err()
        {
            std::process::exit(1);
        }
    } else {
        tauri_build::build();
    }
}
