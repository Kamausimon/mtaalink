-- Allows providers and businesses to reply to reviews left on their profiles
CREATE TABLE IF NOT EXISTS review_replies (
    id          SERIAL PRIMARY KEY,
    review_id   INTEGER NOT NULL REFERENCES reviews(id) ON DELETE CASCADE,
    reviewer_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    comment     TEXT NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
);
