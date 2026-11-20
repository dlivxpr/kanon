//! Kanon: a local-first knowledge base, tasks and projects desktop app.
//!
//! `src/lib.rs` keeps only the module lines, the `#[native_module]` export
//! and the root component (AGENTS.md); the domain lives in `domain/`, the
//! styled control layer in `ui/`, and the workbench shell in `shell/`.

pub mod domain;
mod shell;
mod today;
mod ui;

use std::path::Path;
use std::sync::Arc;

use nsh_core::prelude::*;

use domain::{Clock, FixedClock, LocalDateTime, SystemClock, Workspace};

/// How the root learns where its workspace lives and what time it is.
///
/// `Default` (what `open(".")` in `*.test.tsx` and `Root::of::<App>()`
/// use) opens an ephemeral in-memory workspace on the system clock; the
/// desktop launcher and the Rust headless tests pin a real file path and,
/// for deterministic tests, a fixed local datetime.
#[derive(Clone, PartialEq, Props)]
pub struct AppProps {
    /// The workspace SQLite file, or `None` for in-memory.
    pub workspace_path: Prop<Option<String>>,
    /// A fixed local datetime `YYYY-MM-DD HH:MM` (deterministic tests),
    /// or `None` for the system clock.
    pub fixed_now: Prop<Option<String>>,
}

impl AppProps {
    /// The props the desktop binary launches with: a file in the user's
    /// home (overridable with `KANON_WORKSPACE_PATH`), system clock.
    pub fn desktop() -> Self {
        Self {
            workspace_path: std::env::var("KANON_WORKSPACE_PATH")
                .ok()
                .or_else(default_workspace_path)
                .into(),
            fixed_now: None.into(),
        }
    }
}

fn default_workspace_path() -> Option<String> {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(std::path::PathBuf::from)?;
    Some(home.join(".kanon").join("workspace.sqlite3").to_string_lossy().into_owned())
}

/// The NSH UI set that registers the application's Rust root.
#[native_module("kanon/native")]
fn native(registry: &mut Registry) {
    registry.register::<App>();
}

/// The application's root component: the multi-tab workbench shell.
#[component]
pub fn App(props: AppProps) -> impl IntoElement {
    let clock: Arc<dyn Clock> = match props.fixed_now.get_untracked() {
        Some(text) => match LocalDateTime::parse(&text) {
            Some(fixed) => FixedClock::at(fixed),
            None => {
                return error_panel(&format!(
                    "Kanon: invalid fixed_now {text:?}, want \"YYYY-MM-DD HH:MM\""
                ));
            }
        },
        None => Arc::new(SystemClock),
    };
    let workspace = match props.workspace_path.get_untracked() {
        Some(path) => Workspace::open(Path::new(&path), clock),
        None => Workspace::open_in_memory_with_clock(clock),
    };
    match workspace {
        Ok(workspace) => {
            let store = shell::Store::new(workspace);
            provide_context(store);
            shell::workbench(store)
        }
        Err(err) => error_panel(&format!("Kanon: cannot open the workspace: {err}")),
    }
}

fn error_panel(message: &str) -> View {
    let message = message.to_owned();
    view()
        .flex_1()
        .items_center()
        .justify_center()
        .bg(move || nsh_base::theme().background)
        .child(text(message).text_color(move || nsh_base::theme().destructive))
}
