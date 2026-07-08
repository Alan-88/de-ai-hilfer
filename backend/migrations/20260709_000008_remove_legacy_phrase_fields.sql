UPDATE knowledge_entries
SET analysis = analysis - 'phrase_lookup' - 'phrase_usage_preview'
WHERE analysis ? 'phrase_lookup'
   OR analysis ? 'phrase_usage_preview';
