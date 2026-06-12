use crate::prompts::PromptConfig;
use crate::services::analysis_grounded_model_a::ModelAOutput;

const DEFAULT_MODEL_A_PROMPT: &str = include_str!("../../config/prompts/model_a.md");
const DEFAULT_STAGE2_PROMPT: &str = include_str!("../../config/prompts/stage2.md");

pub fn build_model_a_prompt(prompts: &PromptConfig) -> String {
    let prompt = prompt_or_default(&prompts.model_a_prompt, DEFAULT_MODEL_A_PROMPT);
    prompt.replace("{model_a_schema}", model_a_schema_block())
}

pub fn build_model_a_user_payload(word: &str, dictionary_facts: Option<&str>) -> String {
    let dictionary_facts = dictionary_facts.unwrap_or("{}");
    format!("查询词：{word}\n\n--- Dictionary Facts JSON ---\n{dictionary_facts}")
}

pub fn build_stage2_prompt(prompts: &PromptConfig) -> String {
    prompt_or_default(&prompts.stage2_prompt, DEFAULT_STAGE2_PROMPT).to_string()
}

pub fn build_stage2_user_payload(
    word: &str,
    dictionary_facts: Option<&str>,
    stage1_output: &ModelAOutput,
) -> String {
    let dictionary_facts = dictionary_facts.unwrap_or("{}");
    let stage1_summary = render_stage1_boundary_summary(stage1_output);
    format!(
        "查询词：{word}\n\n--- Dictionary Facts JSON ---\n{dictionary_facts}\n\n--- Branch Skeleton Reference ---\n{stage1_summary}"
    )
}

fn render_stage1_boundary_summary(stage1_output: &ModelAOutput) -> String {
    if stage1_output.entries.is_empty() {
        return "无稳定 branch 参考。".to_string();
    }

    let mut lines = vec![
        "下面是本轮输入的 Branch Skeleton。".to_string(),
        "它的作用是防止你越界，不是要求你在 usage_modules 中给每个 branch 都分配名额。".to_string(),
        "默认把靠前的 branch 视为更核心、更常用；靠后的 branch 可以只由释义/语法层承接，不必单独展开成 usage module。".to_string(),
        "如果 secondary branch 只是补充性的分词形容词、书面义、军事义或边缘义，不要因为它单独成条就默认展开成 usage module。".to_string(),
    ];

    for (index, entry) in stage1_output.entries.iter().enumerate() {
        let meanings = entry
            .meanings
            .iter()
            .map(|meaning| match (meaning.zh.trim(), meaning.en.trim()) {
                ("", "") => String::new(),
                (zh, "") => zh.to_string(),
                ("", en) => en.to_string(),
                (zh, en) => format!("{zh} ({en})"),
            })
            .filter(|item| !item.is_empty())
            .collect::<Vec<_>>()
            .join("；");

        let grammar_bits = [
            (!entry.grammar.separable.is_empty())
                .then(|| format!("separable={}", entry.grammar.separable)),
            (!entry.grammar.transitivity.is_empty())
                .then(|| format!("transitivity={}", entry.grammar.transitivity)),
            (!entry.grammar.reflexive.is_empty())
                .then(|| format!("reflexive={}", entry.grammar.reflexive)),
            (!entry.grammar.governs_cases.is_empty())
                .then(|| format!("cases={}", entry.grammar.governs_cases.join("/"))),
            (!entry.grammar.present_3sg.is_empty())
                .then(|| format!("present_3sg={}", entry.grammar.present_3sg)),
            (!entry.grammar.preterite_3sg.is_empty())
                .then(|| format!("preterite_3sg={}", entry.grammar.preterite_3sg)),
            (!entry.grammar.partizip_ii.is_empty())
                .then(|| format!("partizip_ii={}", entry.grammar.partizip_ii)),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");

        let priority = if index == 0 { "core" } else { "secondary" };
        let grammar_fragment = if index == 0 && !grammar_bits.is_empty() {
            format!("; grammar: {grammar_bits}")
        } else {
            String::new()
        };
        lines.push(format!(
            "{idx}. [{priority}] selector={selector}; pos={pos}; meanings={meanings}{grammar}",
            idx = index + 1,
            selector = entry.selector,
            pos = entry.pos,
            meanings = if meanings.is_empty() {
                "（空）"
            } else {
                meanings.as_str()
            },
            grammar = grammar_fragment
        ));
    }

    lines.join("\n")
}

fn prompt_or_default<'a>(configured: &'a str, default_prompt: &'static str) -> &'a str {
    let prompt = configured.trim();
    if prompt.is_empty() {
        default_prompt.trim()
    } else {
        prompt
    }
}

fn model_a_schema_block() -> &'static str {
    "{\n\
  \"word\": \"\",\n\
  \"entries\": [\n\
    {\n\
      \"selector\": \"\",\n\
      \"pos\": \"\",\n\
      \"meanings\": [{\"zh\": \"\", \"en\": \"\"}],\n\
      \"grammar\": {\n\
        \"genders\": [\"\"],\n\
        \"noun_class\": \"\",\n\
        \"plural_forms\": [\"\"],\n\
        \"genitive_forms\": [\"\"],\n\
        \"separable\": \"\",\n\
        \"transitivity\": \"\",\n\
        \"reflexive\": \"\",\n\
        \"auxiliaries\": [\"\"],\n\
        \"present_3sg\": \"\",\n\
        \"preterite_3sg\": \"\",\n\
        \"partizip_ii\": \"\",\n\
        \"comparative\": \"\",\n\
        \"superlative\": \"\",\n\
        \"governs_cases\": [\"\"],\n\
        \"word_order\": \"\"\n\
      }\n\
    }\n\
  ]\n\
}"
}
