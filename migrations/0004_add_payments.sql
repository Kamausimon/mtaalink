-- Tracks M-Pesa STK Push transactions tied to bookings
CREATE TABLE IF NOT EXISTS payments (
    id                   SERIAL PRIMARY KEY,
    booking_id           INTEGER REFERENCES bookings(id) ON DELETE CASCADE,
    phone_number         TEXT NOT NULL,
    amount               NUMERIC(10, 2) NOT NULL,
    checkout_request_id  TEXT UNIQUE,          -- M-Pesa STK push request identifier
    merchant_request_id  TEXT,
    transaction_id       TEXT,                 -- M-Pesa receipt number (set after success)
    status               TEXT NOT NULL DEFAULT 'pending'
                             CHECK (status IN ('pending', 'completed', 'failed', 'cancelled')),
    result_code          INTEGER,
    result_desc          TEXT,
    created_at           TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at           TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_payments_booking_id ON payments (booking_id);
CREATE INDEX IF NOT EXISTS idx_payments_checkout_request_id ON payments (checkout_request_id);
