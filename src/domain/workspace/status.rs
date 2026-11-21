//! The Data & Backup page's workspace status summary (docs/ui-design.md
//! §4.7 设置中的本地数据: 「数据页展示本地有效对象计数」).
//!
//! Every figure is a plain `COUNT(*)` over live (`trashed_at IS NULL`)
//! rows: no Document body, revision or Quick Note content is ever
//! loaded, so the summary stays cheap on large workspaces.

use super::{Result, Workspace};

/// Live counts of the workspace's top-level objects — the 本地工作区状态
/// card of the settings Data & Backup page. "Live" means outside the
/// Trash: a trashed object stops counting until it is restored, and a
/// permanently deleted one counts never again.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceStatus {
    pub knowledge_bases: usize,
    pub documents: usize,
    pub projects: usize,
    pub tasks: usize,
    pub quick_notes: usize,
}

impl Workspace {
    /// Counts the live Knowledge Bases, Documents, Projects, Tasks and
    /// Quick Notes in five `COUNT(*)` queries (see the module docs).
    pub fn workspace_status(&self) -> Result<WorkspaceStatus> {
        let count = |sql: &str| -> Result<usize> {
            self.conn
                .query_row(sql, [], |row| row.get::<_, i64>(0))
                .map(|n| n as usize)
                .map_err(Into::into)
        };
        Ok(WorkspaceStatus {
            knowledge_bases: count(
                "SELECT COUNT(*) FROM knowledge_bases WHERE trashed_at IS NULL",
            )?,
            documents: count("SELECT COUNT(*) FROM documents WHERE trashed_at IS NULL")?,
            projects: count("SELECT COUNT(*) FROM projects WHERE trashed_at IS NULL")?,
            tasks: count("SELECT COUNT(*) FROM tasks WHERE trashed_at IS NULL")?,
            quick_notes: count("SELECT COUNT(*) FROM quick_notes WHERE trashed_at IS NULL")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{FixedClock, LocalDateTime, NewTask};

    fn workspace() -> Workspace {
        let clock = FixedClock::at(LocalDateTime::parse("2026-11-21 10:00").unwrap());
        Workspace::open_in_memory_with_clock(clock).expect("workspace")
    }

    #[test]
    fn empty_workspace_counts_zero() {
        let ws = workspace();
        assert_eq!(
            ws.workspace_status().expect("status"),
            WorkspaceStatus::default()
        );
    }

    #[test]
    fn counts_live_objects_and_skips_trashed() {
        let mut ws = workspace();
        let book = ws.create_knowledge_base("产品手册").expect("kb");
        ws.create_document(book.id, None, "入门").expect("doc");
        ws.create_project("Kanon", "#ff4f4f", "🚀", None, None)
            .expect("project");
        ws.create_task(&NewTask {
            title: "任务".to_owned(),
            ..Default::default()
        })
        .expect("task");
        let note = ws.publish_quick_note("小记", &[]).expect("note");

        let status = ws.workspace_status().expect("status");
        assert_eq!(
            status,
            WorkspaceStatus {
                knowledge_bases: 1,
                documents: 1,
                projects: 1,
                tasks: 1,
                quick_notes: 1,
            }
        );

        // Trashing removes an object from the live counts (the book
        // takes its Documents along); restoring brings them back.
        ws.trash_knowledge_base(book.id).expect("trash kb");
        ws.trash_quick_note(note.id).expect("trash note");
        let status = ws.workspace_status().expect("status");
        assert_eq!(status.knowledge_bases, 0);
        assert_eq!(status.documents, 0);
        assert_eq!(status.quick_notes, 0);
        assert_eq!(status.projects, 1);
        assert_eq!(status.tasks, 1);

        ws.restore_knowledge_base(book.id).expect("restore kb");
        ws.restore_quick_note(note.id).expect("restore note");
        let status = ws.workspace_status().expect("status");
        assert_eq!(
            status,
            WorkspaceStatus {
                knowledge_bases: 1,
                documents: 1,
                projects: 1,
                tasks: 1,
                quick_notes: 1,
            }
        );
    }
}
