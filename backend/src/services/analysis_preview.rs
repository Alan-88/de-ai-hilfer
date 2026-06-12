use crate::models::{StructuredAnalysisDocument, StructuredGrammarBranch};
use serde_json::Value;
use std::collections::BTreeSet;

pub fn analysis_markdown(analysis: &Value) -> String {
    analysis
        .get("markdown")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

pub fn structured_analysis(analysis: &Value) -> Option<StructuredAnalysisDocument> {
    analysis
        .get("structured")
        .cloned()
        .and_then(|value| serde_json::from_value(value).ok())
}

pub fn preview_from_analysis(analysis: &Value) -> String {
    if let Some(preview) = structured_preview_from_analysis(analysis) {
        return truncate(&preview, 90);
    }

    let markdown = analysis_markdown(analysis);
    if markdown.is_empty() {
        return "无法生成预览".to_string();
    }

    let lines = markdown
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| !line.starts_with('#'))
        .filter(|line| !line.starts_with("*/"))
        .collect::<Vec<_>>();

    let preview = if let Some(line) = lines
        .iter()
        .find(|line| line.contains("**") || line.contains("核心释义") || line.contains("* "))
    {
        line
    } else {
        lines.first().copied().unwrap_or("无法生成预览")
    };

    truncate(&strip_markdown_preview(preview), 90)
}

fn structured_preview_from_analysis(analysis: &Value) -> Option<String> {
    let structured = structured_analysis(analysis)?;

    if !structured.grammar_branches.is_empty() {
        if let Some(preview) = branch_preview(&structured.grammar_branches) {
            return Some(preview);
        }
    }

    structured.meanings.first().and_then(|meaning| {
        let pos = compact_pos(&meaning.part_of_speech, "");
        let chinese = meaning.chinese.trim();
        (!chinese.is_empty()).then(|| format!("[{pos}] {chinese}"))
    })
}

fn branch_preview(branches: &[StructuredGrammarBranch]) -> Option<String> {
    let summaries = preview_branches(branches);
    if summaries.is_empty() {
        return None;
    }

    Some(
        summaries
            .into_iter()
            .map(|branch| format!("[{}] {}", branch.label, branch.meaning))
            .collect::<Vec<_>>()
            .join(" · "),
    )
}

fn preview_branches(branches: &[StructuredGrammarBranch]) -> Vec<BranchPreview> {
    let usable = branches
        .iter()
        .filter_map(|branch| {
            let meaning = primary_branch_meaning(branch)?;
            Some(BranchPreview {
                pos: compact_pos(&branch.pos, &branch.selector),
                label: compact_branch_label(branch),
                meaning,
                discriminant: branch_discriminant(branch),
            })
        })
        .collect::<Vec<_>>();

    if usable.is_empty() {
        return Vec::new();
    }

    if has_distinct_discriminants(&usable) {
        return usable.into_iter().take(3).collect();
    }

    let mut by_pos = Vec::<BranchPreview>::new();
    for branch in usable {
        if let Some(existing) = by_pos.iter_mut().find(|item| item.pos == branch.pos) {
            merge_meaning(existing, &branch.meaning);
        } else {
            by_pos.push(branch);
        }
    }
    by_pos.into_iter().take(3).collect()
}

fn primary_branch_meaning(branch: &StructuredGrammarBranch) -> Option<String> {
    branch
        .meanings
        .iter()
        .map(|meaning| meaning.zh.trim())
        .find(|meaning| !meaning.is_empty())
        .map(str::to_string)
}

fn merge_meaning(target: &mut BranchPreview, meaning: &str) {
    let mut parts = target
        .meaning
        .split(['；', ';', '，', ',', '/'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();

    parts.extend(
        meaning
            .split(['；', ';', '，', ',', '/'])
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(str::to_string),
    );

    let mut seen = BTreeSet::new();
    let merged = parts
        .into_iter()
        .filter(|part| seen.insert(part.clone()))
        .take(3)
        .collect::<Vec<_>>();

    if !merged.is_empty() {
        target.meaning = merged.join("/");
    }
}

fn has_distinct_discriminants(branches: &[BranchPreview]) -> bool {
    branches
        .windows(2)
        .any(|items| items[0].pos == items[1].pos && items[0].discriminant != items[1].discriminant)
}

fn compact_branch_label(branch: &StructuredGrammarBranch) -> String {
    let pos = compact_pos(&branch.pos, &branch.selector);
    if pos == "v." {
        let separable = branch.grammar.separable.to_ascii_lowercase();
        if matches!(
            separable.as_str(),
            "separable" | "trennbar" | "yes" | "可分"
        ) {
            return "v. sep.".to_string();
        }
        if matches!(
            separable.as_str(),
            "inseparable" | "untrennbar" | "no" | "不可分"
        ) {
            return "v. insep.".to_string();
        }
    }

    if pos == "n." {
        if let Some(gender) = branch
            .grammar
            .genders
            .iter()
            .find_map(|gender| compact_gender(gender))
        {
            return format!("({gender}) n.");
        }
    }

    pos
}

fn branch_discriminant(branch: &StructuredGrammarBranch) -> String {
    let pos = compact_pos(&branch.pos, &branch.selector);
    if pos == "v." {
        let separable = branch.grammar.separable.trim();
        if !separable.is_empty() {
            return format!("separable:{separable}");
        }
    }

    if pos == "n." && !branch.grammar.genders.is_empty() {
        return format!("gender:{}", branch.grammar.genders.join("/"));
    }

    branch.selector.trim().to_string()
}

fn compact_gender(value: &str) -> Option<&'static str> {
    match value.trim().to_ascii_lowercase().as_str() {
        "masculine" | "maskulin" | "m" | "阳性" => Some("m."),
        "feminine" | "feminin" | "f" | "阴性" => Some("f."),
        "neuter" | "neutrum" | "n" | "中性" => Some("n."),
        "plural" | "pluralia tantum" | "pl" | "复数" => Some("pl."),
        _ => None,
    }
}

fn compact_pos(pos: &str, fallback: &str) -> String {
    let source = if pos.trim().is_empty() { fallback } else { pos };
    let lower = source.to_ascii_lowercase();

    if matches!(lower.as_str(), "v." | "verb") {
        "v.".to_string()
    } else if matches!(lower.as_str(), "adv." | "adverb") || lower.contains("adverb") {
        "adv.".to_string()
    } else if lower.contains("verb") {
        "v.".to_string()
    } else if lower.contains("noun") || lower.contains("substantiv") || lower.contains("nomen") {
        "n.".to_string()
    } else if lower.contains("adjective") || lower.contains("adjektiv") {
        "adj.".to_string()
    } else if lower.contains("preposition") || lower.contains("präposition") {
        "prep.".to_string()
    } else if lower.contains("conjunction")
        || lower.contains("konjunktion")
        || lower.contains("subjunktion")
    {
        "conj.".to_string()
    } else if lower.contains("pronoun") || lower.contains("pronomen") {
        "pron.".to_string()
    } else if lower.contains("article") || lower.contains("artikel") {
        "art.".to_string()
    } else {
        let trimmed = source.trim();
        if trimmed.is_empty() {
            "?".to_string()
        } else {
            trimmed.to_string()
        }
    }
}

struct BranchPreview {
    pos: String,
    label: String,
    meaning: String,
    discriminant: String,
}

fn truncate(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        value.to_string()
    } else {
        value.chars().take(max_len).collect::<String>() + "..."
    }
}

fn strip_markdown_preview(value: &str) -> String {
    let mut cleaned = String::with_capacity(value.len());
    let mut previous_was_space = false;

    for ch in value.chars() {
        let normalized = match ch {
            '*' | '`' | '#' | '[' | ']' | '(' | ')' | '>' | '_' => None,
            '\n' | '\r' | '\t' => Some(' '),
            _ => Some(ch),
        };

        if let Some(ch) = normalized {
            let ch = if ch.is_whitespace() { ' ' } else { ch };
            if ch == ' ' {
                if previous_was_space {
                    continue;
                }
                previous_was_space = true;
            } else {
                previous_was_space = false;
            }
            cleaned.push(ch);
        }
    }

    cleaned
        .trim()
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim()
        .to_string()
}
