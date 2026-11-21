//! Runs the Rust application directly on the machine's native backend.

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "macos")]
use nsh_platform_macos as platform;
#[cfg(target_os = "windows")]
use nsh_platform_windows as platform;

#[cfg(any(target_os = "macos", target_os = "windows"))]
fn main() -> impl std::process::Termination {
    env_logger::init();
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../nsh.json")).expect("valid nsh.json");
    let window = nsh_core::desktop::MainWindowOptions::from_json(&config["window"])
        .expect("valid window options in nsh.json");
    let capabilities = nsh_core::access::Capabilities::from_json(&config["capabilities"])
        .expect("valid capabilities in nsh.json");
    platform::run_app(
        platform::AppSource::Rust(
            platform::Root::new::<nsh_app_kanon::App>(nsh_app_kanon::AppProps::desktop)
                .tokens(
                    nsh_base::Theme::from_json(include_str!("../theme.json"))
                        .expect("nsh.json: theme")
                        .into_tokens(),
                )
                .capabilities(capabilities)
                .main_window(window),
        ),
        platform::AppOptions {
            title: "Kanon".to_owned(),
            app_id: "com.example.kanon".to_owned(),
            icon: Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
                if cfg!(target_os = "windows") {
                    "assets/kanon.ico"
                } else {
                    "assets/kanon.png"
                },
            )),
            ..Default::default()
        },
    )
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
compile_error!("Kanon's desktop binary supports only macOS and Windows.");
