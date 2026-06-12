use crate::models::{
    StructuredAnalysisDocument, StructuredBranchMeaning, StructuredGrammarBranch,
    StructuredGrammarRow, StructuredMeaning,
};
use crate::services::analysis_grounded_facts::first_ipa_from_dictionary_facts;
use crate::services::analysis_grounded_model_a::{ModelAEntry, ModelAOutput};

pub fn assemble_grounded_structured_document(
    word: &str,
    dictionary_facts: Option<&str>,
    stage1_output: &ModelAOutput,
    mut module_document: StructuredAnalysisDocument,
) -> StructuredAnalysisDocument {
    module_document.headword = word.trim().to_string();
    module_document.phonetic =
        first_ipa_from_dictionary_facts(dictionary_facts).unwrap_or_default();
    module_document.meanings = meanings_from_stage1(stage1_output);
    module_document.grammar_rows = grammar_rows_from_stage1(stage1_output);
    module_document.grammar_branches = grammar_branches_from_stage1(stage1_output);
    module_document
}

fn meanings_from_stage1(stage1_output: &ModelAOutput) -> Vec<StructuredMeaning> {
    stage1_output
        .entries
        .iter()
        .flat_map(|entry| {
            let part_of_speech = display_selector(entry);
            entry.meanings.iter().map(move |meaning| StructuredMeaning {
                part_of_speech: part_of_speech.clone(),
                chinese: meaning.zh.trim().to_string(),
                english: meaning.en.trim().to_string(),
            })
        })
        .filter(|meaning| !meaning.chinese.is_empty() || !meaning.english.is_empty())
        .collect()
}

fn grammar_rows_from_stage1(stage1_output: &ModelAOutput) -> Vec<StructuredGrammarRow> {
    let multiple_entries = stage1_output.entries.len() > 1;
    let mut rows = Vec::new();

    for entry in &stage1_output.entries {
        let selector = display_selector(entry);
        push_grammar_row(&mut rows, multiple_entries, &selector, "词性", &selector);
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "性",
            &entry.grammar.genders.join("/"),
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "名词变化",
            &entry.grammar.noun_class,
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "复数",
            &entry.grammar.plural_forms.join("/"),
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "第二格",
            &entry.grammar.genitive_forms.join("/"),
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "可分性",
            &entry.grammar.separable,
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "及物性",
            &entry.grammar.transitivity,
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "反身",
            &entry.grammar.reflexive,
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "助动词",
            &entry.grammar.auxiliaries.join("/"),
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "现在时第三人称单数",
            &entry.grammar.present_3sg,
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "过去时第三人称单数",
            &entry.grammar.preterite_3sg,
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "第二分词",
            &entry.grammar.partizip_ii,
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "比较级",
            &entry.grammar.comparative,
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "最高级",
            &entry.grammar.superlative,
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "支配格",
            &entry.grammar.governs_cases.join("/"),
        );
        push_grammar_row(
            &mut rows,
            multiple_entries,
            &selector,
            "语序",
            &entry.grammar.word_order,
        );
    }

    rows
}

fn grammar_branches_from_stage1(stage1_output: &ModelAOutput) -> Vec<StructuredGrammarBranch> {
    stage1_output
        .entries
        .iter()
        .map(|entry| StructuredGrammarBranch {
            selector: display_selector(entry),
            pos: entry.pos.trim().to_string(),
            meanings: entry
                .meanings
                .iter()
                .map(|meaning| StructuredBranchMeaning {
                    zh: meaning.zh.trim().to_string(),
                    en: meaning.en.trim().to_string(),
                })
                .filter(|meaning| !meaning.zh.is_empty() || !meaning.en.is_empty())
                .collect(),
            grammar: (&entry.grammar).into(),
        })
        .filter(|branch| {
            !branch.selector.is_empty()
                || !branch.pos.is_empty()
                || !branch.meanings.is_empty()
                || has_non_empty_branch_grammar(&branch.grammar)
        })
        .collect()
}

fn has_non_empty_branch_grammar(grammar: &crate::models::StructuredBranchGrammar) -> bool {
    !grammar.genders.is_empty()
        || !grammar.noun_class.is_empty()
        || !grammar.plural_forms.is_empty()
        || !grammar.genitive_forms.is_empty()
        || !grammar.separable.is_empty()
        || !grammar.transitivity.is_empty()
        || !grammar.reflexive.is_empty()
        || !grammar.auxiliaries.is_empty()
        || !grammar.present_3sg.is_empty()
        || !grammar.preterite_3sg.is_empty()
        || !grammar.partizip_ii.is_empty()
        || !grammar.comparative.is_empty()
        || !grammar.superlative.is_empty()
        || !grammar.governs_cases.is_empty()
        || !grammar.word_order.is_empty()
}

fn push_grammar_row(
    rows: &mut Vec<StructuredGrammarRow>,
    multiple_entries: bool,
    selector: &str,
    label: &str,
    value: &str,
) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }

    let key = if multiple_entries {
        format!("{selector} / {label}")
    } else {
        label.to_string()
    };

    rows.push(StructuredGrammarRow {
        key,
        value: value.to_string(),
    });
}

fn display_selector(entry: &ModelAEntry) -> String {
    if !entry.selector.trim().is_empty() {
        entry.selector.trim().to_string()
    } else {
        entry.pos.trim().to_string()
    }
}
