//! Document content rules that stay pure Rust (no UI crates, no SQLite):
//! word counting for the counter under a Document, the heading outline
//! that drives reading-mode navigation, and the auto-history snapshot
//! policy. The `Workspace` facade builds on these.

/// One outline entry: a heading a reader can jump to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlineEntry {
    /// The heading level (`1..=6`, ATX `#` count).
    pub level: u8,
    /// The heading text, markup stripped.
    pub text: String,
    /// Which heading this is among the body's headings, in document
    /// order (the screen maps it to the editor's nth heading node).
    pub ordinal: usize,
}

/// The word count shown under a Document. CJK characters each count as
/// one word (they carry no spaces); everything else counts by
/// whitespace-separated runs, so `hello world` is 2 and `你好 world` is 3.
/// Markdown syntax never counts: list markers (`-`, `1.`) and checkbox
/// markers (`[ ]`, `[x]`) are stripped outside fenced code blocks (inside
/// them every character is code and list-looking text counts as code).
pub fn count_words(markdown: &str) -> u64 {
    /// CJK ideographs (and kana/hangul syllables) count one per character.
    fn cjk(c: char) -> bool {
        matches!(c,
            '\u{4e00}'..='\u{9fff}'
            | '\u{3400}'..='\u{4dbf}'
            | '\u{f900}'..='\u{faff}'
            | '\u{3040}'..='\u{30ff}'
            | '\u{ac00}'..='\u{d7af}')
    }
    let mut words = 0_u64;
    let mut latin_run = false;
    let mut fence = false;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") || line.trim_start().starts_with("~~~") {
            fence = !fence;
            continue;
        }
        let text = if fence { line } else { strip_list_marker(line) };
        for ch in text.chars() {
            if cjk(ch) {
                words += 1;
                latin_run = false;
            } else if ch.is_alphanumeric() && !ch.is_whitespace() && !cjk(ch) {
                // Latin letters and digits count once per run.
                latin_run = true;
            } else if latin_run {
                words += 1;
                latin_run = false;
            }
        }
        if latin_run {
            words += 1;
            latin_run = false;
        }
    }
    words
}

/// A line without its leading list marker (`-`, `*`, `+`, `12.`) and
/// checkbox marker (`[ ]`, `[x]`, `[X]`): neither is a word.
fn strip_list_marker(line: &str) -> &str {
    let rest = line.trim_start();
    let after_list = if let Some(stripped) = rest.strip_prefix(['-', '*', '+']) {
        stripped.trim_start()
    } else {
        let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
        match rest[digits..].strip_prefix('.') {
            Some(after_dot) if digits > 0 => after_dot.trim_start(),
            _ => rest,
        }
    };
    for marker in ["[ ] ", "[x] ", "[X] "] {
        if let Some(stripped) = after_list.strip_prefix(marker) {
            return stripped;
        }
    }
    after_list
}

/// The heading outline of a Document body (ATX headings `#`..`######`),
/// in document order. Headings inside fenced code blocks are not outline
/// entries. `max_level` bounds what navigation shows (the design shows
/// H1–H3 in the right panel).
pub fn outline(markdown: &str, max_level: u8) -> Vec<OutlineEntry> {
    let mut entries = Vec::new();
    let mut fence: Option<char> = None;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if fence.is_some() {
            if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
                fence = None;
            }
            continue;
        }
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            fence = Some(trimmed.chars().next().unwrap_or('`'));
            continue;
        }
        let hashes = trimmed.chars().take_while(|&c| c == '#').count();
        if hashes == 0 || hashes > 6 || trimmed.len() == hashes {
            continue;
        }
        let rest = &trimmed[hashes..];
        if !rest.starts_with(' ') {
            continue;
        }
        let level = hashes as u8;
        if level > max_level {
            continue;
        }
        let text = rest.trim().trim_end_matches('#').trim().to_owned();
        if text.is_empty() {
            continue;
        }
        let ordinal = entries.len();
        entries.push(OutlineEntry { level, text, ordinal });
    }
    entries
}

/// Whether an automatic history snapshot is due. Editing records one when
/// the document stayed dirty for the idle window, or when the interval
/// elapsed since the last snapshot while edits keep coming (a busy editor
/// never starves history, a quiet one snapshots once and rests).
pub fn should_snapshot(
    last_edit_at_ms: u64,
    last_snapshot_at_ms: Option<u64>,
    now_ms: u64,
    idle_after_ms: u64,
    interval_ms: u64,
) -> bool {
    let dirty_since = match last_snapshot_at_ms {
        Some(at) => last_edit_at_ms.max(at),
        None => last_edit_at_ms,
    };
    if now_ms < dirty_since {
        return false;
    }
    if now_ms - last_edit_at_ms >= idle_after_ms && last_edit_at_ms > last_snapshot_at_ms.unwrap_or(0) {
        return true;
    }
    match last_snapshot_at_ms {
        Some(at) => now_ms - at >= interval_ms && last_edit_at_ms > at,
        None => false,
    }
}

/// The Kanon archive markers: editor-only constructs — Linked Task Cards
/// and inline `@` mentions — survive the markdown body as text (ADR-0002).
/// The editor swaps them for interactive nodes; the domain scans the
/// committed body to keep `document_references` indexed.
///
/// A Linked Task Card paragraph in the archive body (`@task(3)`).
pub const TASK_CARD_PLACEHOLDER_MARKER: &str = "@task(0)";

/// The archived form of a Linked Task Card referencing `task_id`. The
/// paren form survives the markdown serializer without escapes (a
/// bracket form would come back `@task\[3\]`).
pub fn task_card_marker(task_id: i64) -> String {
    format!("@task({task_id})")
}

/// The archived form of an inline `@` mention. `id_part` is the target id
/// (`3`) or, for dates, the date text itself (`2026-11-20`). The label
/// sanitizes exactly the characters the parser rejects (a `)` would close
/// the marker early; `{`, `}` and `@` would corrupt re-parsing): they are
/// full-widthed instead, so every label round-trips losslessly.
pub fn mention_marker(kind: &str, id_part: &str, label: &str) -> String {
    let label: String = label
        .chars()
        .map(|c| match c {
            ')' => '）',
            '{' => '｛',
            '}' => '｝',
            '@' => '＠',
            c => c,
        })
        .collect();
    format!("@m{{{kind}:{id_part}}}({label})")
}

/// One inline mention marker found in a body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MentionMarker {
    /// `document` | `project` | `task` | `date`.
    pub kind: String,
    /// The raw id part inside the braces (a number, or the date text).
    pub id_part: String,
    /// `Some(id)` for document/project/task targets; `None` for dates.
    pub target_id: Option<i64>,
    /// The display label.
    pub label: String,
    /// Where in the body the marker anchors, in UTF-16 code units.
    pub position: usize,
}

/// Parses `text` as one whole Linked Task Card marker (`@task(3)`,
/// surrounding whitespace allowed).
pub fn parse_task_card_marker(text: &str) -> Option<i64> {
    let inner = text.trim().strip_prefix("@task(")?.strip_suffix(')')?;
    inner.parse::<i64>().ok()
}

/// Parses `text` as an inline mention marker (`@m{task:3}(写周报)`)
/// starting at its first byte. Returns the parsed marker and the marker's
/// byte length.
pub fn parse_mention_marker(text: &str) -> Option<(MentionMarker, usize)> {
    let rest = text.strip_prefix("@m{")?;
    let close = rest.find('}')?;
    let inner = &rest[..close];
    let (kind, id_part) = inner.split_once(':')?;
    if kind.is_empty() || id_part.is_empty() || id_part.contains(['{', '}', '(', ')']) {
        return None;
    }
    let after = &rest[close + 1..];
    let label = after.strip_prefix('(')?;
    let label_end = label.find(')')?;
    let label = &label[..label_end];
    if label.contains(['{', '}', '@']) {
        return None;
    }
    let target_id = if kind == "date" { None } else { id_part.parse::<i64>().ok() };
    let byte_len = 3 + close + 1 + 1 + label_end + 1;
    Some((
        MentionMarker {
            kind: kind.to_owned(),
            id_part: id_part.to_owned(),
            target_id,
            label: label.to_owned(),
            position: 0,
        },
        byte_len,
    ))
}

/// Every Linked Task Card marker in `body`, in document order, with the
/// UTF-16 offset of each.
pub fn scan_task_card_markers(body: &str) -> Vec<(i64, usize)> {
    let mut found = Vec::new();
    let mut utf16_at = 0_usize;
    let mut from = 0_usize;
    while let Some(offset) = body[from..].find("@task(") {
        let start = from + offset;
        utf16_at += body[from..start].chars().map(char::len_utf16).sum::<usize>();
        let digits = body[start + 6..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .count();
        let end = start + 6 + digits;
        let closed = body[end..].starts_with(')');
        if closed && digits > 0 {
            let id = body[start + 6..end].parse::<i64>().expect("digits");
            found.push((id, utf16_at));
        }
        let step = if closed { digits + 1 } else { 6 };
        utf16_at += body[start..start + step].chars().map(char::len_utf16).sum::<usize>();
        from = start + step;
    }
    found
}

/// Every inline mention marker in `body`, in document order, with the
/// UTF-16 offset of each.
pub fn scan_mention_markers(body: &str) -> Vec<MentionMarker> {
    let mut found = Vec::new();
    let mut utf16_at = 0_usize;
    let mut from = 0_usize;
    while let Some(offset) = body[from..].find("@m{") {
        let start = from + offset;
        utf16_at += body[from..start].chars().map(char::len_utf16).sum::<usize>();
        match parse_mention_marker(&body[start..]) {
            Some((mut marker, byte_len)) => {
                marker.position = utf16_at;
                utf16_at += body[start..start + byte_len].chars().map(char::len_utf16).sum::<usize>();
                from = start + byte_len;
                found.push(marker);
            }
            None => {
                utf16_at += '@'.len_utf16();
                from = start + 1;
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_cjk_per_character_and_latin_per_word() {
        assert_eq!(count_words("你好，世界"), 4);
        assert_eq!(count_words("hello world"), 2);
        assert_eq!(count_words("你好 world"), 3);
        assert_eq!(count_words(""), 0);
        assert_eq!(count_words("# 标题\n- [ ] 清单项"), 5);
        assert_eq!(count_words("123 456"), 2);
    }

    #[test]
    fn outline_skips_fenced_code_and_deep_headings() {
        let body = "# 一\n\n```\n# not a heading\n```\n\n## 二\n\n#### 四\n\n### 三 ###\n\nnot # one\n";
        let entries = outline(body, 3);
        let texts = entries.iter().map(|e| (e.level, e.text.as_str())).collect::<Vec<_>>();
        assert_eq!(
            texts,
            vec![(1, "一"), (2, "二"), (3, "三")],
            "code fences and H4 are left out, closing hashes stripped"
        );
        assert_eq!(entries[0].ordinal, 0);
        assert_eq!(entries[2].ordinal, 2, "ordinals follow document order");
    }

    #[test]
    fn snapshot_policy_never_spams_and_never_starves() {
        // Idle window reached with edits after the last snapshot.
        assert!(should_snapshot(1_000, Some(0), 10_000 + 1_000, 10_000, 60_000));
        // Not idle yet, interval not reached.
        assert!(!should_snapshot(1_000, Some(0), 5_000, 10_000, 60_000));
        // Continuous typing: the interval forces a checkpoint.
        assert!(should_snapshot(59_000, Some(0), 60_000, 360_000, 60_000));
        // No edits since the last snapshot: no duplicate snapshot.
        assert!(!should_snapshot(1_000, Some(1_000), 120_000, 10_000, 60_000));
        // No snapshot yet: the idle window alone triggers it.
        assert!(should_snapshot(1_000, None, 11_000, 10_000, 60_000));
        // Clock skew never snapshots.
        assert!(!should_snapshot(5_000, None, 4_000, 10_000, 60_000));
    }

    #[test]
    fn task_card_markers_round_trip_and_scan_with_utf16_positions() {
        assert_eq!(parse_task_card_marker("@task(3)"), Some(3));
        assert_eq!(parse_task_card_marker("  @task(42)  "), Some(42));
        assert_eq!(parse_task_card_marker("@task(x)"), None);
        assert_eq!(parse_task_card_marker("@task(3) trailing"), None);

        let body = "前文\n\n@task(7)\n\n@task(8)";
        assert_eq!(scan_task_card_markers(body), vec![(7, 4), (8, 14)]);
        // CJK before the marker shifts the UTF-16 offset correctly.
        assert_eq!(scan_task_card_markers("清单@task(5)"), vec![(5, 2)]);
    }

    #[test]
    fn mention_markers_round_trip_and_scan_with_utf16_positions() {
        let marker = mention_marker("task", "3", "写周报");
        assert_eq!(marker, "@m{task:3}(写周报)");
        let (parsed, len) = parse_mention_marker(&marker).expect("parses");
        assert_eq!(parsed.kind, "task");
        assert_eq!(parsed.id_part, "3");
        assert_eq!(parsed.target_id, Some(3));
        assert_eq!(parsed.label, "写周报");
        assert_eq!(len, marker.len());

        let date = mention_marker("date", "2026-11-20", "2026-11-20");
        let (parsed, _) = parse_mention_marker(&date).expect("date parses");
        assert_eq!(parsed.kind, "date");
        assert_eq!(parsed.target_id, None, "dates carry no id");

        // A `)` in the label is full-widthed so the marker stays parseable.
        let nasty = mention_marker("document", "9", "a)b");
        let (parsed, _) = parse_mention_marker(&nasty).expect("sanitized label parses");
        assert_eq!(parsed.label, "a）b");

        let body = "前文@m{document:7}(手册)后文@m{task:3}(写周报)";
        let markers = scan_mention_markers(body);
        assert_eq!(markers.len(), 2);
        assert_eq!(markers[0].kind, "document");
        assert_eq!(markers[0].target_id, Some(7));
        assert_eq!(markers[0].label, "手册");
        assert_eq!(markers[0].position, 2);
        assert_eq!(markers[1].kind, "task");
        assert_eq!(markers[1].position, 2 + 18 + 2, "positions accumulate in UTF-16 units");
    }

    #[test]
    fn scans_ignore_almost_markers() {
        assert!(scan_mention_markers("邮箱 a@b.com 与 @m{broken 文本").is_empty());
        assert!(scan_task_card_markers("@task() @task(x) @task(1").is_empty());
        // A lone `@` advances the search without looping.
        assert_eq!(scan_mention_markers("@ @m{task:1}(一)"), {
            let mut m = scan_mention_markers("@m{task:1}(一)");
            m[0].position = 2;
            m
        });
    }
}
