//! Recover a workspace without replacing either the live database or the backup.

use std::path::Path;
use std::process::Command;

use nsh_core::file_dialog::{FileFilter, OpenDialog, SaveDialog};
use nsh_core::prelude::*;

use crate::domain::{Workspace, WorkspaceError};

pub(super) fn restore(busy: Signal<bool>, notice: Signal<Option<(bool, String)>>) {
    busy.set(true);
    notice.set(None);
    spawn(async move {
        let status = restore_copy().await;
        notice.set(status);
        busy.set(false);
    });
}

async fn restore_copy() -> Option<(bool, String)> {
    let source = match pick_paths(
        OpenDialog::files()
            .title("选择 Kanon 工作区备份")
            .button_label("选择备份")
            .filter(backup_filter()),
    )
    .await
    {
        Ok(choice) => choice?.into_iter().next()?,
        Err(error) => {
            log::warn!("backup selection failed: {error}");
            return Some((false, "无法打开文件对话框。请重试。".to_owned()));
        }
    };
    let name = source
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("kanon");
    let destination = match pick_save_path(
        SaveDialog::new()
            .title("恢复到新的工作区文件")
            .button_label("恢复副本")
            .file_name(format!("{name}-恢复.sqlite3"))
            .filter(backup_filter()),
    )
    .await
    {
        Ok(choice) => choice?,
        Err(error) => {
            log::warn!("backup destination dialog failed: {error}");
            return Some((false, "无法打开保存对话框。请重试。".to_owned()));
        }
    };
    if let Err(error) = Workspace::restore_backup_copy(&source, &destination) {
        let message = match &error {
            WorkspaceError::Io(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                "文件已存在。请选择一个新文件名。"
            }
            WorkspaceError::Io(_) => "无法写入副本。请检查目标文件夹是否可写。",
            _ => "无法恢复。请选择有效且与当前版本兼容的 Kanon SQLite 备份。",
        };
        log::warn!("backup restore failed: {error}");
        return Some((false, message.to_owned()));
    }
    if let Err(error) = open_recovered_workspace(&destination) {
        log::warn!("opening recovered workspace failed: {error}");
        return Some((
            false,
            format!("副本已保存到 {}，但未能打开。", destination.display()),
        ));
    }
    Some((
        true,
        format!("已恢复到 {}，原工作区未更改。", destination.display()),
    ))
}

fn backup_filter() -> FileFilter {
    FileFilter::new("Kanon 工作区备份", ["sqlite3"])
}

fn open_recovered_workspace(path: &Path) -> std::io::Result<()> {
    let mut child = Command::new(std::env::current_exe()?)
        .env("KANON_WORKSPACE_PATH", path)
        .spawn()?;
    // On Unix collect the exit status without tying the new window's
    // lifetime to the current settings page or application.
    std::thread::spawn(move || {
        let _ = child.wait();
    });
    Ok(())
}
