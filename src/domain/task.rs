//! The Task aggregate and its business rules.

use super::date::{LocalDate, LocalDateTime, LocalTime};

/// The four task statuses (GLOSSARY: 待办 / 进行中 / 已完成 / 已取消).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskStatus {
    #[default]
    Todo,
    Doing,
    Done,
    Cancelled,
}

impl TaskStatus {
    pub fn label(self) -> &'static str {
        match self {
            Self::Todo => "待办",
            Self::Doing => "进行中",
            Self::Done => "已完成",
            Self::Cancelled => "已取消",
        }
    }

    /// Whether this status still owes work (overdue, Today, Inbox, …
    /// predicates only count these).
    pub fn is_active(self) -> bool {
        matches!(self, Self::Todo | Self::Doing)
    }

    /// The status a quick circle toggle lands on (spec: the circle toggles
    /// strictly between `done` and `todo`).
    pub fn circle_toggled(self) -> Self {
        match self {
            Self::Done => Self::Todo,
            Self::Todo | Self::Doing | Self::Cancelled => Self::Done,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Todo => "todo",
            Self::Doing => "doing",
            Self::Done => "done",
            Self::Cancelled => "cancel",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "todo" => Some(Self::Todo),
            "doing" => Some(Self::Doing),
            "done" => Some(Self::Done),
            "cancel" => Some(Self::Cancelled),
            _ => None,
        }
    }
}

/// Priority levels. `None` is the default (no priority).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Priority {
    #[default]
    None,
    Low,
    Medium,
    High,
}

impl Priority {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "无",
            Self::Low => "低",
            Self::Medium => "中",
            Self::High => "高",
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "none" => Some(Self::None),
            "low" => Some(Self::Low),
            "medium" => Some(Self::Medium),
            "high" => Some(Self::High),
            _ => None,
        }
    }
}

/// A subtask of a Task (spec: `task_subtasks`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Subtask {
    pub id: i64,
    pub task_id: i64,
    pub title: String,
    pub done: bool,
}

/// A cross-module tag (spec: `tags`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub id: i64,
    pub name: String,
    pub color: String,
}

/// The project a Task belongs to, as shown in task rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRef {
    pub id: i64,
    pub name: String,
    pub color: String,
}

/// A Task row with its independent Scheduled and Due date/times, stored as
/// floating local wall-clock values (no UTC offset conversion).
#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub status: TaskStatus,
    pub priority: Priority,
    pub memo: String,
    pub project_id: Option<i64>,
    pub milestone_id: Option<i64>,
    pub scheduled_date: Option<LocalDate>,
    pub scheduled_time: Option<LocalTime>,
    pub due_date: Option<LocalDate>,
    pub due_time: Option<LocalTime>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

/// What a Task creation must provide.
#[derive(Debug, Clone, Default)]
pub struct NewTask {
    pub title: String,
    pub status: Option<TaskStatus>,
    pub priority: Option<Priority>,
    pub project_id: Option<i64>,
    pub scheduled_date: Option<LocalDate>,
    pub scheduled_time: Option<LocalTime>,
    pub due_date: Option<LocalDate>,
    pub due_time: Option<LocalTime>,
}

/// A partial edit of a Task. `Some(Some(v))` sets, `Some(None)` clears,
/// `None` leaves the field untouched.
#[derive(Debug, Clone, Default)]
pub struct TaskPatch {
    pub title: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<Priority>,
    pub memo: Option<String>,
    pub project_id: Option<Option<i64>>,
    pub scheduled_date: Option<Option<LocalDate>>,
    pub scheduled_time: Option<Option<LocalTime>>,
    pub due_date: Option<Option<LocalDate>>,
    pub due_time: Option<Option<LocalTime>>,
}

/// A Task with the rows a view lists: tags, subtasks, and its project.
#[derive(Debug, Clone, PartialEq)]
pub struct TaskAggregate {
    pub task: Task,
    pub tags: Vec<Tag>,
    pub subtasks: Vec<Subtask>,
    pub project: Option<ProjectRef>,
}

impl Task {
    /// The overdue predicate (spec): an active task is overdue when its Due
    /// Date has passed in floating local time — a date-only Due Date is
    /// overdue from the next local midnight, a timed one from that local
    /// time.
    pub fn is_overdue_at(&self, now: LocalDateTime) -> bool {
        if !self.status.is_active() {
            return false;
        }
        match (self.due_date, self.due_time) {
            (Some(date), Some(time)) => now > LocalDateTime::new(date, time),
            (Some(date), None) => now.date > date,
            _ => false,
        }
    }

    /// The Inbox predicate (spec): an active, non-deleted task with no
    /// Project, no Scheduled Date and no Due Date.
    pub fn is_inbox_shape(&self) -> bool {
        self.status.is_active()
            && self.project_id.is_none()
            && self.scheduled_date.is_none()
            && self.due_date.is_none()
    }

    /// Whether Carryover moves this task: still active and scheduled
    /// strictly before `today`.
    pub fn is_carryover_candidate(&self, today: LocalDate) -> bool {
        self.status.is_active()
            && self.scheduled_date.is_some_and(|date| date < today)
    }

    /// Subtask progress as `done/total`.
    pub fn subtask_progress(subtasks: &[Subtask]) -> (usize, usize) {
        (subtasks.iter().filter(|s| s.done).count(), subtasks.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task_with_due(due_date: Option<LocalDate>, due_time: Option<LocalTime>) -> Task {
        Task {
            id: 1,
            title: "写周报".into(),
            status: TaskStatus::Todo,
            priority: Priority::None,
            memo: String::new(),
            project_id: None,
            milestone_id: None,
            scheduled_date: None,
            scheduled_time: None,
            due_date,
            due_time,
            created_at: String::new(),
            updated_at: String::new(),
            completed_at: None,
        }
    }

    #[test]
    fn date_only_due_date_is_overdue_from_next_local_midnight() {
        let task = task_with_due(LocalDate::parse("2026-11-20"), None);
        let same_day = LocalDateTime::parse("2026-11-20 23:59").unwrap();
        let next_midnight = LocalDateTime::parse("2026-11-21 00:00").unwrap();
        assert!(!task.is_overdue_at(same_day), "date-only due date is not overdue on its day");
        assert!(task.is_overdue_at(next_midnight), "date-only due date is overdue the next local midnight");
    }

    #[test]
    fn timed_due_date_is_overdue_after_local_cutoff() {
        let task = task_with_due(LocalDate::parse("2026-11-20"), LocalTime::parse("09:30"));
        let before = LocalDateTime::parse("2026-11-20 09:30").unwrap();
        let after = LocalDateTime::parse("2026-11-20 09:31").unwrap();
        assert!(!task.is_overdue_at(before), "exactly at the cutoff is not yet overdue");
        assert!(task.is_overdue_at(after), "one minute past the cutoff is overdue");
    }

    #[test]
    fn done_and_cancelled_are_never_overdue() {
        let mut task = task_with_due(LocalDate::parse("2026-11-20"), None);
        let later = LocalDateTime::parse("2026-11-25 12:00").unwrap();
        task.status = TaskStatus::Done;
        assert!(!task.is_overdue_at(later));
        task.status = TaskStatus::Cancelled;
        assert!(!task.is_overdue_at(later));
    }

    #[test]
    fn circle_toggle_is_strictly_done_todo() {
        assert_eq!(TaskStatus::Todo.circle_toggled(), TaskStatus::Done);
        assert_eq!(TaskStatus::Doing.circle_toggled(), TaskStatus::Done);
        assert_eq!(TaskStatus::Cancelled.circle_toggled(), TaskStatus::Done);
        assert_eq!(TaskStatus::Done.circle_toggled(), TaskStatus::Todo);
    }

    #[test]
    fn inbox_shape_requires_active_no_project_no_dates() {
        let task = task_with_due(None, None);
        assert!(task.is_inbox_shape());
        let mut scheduled = task.clone();
        scheduled.scheduled_date = LocalDate::parse("2026-11-20");
        assert!(!scheduled.is_inbox_shape());
        let mut project_bound = task.clone();
        project_bound.project_id = Some(7);
        assert!(!project_bound.is_inbox_shape());
        let mut due_only = task.clone();
        due_only.due_date = LocalDate::parse("2026-11-21");
        assert!(!due_only.is_inbox_shape());
        let mut done = task.clone();
        done.status = TaskStatus::Done;
        assert!(!done.is_inbox_shape());
    }

    #[test]
    fn carryover_candidate_is_active_and_scheduled_before_today() {
        let today = LocalDate::parse("2026-11-20").unwrap();
        let mut task = task_with_due(None, None);
        assert!(!task.is_carryover_candidate(today), "unscheduled tasks are not carried over");
        task.scheduled_date = LocalDate::parse("2026-11-20");
        assert!(!task.is_carryover_candidate(today), "today's tasks stay");
        task.scheduled_date = LocalDate::parse("2026-11-19");
        assert!(task.is_carryover_candidate(today));
        task.status = TaskStatus::Done;
        assert!(!task.is_carryover_candidate(today), "done tasks are not carried over");
    }
}
