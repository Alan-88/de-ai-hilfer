#[derive(Debug, Clone, Copy)]
pub enum FollowUpIntent {
    Form,
    Meaning,
    Pronunciation,
    Example,
    Generic,
}

pub fn detect_follow_up_intent(question: &str) -> FollowUpIntent {
    if contains_any(
        question,
        &[
            "变位",
            "形式",
            "过去式",
            "过去分词",
            "第三人称",
            "助动词",
            "时态",
        ],
    ) {
        FollowUpIntent::Form
    } else if contains_any(
        question,
        &["意思", "释义", "含义", "翻译", "区别", "什么意思"],
    ) {
        FollowUpIntent::Meaning
    } else if contains_any(question, &["发音", "音标", "怎么读", "读音"]) {
        FollowUpIntent::Pronunciation
    } else if contains_any(question, &["例句", "造句", "怎么用", "用法", "搭配"]) {
        FollowUpIntent::Example
    } else {
        FollowUpIntent::Generic
    }
}

pub fn select_relevant_form_lines(question: &str, lines: &[String]) -> Vec<String> {
    let cleaned = lines
        .iter()
        .filter(|line| !line.contains("table-tags") && !line.contains("inflection-template"))
        .cloned()
        .collect::<Vec<_>>();
    let mut selected = Vec::new();

    if question.contains("过去式") {
        selected.extend(
            cleaned
                .iter()
                .filter(|line| line.contains("(past)") || line.contains("preterite"))
                .cloned(),
        );
    }

    if question.contains("过去分词") {
        selected.extend(
            cleaned
                .iter()
                .filter(|line| line.contains("participle"))
                .cloned(),
        );
    }

    if question.contains("第三人称") {
        selected.extend(
            cleaned
                .iter()
                .filter(|line| line.contains("third-person"))
                .cloned(),
        );
    }

    if question.contains("助动词") {
        selected.extend(
            cleaned
                .iter()
                .filter(|line| line.contains("auxiliary"))
                .cloned(),
        );
    }

    if selected.is_empty() {
        cleaned.into_iter().take(4).collect()
    } else {
        selected.dedup();
        selected
    }
}

fn contains_any(question: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| question.contains(needle))
}
