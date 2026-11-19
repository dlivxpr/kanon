//! Runs the Rust application directly on the machine's native backend.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    env_logger::init();
    platform::run_app(
        platform::AppSource::Rust(platform::Root::of::<nsh_app_kanon::App>()),
        platform::AppOptions {
            title: "kanon".to_owned(),
            #[cfg(target_os = "windows")]
            app_id: "com.example.kanon".to_owned(),
            size: (480, 640),
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
compile_error!("Kanon's desktop binary supports only macOS and Windows.");
