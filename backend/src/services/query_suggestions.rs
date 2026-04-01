use crate::models::DBSuggestion;
use crate::services::query_inference::normalize_for_match;
use std::collections::HashSet;

#[derive(Debug)]
pub struct RankedSuggestion {
    pub suggestion: DBSuggestion,
    pub normalized_query_text: String,
    pub match_tier: u8,
    pub source_tier: u8,
    pub relation_tier: u8,
    pub case_tier: u8,
    pub distance: usize,
}

pub fn rank_candidate(
    query: &str,
    match_text: &str,
    source: &str,
    suggestion: DBSuggestion,
    relation_tier: u8,
) -> Option<RankedSuggestion> {
    let normalized_query = normalize_for_match(query);
    let normalized_match = normalize_for_match(match_text);
    if normalized_query.is_empty() || normalized_match.is_empty() {
        return None;
    }

    let query_lower = query.trim().to_lowercase();
    let match_lower = match_text.trim().to_lowercase();
    let distance = levenshtein_distance(&normalized_query, &normalized_match);
    let threshold = fuzzy_threshold(normalized_query.chars().count());

    let match_tier = if match_lower.starts_with(&query_lower) {
        0
    } else if normalized_match.starts_with(&normalized_query) {
        1
    } else if distance <= threshold {
        2
    } else {
        return None;
    };

    let source_tier = match (source, suggestion.suggestion_type.as_str()) {
        ("知识库", "knowledge_prefix") => 0,
        ("知识库", "knowledge_alias_prefix") => 1,
        ("词典", "dictionary_lexeme") => 2,
        ("词典", "dictionary_prefix") => 2,
        ("知识库", "knowledge_fuzzy") => 3,
        ("知识库", "knowledge_alias_fuzzy") => 4,
        ("词典", "dictionary_fuzzy") => 5,
        ("知识库", _) => 6,
        _ => 7,
    };
    let case_tier = case_match_tier(query, match_text);

    Some(RankedSuggestion {
        normalized_query_text: normalize_for_match(&suggestion.query_text),
        suggestion,
        match_tier,
        source_tier,
        relation_tier,
        case_tier,
        distance,
    })
}

pub fn sort_and_limit(mut ranked: Vec<RankedSuggestion>, limit: usize) -> Vec<DBSuggestion> {
    ranked.sort_by(|left, right| {
        left.match_tier
            .cmp(&right.match_tier)
            .then(left.case_tier.cmp(&right.case_tier))
            .then(left.source_tier.cmp(&right.source_tier))
            .then(left.relation_tier.cmp(&right.relation_tier))
            .then(left.distance.cmp(&right.distance))
            .then(left.normalized_query_text.cmp(&right.normalized_query_text))
            .then(left.suggestion.query_text.cmp(&right.suggestion.query_text))
    });

    let mut seen = HashSet::new();
    ranked
        .into_iter()
        .filter(|item| seen.insert(item.suggestion.query_text.clone()))
        .take(limit)
        .map(|item| item.suggestion)
        .collect()
}

fn case_match_tier(query: &str, match_text: &str) -> u8 {
    if match_text == query {
        return 0;
    }

    if same_initial_case(query, match_text) {
        return 1;
    }

    if match_text.trim().to_lowercase() == query.trim().to_lowercase() {
        return 2;
    }

    3
}

fn same_initial_case(query: &str, match_text: &str) -> bool {
    match (
        first_alphabetic_char(query).map(|ch| ch.is_uppercase()),
        first_alphabetic_char(match_text).map(|ch| ch.is_uppercase()),
    ) {
        (Some(left), Some(right)) => left == right,
        _ => false,
    }
}

fn first_alphabetic_char(value: &str) -> Option<char> {
    value.chars().find(|ch| ch.is_alphabetic())
}

fn fuzzy_threshold(term_len: usize) -> usize {
    match term_len {
        0..=4 => 1,
        5..=7 => 2,
        _ => 3,
    }
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    let left_chars = left.chars().collect::<Vec<_>>();
    let right_chars = right.chars().collect::<Vec<_>>();
    let mut prev = (0..=right_chars.len()).collect::<Vec<_>>();
    let mut curr = vec![0; right_chars.len() + 1];

    for (i, left_char) in left_chars.iter().enumerate() {
        curr[0] = i + 1;

        for (j, right_char) in right_chars.iter().enumerate() {
            let cost = usize::from(left_char != right_char);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }

        prev.clone_from(&curr);
    }

    prev[right_chars.len()]
}
