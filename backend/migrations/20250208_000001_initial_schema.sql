-- De-AI-Hilfer Database Schema v3.0
-- Modern architecture with JSONB, arrays, and FSRS integration
-- Created: 2025-02-08

-- 0. 启用向量扩展 (用于未来的 RAG/Search)
CREATE EXTENSION IF NOT EXISTS vector;

-- ==========================================
-- 1. 权威数据层 (The Truth Layer)
-- 来源: Kaikki.org JSONL
-- ==========================================
CREATE TABLE dictionary_raw (
    -- 使用原型词作为主键 (e.g., "gehen")
    headword TEXT PRIMARY KEY,
    
    -- 存储完整的清洗后 JSON 数据 (含变位、释义、例句)
    -- Rust 结构体对应: serde_json::Value
    raw_data JSONB NOT NULL,
    
    -- 标记是否已关联本地音频文件
    has_audio BOOLEAN DEFAULT FALSE,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- 建立 GIN 索引以加速 JSON 内部查询 (如查询变位 forms)
CREATE INDEX idx_dictionary_raw_data ON dictionary_raw USING GIN (raw_data);


-- ==========================================
-- 2. 用户知识层 (The User Layer)
-- 来源: User Query + AI Analysis
-- ==========================================
CREATE TABLE knowledge_entries (
    id BIGSERIAL PRIMARY KEY,
    
    -- 用户实际查询的文本 (e.g., "ging")
    query_text TEXT NOT NULL,
    
    -- 关联的权威原型 (e.g., "gehen")
    -- 如果是生僻词没查到字典，此字段可为 NULL
    prototype TEXT REFERENCES dictionary_raw(headword),
    
    -- 条目类型: WORD, PHRASE, PREFIX, SUFFIX
    entry_type TEXT NOT NULL DEFAULT 'WORD',
    
    -- 核心：AI 生成的结构化分析 (JSONB)
    -- 包含: definition, examples, etymology, etc.
    -- 替代了旧版的 analysis_markdown 纯文本字段
    analysis JSONB NOT NULL,
    
    -- 辅助检索的标签与别名
    tags TEXT[],       -- e.g. ['Verb', 'B1', 'Starkes Verb']
    aliases TEXT[],    -- e.g. ['ging', 'gegangen']
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_knowledge_entries_query ON knowledge_entries(query_text);
CREATE INDEX idx_knowledge_entries_aliases ON knowledge_entries USING GIN(aliases);


-- ==========================================
-- 3. 追问记录 (Legacy Feature)
-- 用于存储针对某个条目的后续对话
-- ==========================================
CREATE TABLE follow_ups (
    id BIGSERIAL PRIMARY KEY,
    entry_id BIGINT NOT NULL REFERENCES knowledge_entries(id) ON DELETE CASCADE,
    
    question TEXT NOT NULL,
    answer TEXT NOT NULL,
    
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_follow_ups_entry_id ON follow_ups(entry_id);


-- ==========================================
-- 4. 学习进度层 (The Memory Layer)
-- 算法: FSRS (Free Spaced Repetition Scheduler)
-- ==========================================
CREATE TABLE learning_progress (
    -- 1:1 关联 knowledge_entries
    entry_id BIGINT PRIMARY KEY REFERENCES knowledge_entries(id) ON DELETE CASCADE,
    
    -- FSRS 核心参数 (对应 fsrs crate 的 Card 结构体)
    stability REAL NOT NULL DEFAULT 0,
    difficulty REAL NOT NULL DEFAULT 0,
    elapsed_days BIGINT NOT NULL DEFAULT 0,
    scheduled_days BIGINT NOT NULL DEFAULT 0,
    state INTEGER NOT NULL DEFAULT 0, -- 0:New, 1:Learning, 2:Review, 3:Relearning
    
    -- 复习时间管理
    last_review_at TIMESTAMPTZ,
    due_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    
    review_count INTEGER NOT NULL DEFAULT 0
);

CREATE INDEX idx_learning_progress_due ON learning_progress(due_date);


-- ==========================================
-- 5. 触发器：自动更新 updated_at
-- ==========================================
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

CREATE TRIGGER update_knowledge_entries_updated_at 
    BEFORE UPDATE ON knowledge_entries
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();
