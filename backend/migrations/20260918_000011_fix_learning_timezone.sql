-- The learning engine previously used the container's Etc/UTC `Local` timezone.
-- Its intended business-day boundary is 04:00 in UTC+8.

UPDATE learning_progress
SET due_date = due_date - INTERVAL '8 hours'
WHERE review_count > 0
  AND last_review_at IS NOT NULL
  AND EXTRACT(HOUR FROM due_date AT TIME ZONE 'UTC') = 4
  AND EXTRACT(MINUTE FROM due_date AT TIME ZONE 'UTC') = 0
  AND EXTRACT(SECOND FROM due_date AT TIME ZONE 'UTC') = 0;

UPDATE learning_review_logs
SET business_date = (created_at AT TIME ZONE 'Asia/Shanghai')::date;
