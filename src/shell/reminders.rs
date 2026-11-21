//! Reminders while the workspace is open. The OS owns permission and delivery.
use std::time::Duration;

use nsh_core::{notifications::Notification, prelude::*};

use crate::domain::{LocalDateTime, LocalTime};

use super::Store;

pub(super) const PREF_DIGEST: &str = "daily_digest";
pub(super) const PREF_DIGEST_TIME: &str = "digest_time";
pub(super) const PREF_DUE_REMINDER: &str = "due_reminder";
pub(super) const DEFAULT_DIGEST_TIME: LocalTime = LocalTime::new(9, 0);

pub(super) fn watch(store: Store) {
    let digest = computed(move || {
        (store.preference(PREF_DIGEST).as_deref() == Some("true")).then(|| {
            store
                .preference(PREF_DIGEST_TIME)
                .as_deref()
                .and_then(LocalTime::parse)
                .unwrap_or(DEFAULT_DIGEST_TIME)
        })
    });
    let lead = computed(
        move || match store.preference(PREF_DUE_REMINDER).as_deref() {
            Some("0m") => Some(0),
            Some("15m") => Some(15),
            Some("30m") => Some(30),
            Some("60m") => Some(60),
            _ => None,
        },
    );
    let last = stored(None);
    set_interval(Duration::from_secs(10), move || {
        let (digest, lead) = (digest.get_untracked(), lead.get_untracked());
        if digest.is_none() && lead.is_none() {
            return;
        }
        let now = store.now();
        if last.replace(Some(now)) == Some(now) {
            return;
        }
        let digest_now = digest == Some(now.time);
        let due_at = lead.map(|lead| due_at_reminder(now, lead));
        if !digest_now && due_at.is_none() {
            return;
        }
        spawn(async move {
            use nsh_core::notifications::NotificationPermission;
            if notification_permission().await != NotificationPermission::Granted {
                return;
            }
            let tasks = store.live_tasks();
            if digest_now {
                let count = tasks
                    .iter()
                    .filter(|task| {
                        let task = &task.task;
                        task.status.is_active()
                            && (task.scheduled_date == Some(now.date)
                                || task.due_date == Some(now.date))
                    })
                    .count();
                let milestones = store
                    .projects()
                    .into_iter()
                    .map(|project| {
                        store
                            .milestones(project.id)
                            .into_iter()
                            .filter(|entry| entry.target_date == Some(now.date))
                            .count()
                    })
                    .sum::<usize>();
                let _ = show_notification(Notification::new(
                    format!("kanon-digest-{}", now.date),
                    "晨间概览",
                    format!("今天有 {count} 项任务、{milestones} 个项目里程碑。"),
                ))
                .await;
            }
            if let Some(due) = due_at {
                for task in tasks.into_iter().map(|task| task.task).filter(|task| {
                    task.status.is_active()
                        && task.due_date == Some(due.date)
                        && task.due_time == Some(due.time)
                }) {
                    let _ = show_notification(Notification::new(
                        format!("kanon-due-{}-{}", task.id, due.date),
                        "任务到期提醒",
                        format!("{} · 截止 {}", task.title, due),
                    ))
                    .await;
                }
            }
        });
    });
}

fn due_at_reminder(now: LocalDateTime, lead: u16) -> LocalDateTime {
    let minutes = u16::from(now.time.hour) * 60 + u16::from(now.time.minute) + lead;
    LocalDateTime::new(
        now.date.add_days(i64::from(minutes / 1440)),
        LocalTime::new((minutes / 60 % 24) as u8, (minutes % 60) as u8),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_reminder_can_precede_a_due_time_on_the_next_day() {
        let now = LocalDateTime::parse("2026-12-31 23:45").unwrap();
        assert_eq!(
            due_at_reminder(now, 30),
            LocalDateTime::parse("2027-01-01 00:15").unwrap()
        );
    }
}
