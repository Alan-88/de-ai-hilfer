use crate::models::{KnowledgeEntry, LearningProgress, LearningRecallRating};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use std::collections::{HashMap, VecDeque};

pub(crate) const MAX_INTRADAY_APPEARANCES: i32 = 6;

pub type LearningSessionStore = HashMap<String, LearningSessionRuntime>;

#[derive(Debug, Clone)]
pub struct LearningSessionRuntime {
    pub(crate) id: String,
    pub(crate) business_date: NaiveDate,
    pub(crate) review_items: VecDeque<LearningSessionItem>,
    pub(crate) new_items: VecDeque<LearningSessionItem>,
    pub(crate) intraday_items: Vec<LearningSessionItem>,
    pub(crate) current: Option<LearningSessionItem>,
    pub(crate) completed_count: i32,
    pub(crate) total_count: i32,
    seen_cursor: i32,
}

#[derive(Debug, Clone)]
pub(crate) struct LearningSessionItem {
    pub(crate) progress: LearningProgress,
    pub(crate) entry: KnowledgeEntry,
    pub(crate) phase: LearningPhase,
    pub(crate) first_rating: Option<LearningRecallRating>,
    pub(crate) appearance_count_today: i32,
    success_streak: i32,
    due_after_seen: i32,
    earliest_review_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LearningPhase {
    New,
    Review,
    Intraday,
}

impl LearningPhase {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            LearningPhase::New => "new",
            LearningPhase::Review => "review",
            LearningPhase::Intraday => "intraday",
        }
    }
}

impl LearningSessionRuntime {
    pub(crate) fn new(
        id: String,
        business_date: NaiveDate,
        review_items: VecDeque<LearningSessionItem>,
        new_items: VecDeque<LearningSessionItem>,
    ) -> Self {
        let total_count = (review_items.len() + new_items.len()) as i32;
        Self {
            id,
            business_date,
            review_items,
            new_items,
            intraday_items: Vec::new(),
            current: None,
            completed_count: 0,
            total_count,
            seen_cursor: 0,
        }
    }

    pub(crate) fn select_next(&mut self, now: DateTime<Utc>) {
        if self.current.is_some() {
            return;
        }

        if let Some(index) = self.next_due_intraday_index(now) {
            let item = self.intraday_items.remove(index);
            self.set_current(item);
            return;
        }

        if let Some(item) = self.review_items.pop_front() {
            self.set_current(item);
            return;
        }

        if let Some(item) = self.new_items.pop_front() {
            self.set_current(item);
            return;
        }

        if !self.intraday_items.is_empty() {
            let index = self.closest_intraday_index();
            let item = self.intraday_items.remove(index);
            self.set_current(item);
        }
    }

    pub(crate) fn requeue_intraday(
        &mut self,
        mut item: LearningSessionItem,
        rating: LearningRecallRating,
    ) {
        item.phase = LearningPhase::Intraday;
        item.due_after_seen = self.seen_cursor + card_gap_for_rating(rating);
        item.earliest_review_at = Utc::now() + min_delay_for_rating(rating);
        self.intraday_items.push(item);
    }

    pub(crate) fn is_completed(&self) -> bool {
        self.current.is_none()
            && self.review_items.is_empty()
            && self.new_items.is_empty()
            && self.intraday_items.is_empty()
    }

    fn next_due_intraday_index(&self, now: DateTime<Utc>) -> Option<usize> {
        self.intraday_items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                item.due_after_seen <= self.seen_cursor && item.earliest_review_at <= now
            })
            .min_by_key(|(_, item)| (item.due_after_seen, item.earliest_review_at))
            .map(|(index, _)| index)
    }

    fn closest_intraday_index(&self) -> usize {
        self.intraday_items
            .iter()
            .enumerate()
            .min_by_key(|(_, item)| (item.due_after_seen, item.earliest_review_at))
            .map(|(index, _)| index)
            .unwrap_or(0)
    }

    fn set_current(&mut self, mut item: LearningSessionItem) {
        if item.appearance_count_today > 0 {
            item.phase = LearningPhase::Intraday;
        }
        item.appearance_count_today += 1;
        self.seen_cursor += 1;
        self.current = Some(item);
    }
}

pub(crate) fn session_item(
    progress: LearningProgress,
    entry: KnowledgeEntry,
    phase: LearningPhase,
    now: DateTime<Utc>,
) -> LearningSessionItem {
    LearningSessionItem {
        progress,
        entry,
        phase,
        first_rating: None,
        appearance_count_today: 0,
        success_streak: 0,
        due_after_seen: 0,
        earliest_review_at: now,
    }
}

pub(crate) fn should_complete_today(
    item: &mut LearningSessionItem,
    rating: LearningRecallRating,
    is_first_today: bool,
) -> bool {
    match rating {
        LearningRecallRating::Known => item.success_streak += 1,
        LearningRecallRating::Fuzzy | LearningRecallRating::Forgotten => item.success_streak = 0,
    }

    if is_first_today && rating == LearningRecallRating::Known {
        return true;
    }

    if item.appearance_count_today >= MAX_INTRADAY_APPEARANCES {
        return true;
    }

    item.success_streak >= required_success_streak(item.first_rating)
}

fn required_success_streak(first_rating: Option<LearningRecallRating>) -> i32 {
    match first_rating {
        Some(LearningRecallRating::Forgotten) => 2,
        Some(LearningRecallRating::Fuzzy) => 1,
        _ => 1,
    }
}

fn card_gap_for_rating(rating: LearningRecallRating) -> i32 {
    match rating {
        LearningRecallRating::Forgotten => 2,
        LearningRecallRating::Fuzzy => 5,
        LearningRecallRating::Known => 5,
    }
}

fn min_delay_for_rating(rating: LearningRecallRating) -> Duration {
    match rating {
        LearningRecallRating::Forgotten => Duration::seconds(45),
        LearningRecallRating::Fuzzy => Duration::minutes(2),
        LearningRecallRating::Known => Duration::minutes(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_known_completes_today() {
        let mut item = empty_item(LearningPhase::New);
        item.first_rating = Some(LearningRecallRating::Known);
        item.appearance_count_today = 1;

        assert!(should_complete_today(
            &mut item,
            LearningRecallRating::Known,
            true
        ));
    }

    #[test]
    fn first_fuzzy_needs_one_later_known() {
        let mut item = empty_item(LearningPhase::New);
        item.first_rating = Some(LearningRecallRating::Fuzzy);
        item.appearance_count_today = 1;

        assert!(!should_complete_today(
            &mut item,
            LearningRecallRating::Fuzzy,
            true
        ));

        item.appearance_count_today = 2;
        assert!(should_complete_today(
            &mut item,
            LearningRecallRating::Known,
            false
        ));
    }

    #[test]
    fn first_forgotten_needs_two_later_known_answers() {
        let mut item = empty_item(LearningPhase::Review);
        item.first_rating = Some(LearningRecallRating::Forgotten);
        item.appearance_count_today = 2;

        assert!(!should_complete_today(
            &mut item,
            LearningRecallRating::Known,
            false
        ));

        item.appearance_count_today = 3;
        assert!(should_complete_today(
            &mut item,
            LearningRecallRating::Known,
            false
        ));
    }

    #[test]
    fn max_appearance_caps_intraday_loop() {
        let mut item = empty_item(LearningPhase::Intraday);
        item.first_rating = Some(LearningRecallRating::Forgotten);
        item.appearance_count_today = MAX_INTRADAY_APPEARANCES;

        assert!(should_complete_today(
            &mut item,
            LearningRecallRating::Forgotten,
            false
        ));
    }

    fn empty_item(phase: LearningPhase) -> LearningSessionItem {
        LearningSessionItem {
            progress: LearningProgress {
                entry_id: 1,
                stability: 0.0,
                difficulty: 0.0,
                elapsed_days: 0,
                scheduled_days: 0,
                state: 0,
                last_review_at: None,
                due_date: Utc::now(),
                review_count: 0,
                lapses: 0,
            },
            entry: KnowledgeEntry {
                id: 1,
                query_text: "Test".to_string(),
                lexeme_id: None,
                prototype: None,
                entry_type: "WORD".to_string(),
                analysis: serde_json::json!({ "markdown": "" }),
                tags: None,
                aliases: None,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            phase,
            first_rating: None,
            appearance_count_today: 0,
            success_streak: 0,
            due_after_seen: 0,
            earliest_review_at: Utc::now(),
        }
    }
}
