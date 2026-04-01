use crate::services::follow_up_context::{
    dictionary_forms, dictionary_ipa, dictionary_senses, first_summary_line, section_lines,
};
use crate::services::follow_up_intent::{
    detect_follow_up_intent, select_relevant_form_lines, FollowUpIntent,
};
use serde_json::Value;

pub fn build_follow_up_fallback(query_text: &str, question: &str, analysis: &Value) -> String {
    let markdown = analysis
        .get("markdown")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let empty_excerpt = Value::Null;
    let excerpt = analysis.get("dictionary_excerpt").unwrap_or(&empty_excerpt);

    let answer = match detect_follow_up_intent(question) {
        FollowUpIntent::Form => build_form_answer(query_text, question, markdown, excerpt),
        FollowUpIntent::Meaning => build_meaning_answer(query_text, markdown, excerpt),
        FollowUpIntent::Pronunciation => build_pronunciation_answer(query_text, markdown, excerpt),
        FollowUpIntent::Example => build_example_answer(query_text, markdown, excerpt),
        FollowUpIntent::Generic => generic_summary(query_text, markdown, excerpt),
    };

    truncate_text(&normalize_answer(&answer), 180)
}

pub fn normalize_answer(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn build_form_answer(query_text: &str, question: &str, markdown: &str, excerpt: &Value) -> String {
    let forms = select_relevant_form_lines(question, &section_lines(markdown, "关键形式"));
    if !forms.is_empty() {
        return format!(
            "先按当前条目直接回答：`{query_text}` 常用形式是 {}。",
            forms.join("；")
        );
    }

    if let Some(forms) = dictionary_forms(excerpt) {
        return format!(
            "先按当前条目直接回答：`{query_text}` 常用形式是 {}。",
            forms
        );
    }

    generic_summary(query_text, markdown, excerpt)
}

fn build_meaning_answer(query_text: &str, markdown: &str, excerpt: &Value) -> String {
    if let Some(senses) = dictionary_senses(excerpt) {
        format!(
            "先按当前条目直接回答：`{query_text}` 这里优先记 {}。",
            senses
        )
    } else {
        generic_summary(query_text, markdown, excerpt)
    }
}

fn build_pronunciation_answer(query_text: &str, markdown: &str, excerpt: &Value) -> String {
    if let Some(ipa) = dictionary_ipa(excerpt) {
        format!("先按当前条目直接回答：`{query_text}` 的音标是 /{ipa}/。如果你要记忆，先抓重音和长元音。")
    } else {
        generic_summary(query_text, markdown, excerpt)
    }
}

fn build_example_answer(query_text: &str, markdown: &str, excerpt: &Value) -> String {
    let examples = section_lines(markdown, "例句");
    let notes = section_lines(markdown, "使用提示");
    if !examples.is_empty() {
        let mut parts = vec![format!(
            "先按当前条目直接回答：可以先看这个例子 {}",
            examples[0]
        )];
        if let Some(note) = notes.first() {
            parts.push(format!("提醒：{note}"));
        }
        parts.join(" ")
    } else {
        generic_summary(query_text, markdown, excerpt)
    }
}

fn generic_summary(query_text: &str, markdown: &str, excerpt: &Value) -> String {
    let summary_line = first_summary_line(markdown);
    let senses = dictionary_senses(excerpt);
    let forms = dictionary_forms(excerpt);

    match (summary_line, senses, forms) {
        (Some(summary), Some(senses), _) => format!(
            "先按当前条目直接回答：`{query_text}` 的核心还是 {senses}。你可以先抓住这一点：{summary}"
        ),
        (Some(summary), None, Some(forms)) => format!(
            "先按当前条目直接回答：`{query_text}` 先抓这条主线：{summary} 常见形式有 {forms}。"
        ),
        (Some(summary), None, None) => {
            format!("先按当前条目直接回答：`{query_text}` 先抓这条主线：{summary}")
        }
        (None, Some(senses), _) => {
            format!("先按当前条目直接回答：`{query_text}` 的核心还是 {senses}。")
        }
        _ => format!(
            "先按当前条目直接回答：`{query_text}` 的核心信息已经在当前分析里，建议先抓住词义和关键形式，再继续细问。"
        ),
    }
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        format!("{}...", value.chars().take(max_chars).collect::<String>())
    }
}
