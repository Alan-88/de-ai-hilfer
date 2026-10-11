use crate::models::{KnowledgeEntry, LearningProgress, LearningRecallRating};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use std::collections::{HashMap, VecDeque};

pub(crate) const MAX_INTRADAY_APPEARANCES: i32 = 6;
// This limits unresolved intraday intake, not the number of new words per day.
pub(crate) const ACTIVE_INTRADAY_WORKSET_TARGET: usize = 10;
pub(crate) const INTRADAY_FORGOTTEN_DELAY_SECONDS: i64 = 45;
pub(crate) const INTRADAY_FUZZY_DELAY_SECONDS: i64 = 120;
pub(crate) const INTRADAY_KNOWN_DELAY_SECONDS: i64 = 60;
pub(crate) const INTRADAY_RECOVERY_KNOWN_DELAY_SECONDS: i64 = 300;

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
        intraday_items: Vec<LearningSessionItem>,
    ) -> Self {
        let total_count = (review_items.len() + new_items.len() + intraday_items.len()) as i32;
        Self {
            id,
            business_date,
            review_items,
            new_items,
            intraday_items,
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

        if self.intraday_items.len() < ACTIVE_INTRADAY_WORKSET_TARGET {
            if let Some(item) = self.new_items.pop_front() {
                self.set_current(item);
                return;
            }
        }

        if self.intraday_items.len() >= ACTIVE_INTRADAY_WORKSET_TARGET {
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
        item.earliest_review_at = Utc::now() + min_delay_for_item(&item, rating);
        self.intraday_items.push(item);
    }

    pub(crate) fn next_available_at(&self) -> Option<DateTime<Utc>> {
        self.intraday_items
            .iter()
            .map(|item| item.earliest_review_at)
            .min()
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
            .filter(|(_, item)| item.earliest_review_at <= now)
            .min_by_key(|(_, item)| (item.earliest_review_at, item.due_after_seen))
            .map(|(index, _)| index)
    }

    fn closest_intraday_index(&self) -> usize {
        self.intraday_items
            .iter()
            .enumerate()
            .min_by_key(|(_, item)| (item.earliest_review_at, item.due_after_seen))
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

pub(crate) fn restored_session_item(
    progress: LearningProgress,
    entry: KnowledgeEntry,
    first_rating: LearningRecallRating,
    appearance_count_today: i32,
    success_streak: i32,
    next_review_at: DateTime<Utc>,
) -> LearningSessionItem {
    LearningSessionItem {
        progress,
        entry,
        phase: LearningPhase::Intraday,
        first_rating: Some(first_rating),
        appearance_count_today,
        success_streak,
        due_after_seen: 0,
        earliest_review_at: next_review_at,
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

fn min_delay_for_item(item: &LearningSessionItem, rating: LearningRecallRating) -> Duration {
    if item.first_rating == Some(LearningRecallRating::Forgotten)
        && rating == LearningRecallRating::Known
    {
        return Duration::seconds(INTRADAY_RECOVERY_KNOWN_DELAY_SECONDS);
    }

    match rating {
        LearningRecallRating::Forgotten => Duration::seconds(INTRADAY_FORGOTTEN_DELAY_SECONDS),
        LearningRecallRating::Fuzzy => Duration::seconds(INTRADAY_FUZZY_DELAY_SECONDS),
        LearningRecallRating::Known => Duration::seconds(INTRADAY_KNOWN_DELAY_SECONDS),
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

    #[test]
    fn time_is_primary_over_card_gap() {
        let now = Utc::now();
        let mut item = empty_item(LearningPhase::Intraday);
        item.due_after_seen = 100;
        item.earliest_review_at = now - Duration::seconds(1);

        let mut session = LearningSessionRuntime::new(
            "test".to_string(),
            NaiveDate::from_ymd_opt(2026, 9, 18).unwrap(),
            VecDeque::new(),
            VecDeque::new(),
            vec![item].into(),
        );
        session.select_next(now);

        assert!(session.current.is_some());
    }

    #[test]
    fn workset_target_uses_count_fallback_before_new_cards() {
        let now = Utc::now();
        let intraday_items = (0..ACTIVE_INTRADAY_WORKSET_TARGET)
            .map(|_| {
                let mut item = empty_item(LearningPhase::Intraday);
                item.earliest_review_at = now + Duration::minutes(5);
                item
            })
            .collect::<Vec<_>>();
        let mut session = LearningSessionRuntime::new(
            "test".to_string(),
            NaiveDate::from_ymd_opt(2026, 9, 18).unwrap(),
            VecDeque::new(),
            vec![empty_item(LearningPhase::New)].into(),
            intraday_items.into(),
        );
        session.select_next(now);

        assert_eq!(
            session.current.as_ref().map(|item| item.phase),
            Some(LearningPhase::Intraday)
        );
    }

    #[test]
    fn waits_when_intraday_card_is_not_due_and_workset_has_room() {
        let now = Utc::now();
        let mut item = empty_item(LearningPhase::Intraday);
        item.earliest_review_at = now + Duration::minutes(5);
        let mut session = LearningSessionRuntime::new(
            "test".to_string(),
            NaiveDate::from_ymd_opt(2026, 9, 18).unwrap(),
            VecDeque::new(),
            VecDeque::new(),
            vec![item].into(),
        );
        session.select_next(now);

        assert!(session.current.is_none());
        assert!(!session.is_completed());
        assert!(session.next_available_at().is_some());
    }

    #[test]
    fn forgotten_then_known_uses_longer_cooldown() {
        let mut item = empty_item(LearningPhase::Intraday);
        item.first_rating = Some(LearningRecallRating::Forgotten);

        assert_eq!(
            min_delay_for_item(&item, LearningRecallRating::Known),
            Duration::minutes(5)
        );
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
