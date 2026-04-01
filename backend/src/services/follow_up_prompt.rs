use crate::models::FollowUp;
use serde_json::Value;

pub fn build_follow_up_prompt(
    prompt_template: &str,
    vocabulary_list: &[String],
    question: &str,
    analysis: &Value,
    history: &[FollowUp],
) -> String {
    let analysis_markdown = analysis
        .get("markdown")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let history_text = history
        .iter()
        .rev()
        .take(3)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|item| {
            format!(
                "Q: {}\nA: {}",
                truncate_text(&item.question, 80),
                truncate_text(&item.answer, 140)
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let context = format!(
        "原始分析:\n{}\n\n历史问答:\n{}",
        truncate_text(analysis_markdown, 1200),
        if history_text.is_empty() {
            "暂无历史问答".to_string()
        } else {
            history_text
        }
    );

    format!(
        "{}\n\n--- Rust 迁移阶段附加限制 ---\n当前不支持工具调用或额外词条检索；你只能依据提供的上下文直接回答。请使用中文，控制在 150 字以内，优先给结论，再补一条最关键的解释、形式或例子。",
        prompt_template
            .replace("{vocabulary_list}", &vocabulary_list.join(", "))
            .replace("{context}", &context)
            .replace("{question}", question)
    )
}

fn truncate_text(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        value.to_string()
    } else {
        format!("{}...", value.chars().take(max_chars).collect::<String>())
    }
}
