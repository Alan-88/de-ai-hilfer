ALTER TABLE knowledge_entries
    ADD COLUMN lexeme_id BIGINT REFERENCES dictionary_lexemes(id) ON DELETE SET NULL;

CREATE INDEX idx_knowledge_entries_lexeme_id
    ON knowledge_entries(lexeme_id);

WITH exact_candidates AS (
    SELECT
        ke.id AS knowledge_id,
        dl.id AS lexeme_id,
        row_number() OVER (
            PARTITION BY ke.id
            ORDER BY dl.id ASC
        ) AS rn,
        count(*) OVER (
            PARTITION BY ke.id
        ) AS candidate_count
    FROM knowledge_entries ke
    JOIN dictionary_lexemes dl
      ON dl.surface = COALESCE(ke.prototype, ke.query_text)
    WHERE ke.lexeme_id IS NULL
      AND ke.entry_type <> 'PHRASE'
),
case_insensitive_candidates AS (
    SELECT
        ke.id AS knowledge_id,
        dl.id AS lexeme_id,
        row_number() OVER (
            PARTITION BY ke.id
            ORDER BY
                CASE WHEN dl.surface = COALESCE(ke.prototype, ke.query_text) THEN 0 ELSE 1 END,
                dl.id ASC
        ) AS rn,
        count(*) OVER (
            PARTITION BY ke.id
        ) AS candidate_count
    FROM knowledge_entries ke
    JOIN dictionary_lexemes dl
      ON lower(dl.surface) = lower(COALESCE(ke.prototype, ke.query_text))
    WHERE ke.lexeme_id IS NULL
      AND ke.entry_type <> 'PHRASE'
      AND NOT EXISTS (
          SELECT 1
          FROM exact_candidates ec
          WHERE ec.knowledge_id = ke.id
      )
)
UPDATE knowledge_entries ke
SET lexeme_id = resolved.lexeme_id
FROM (
    SELECT knowledge_id, lexeme_id
    FROM exact_candidates
    WHERE candidate_count = 1
      AND rn = 1

    UNION ALL

    SELECT knowledge_id, lexeme_id
    FROM case_insensitive_candidates
    WHERE candidate_count = 1
      AND rn = 1
) AS resolved
WHERE ke.id = resolved.knowledge_id;
