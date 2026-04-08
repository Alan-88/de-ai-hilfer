ALTER TABLE dictionary_learning_order
    ADD COLUMN lexeme_id BIGINT REFERENCES dictionary_lexemes(id) ON DELETE SET NULL;

CREATE INDEX idx_dictionary_learning_order_lexeme_id
    ON dictionary_learning_order(lexeme_id);

WITH exact_candidates AS (
    SELECT
        dlo.headword,
        dl.id AS lexeme_id,
        row_number() OVER (
            PARTITION BY dlo.headword
            ORDER BY dl.id ASC
        ) AS rn,
        count(*) OVER (
            PARTITION BY dlo.headword
        ) AS candidate_count
    FROM dictionary_learning_order dlo
    JOIN dictionary_lexemes dl
      ON dl.surface = dlo.headword
    WHERE dlo.lexeme_id IS NULL
),
case_insensitive_candidates AS (
    SELECT
        dlo.headword,
        dl.id AS lexeme_id,
        row_number() OVER (
            PARTITION BY dlo.headword
            ORDER BY
                CASE WHEN dl.surface = dlo.headword THEN 0 ELSE 1 END,
                dl.id ASC
        ) AS rn,
        count(*) OVER (
            PARTITION BY dlo.headword
        ) AS candidate_count
    FROM dictionary_learning_order dlo
    JOIN dictionary_lexemes dl
      ON lower(dl.surface) = lower(dlo.headword)
    WHERE dlo.lexeme_id IS NULL
      AND NOT EXISTS (
          SELECT 1
          FROM exact_candidates ec
          WHERE ec.headword = dlo.headword
      )
)
UPDATE dictionary_learning_order dlo
SET lexeme_id = resolved.lexeme_id
FROM (
    SELECT headword, lexeme_id
    FROM exact_candidates
    WHERE candidate_count = 1
      AND rn = 1

    UNION ALL

    SELECT headword, lexeme_id
    FROM case_insensitive_candidates
    WHERE candidate_count = 1
      AND rn = 1
) AS resolved
WHERE dlo.headword = resolved.headword;
