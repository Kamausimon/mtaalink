-- Content moderation: users can flag inappropriate reviews, posts, or profiles
CREATE TABLE IF NOT EXISTS content_flags (
    id          SERIAL PRIMARY KEY,
    target_type VARCHAR(50) NOT NULL,
    target_id   INTEGER NOT NULL,
    reason      TEXT NOT NULL,
    flagged_by  INTEGER REFERENCES users(id) ON DELETE SET NULL,
    resolved    BOOLEAN NOT NULL DEFAULT FALSE,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_content_flags_target
    ON content_flags (target_type, target_id);
