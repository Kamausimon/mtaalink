CREATE TABLE IF NOT EXISTS dispute_evidence (
    id            SERIAL PRIMARY KEY,
    booking_id    INTEGER NOT NULL REFERENCES bookings(id) ON DELETE CASCADE,
    uploaded_by   INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    uploader_role TEXT    NOT NULL CHECK (uploader_role IN ('client', 'provider')),
    file_url      TEXT    NOT NULL,
    caption       TEXT,
    created_at    TIMESTAMP WITHOUT TIME ZONE DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_dispute_evidence_booking ON dispute_evidence(booking_id);
