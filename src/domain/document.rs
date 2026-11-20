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
}
