# kanon

Kanon is a pure-Rust [Native Shell 0.6.1](https://github.com/longbridge/nsh/releases/tag/v0.6.1) app for Windows and macOS. `nsh.json` points to the Rust root, `App` in `src/lib.rs`.

```sh
nsh run                      # native backend on Windows or macOS
nsh check                    # cargo check of the crate, then type checks App.test.tsx
nsh test                     # cargo test, then App.test.tsx: the root tested from TypeScript, headless; --target to run it on a platform
nsh test nsh.json --trace    # headless: what the root mounts, and the time of each commit
nsh package --target windows # use --target macos on a Mac
```

`nsh run` and `nsh package` mount the Rust root without bundling QuickJS. Rust edits rebuild and restart the app.

```sh
cargo run
```

This starts the root directly on Win32 or AppKit.

The `nsh` commands build the app's runners from the NSH crates `Cargo.toml` depends on: crates.io's at the version of
the `nsh` you run (keep the `nsh-*` dependencies at that version when you upgrade `nsh`), or an NSH checkout's by path.

The native backends opt in to `svg` for the window and navigation icons; their strokes use `currentColor` from the app theme. Keep the system caption controls and use the NSH title-bar insets instead of drawing replacement controls.

The application icon source is `assets/kanon.svg`. NSH packages `assets/kanon.png` for both desktops; the direct Windows launcher uses the multi-resolution `assets/kanon.ico`.

Download desktop packages from [GitHub Releases](https://github.com/dlivxpr/kanon/releases). The **Package desktop** workflow builds the Windows x64 installer and portable ZIP, and the macOS universal ZIP; macOS builds are ad hoc signed, not notarized.
Earlier development packages used `com.example.kanon` and keep their own workspace. Export a backup before switching to the release build. If you installed the Windows preview, uninstall it after backing up, then install the release build. Use **恢复到新副本** to open the backup without overwriting the release workspace.

Use **设置 → 数据与备份 → 导出备份** to save a complete SQLite snapshot to a new file. **恢复到新副本** opens a separate Kanon window backed by a new file; it never replaces the active workspace or modifies the snapshot. For command-line recovery, launch with `KANON_WORKSPACE_PATH` set to a copy of the snapshot. Keep the original backup separately.

NSH-generated files stay in `.nsh/`: runner sources in `.nsh/rust/` and default packages in `.nsh/out/`.
Cargo keeps its normal `target/` directory and respects your existing `CARGO_TARGET_DIR`.

We recommend building with [huacnlee/cargo](https://github.com/huacnlee/cargo), the Cargo NSH is developed with: a
drop-in fork that builds each dependency once for all your projects and Git worktrees and keeps its build directory
under a size limit on its own, with nothing to configure. It builds under `~/.cargo/targets/` and still puts the
binaries in `target/`; it replaces `~/.cargo/bin/cargo`, and its README says how to put rustup's back.

```sh
curl -fsSL https://huacnlee.github.io/cargo/install.sh | sh     # Windows: irm https://huacnlee.github.io/cargo/install.ps1 | iex
```
