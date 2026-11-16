# Kanon

Kanon is a personal project-driven workspace that unifies structured knowledge management (Yuque-style books and documents) with daily action execution (Woday-style tasks, habits, events, and inbox capture).

## Language

### Containers & Organization

**Project**:
A goal-oriented, time-bounded container with a lifecycle status (e.g., planning, active, completed, archived) that groups both actionable work (`Task`, `Event`) and structured knowledge (`Document` catalog) to drive a personal objective forward.
_Avoid_: Folder, workspace, team, repo

**Book**:
An open-ended, non-expiring knowledge base organized around a long-term topic or domain, structuring `Document` items into a book-like table of contents without project lifecycle constraints.
_Avoid_: Folder, wiki, notebook, repository

**Catalog**:
The ordered, hierarchical tree of section groups and `Document` nodes inside a `Project` or `Book` that gives knowledge a book-like reading order and structure.
_Avoid_: Folder tree, directory, file list

**Tag**:
A color-coded, cross-cutting label that can be attached to `Task`, `Memo`, `Document`, or `Event` items to aggregate related work across different `Project` and `Book` containers.
_Avoid_: Category, folder, topic

### Capture & Knowledge

**Inbox**:
A persistent capture and triage queue that collects unorganized `Task` items and fleeting `Memo` notes before they are scheduled onto the calendar or filed into a `Project` or `Book`.
_Avoid_: Backlog, scratchpad

**Memo**:
A lightweight, unstructured fleeting note captured in the `Inbox` (similar to Yuque "小记") without requiring an upfront `Project` or `Book` location; can later be promoted into a formal `Document` or converted into `Task` items.
_Avoid_: Quick note, sticky note, snippet

**Document**:
A structured long-form note belonging to either a `Project` or a `Book`, composed of rich blocks and positioned within a hierarchical `Catalog`, with separate reading and editing modes.
_Avoid_: Page, article, post

**Backlink**:
An automatic reverse reference recorded when one `Document`, `Task`, or `Event` mentions another via `@`, surfaced alongside the document outline to reveal knowledge and execution connections.
_Avoid_: Citation, trackback

### Execution & Schedule

**Task**:
An actionable to-do item with completion state, optional due date/time, priority, subtasks, and optional assignment to a `Project`, surfaced in global schedule views (`Today`, `Upcoming`, `Tasks`) and optionally embedded in a `Document`.
_Avoid_: Todo, issue, ticket, checklist

**Checklist**:
A lightweight, document-local checkable line item inside a `Document` or `Memo` that does not appear in global `Task` views unless explicitly promoted into a `Task`.
_Avoid_: Todo, global task

**Habit**:
A recurring daily or scheduled practice tracked for consistency and completion streaks across days.
_Avoid_: Routine, check-in

**Event**:
A calendar-bound appointment or time block occupying a specific date and time range on the schedule.
_Avoid_: Meeting, reminder
