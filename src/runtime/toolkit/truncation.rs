#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TruncationDirection {
    Head,
    Tail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TruncationUnit {
    Lines,
    Bytes,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TruncationOptions {
    pub max_lines: Option<usize>,
    pub max_bytes: Option<usize>,
    pub direction: TruncationDirection,
}

impl Default for TruncationOptions {
    fn default() -> Self {
        Self {
            max_lines: None,
            max_bytes: None,
            direction: TruncationDirection::Head,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TruncationResult {
    pub content: String,
    pub truncated: bool,
    pub removed: usize,
    pub unit: Option<TruncationUnit>,
}

impl TruncationResult {
    fn not_truncated(content: String) -> Self {
        Self {
            content,
            truncated: false,
            removed: 0,
            unit: None,
        }
    }
}

pub fn truncate_text(text: &str, options: TruncationOptions) -> TruncationResult {
    if options.max_lines.is_none() && options.max_bytes.is_none() {
        return TruncationResult::not_truncated(text.to_string());
    }

    let max_lines = options.max_lines.unwrap_or(usize::MAX);
    let max_bytes = options.max_bytes.unwrap_or(usize::MAX);
    let total_bytes = text.as_bytes().len();
    let lines: Vec<&str> = if text.is_empty() {
        Vec::new()
    } else {
        text.split('\n').collect()
    };
    let total_lines = lines.len();

    if total_lines <= max_lines && total_bytes <= max_bytes {
        return TruncationResult::not_truncated(text.to_string());
    }

    let mut out: Vec<&str> = Vec::new();
    let mut bytes = 0usize;
    let mut hit_bytes = false;

    match options.direction {
        TruncationDirection::Head => {
            for line in lines.iter() {
                if out.len() >= max_lines {
                    break;
                }
                let size = line.as_bytes().len() + if out.is_empty() { 0 } else { 1 };
                if bytes + size > max_bytes {
                    hit_bytes = true;
                    break;
                }
                out.push(*line);
                bytes += size;
            }
        }
        TruncationDirection::Tail => {
            for line in lines.iter().rev() {
                if out.len() >= max_lines {
                    break;
                }
                let size = line.as_bytes().len() + if out.is_empty() { 0 } else { 1 };
                if bytes + size > max_bytes {
                    hit_bytes = true;
                    break;
                }
                out.push(*line);
                bytes += size;
            }
            out.reverse();
        }
    }

    let truncated_by_lines = total_lines > out.len();
    let truncated = hit_bytes || truncated_by_lines;
    if !truncated {
        return TruncationResult::not_truncated(text.to_string());
    }

    let (removed, unit) = if hit_bytes {
        (total_bytes.saturating_sub(bytes), TruncationUnit::Bytes)
    } else {
        (total_lines.saturating_sub(out.len()), TruncationUnit::Lines)
    };

    TruncationResult {
        content: out.join("\n"),
        truncated: true,
        removed,
        unit: Some(unit),
    }
}

#[cfg(test)]
mod tests {
    use super::{truncate_text, TruncationDirection, TruncationOptions, TruncationUnit};

    #[test]
    fn truncate_text_returns_full_content_without_limits() {
        let text = "alpha\nbeta";
        let result = truncate_text(text, TruncationOptions::default());
        assert_eq!(result.content, text);
        assert!(!result.truncated);
        assert_eq!(result.removed, 0);
        assert_eq!(result.unit, None);
    }

    #[test]
    fn truncate_text_limits_by_lines() {
        let text = "a\nb\nc\nd";
        let result = truncate_text(
            text,
            TruncationOptions {
                max_lines: Some(2),
                max_bytes: None,
                direction: TruncationDirection::Head,
            },
        );
        assert_eq!(result.content, "a\nb");
        assert!(result.truncated);
        assert_eq!(result.removed, 2);
        assert_eq!(result.unit, Some(TruncationUnit::Lines));
    }

    #[test]
    fn truncate_text_limits_by_bytes() {
        let text = "alpha\nbeta";
        let result = truncate_text(
            text,
            TruncationOptions {
                max_lines: None,
                max_bytes: Some(7),
                direction: TruncationDirection::Head,
            },
        );
        assert_eq!(result.content, "alpha");
        assert!(result.truncated);
        assert_eq!(result.removed, 5);
        assert_eq!(result.unit, Some(TruncationUnit::Bytes));
    }

    #[test]
    fn truncate_text_keeps_tail_when_requested() {
        let text = "a\nb\nc\nd";
        let result = truncate_text(
            text,
            TruncationOptions {
                max_lines: Some(2),
                max_bytes: None,
                direction: TruncationDirection::Tail,
            },
        );
        assert_eq!(result.content, "c\nd");
        assert!(result.truncated);
        assert_eq!(result.unit, Some(TruncationUnit::Lines));
    }
}
