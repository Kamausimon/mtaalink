-- One wallet per provider/business tracks running balance
CREATE TABLE IF NOT EXISTS wallets (
    id             SERIAL PRIMARY KEY,
    target_type    VARCHAR(50) NOT NULL CHECK (target_type IN ('provider', 'business')),
    target_id      INTEGER NOT NULL,
    balance        NUMERIC(12, 2) NOT NULL DEFAULT 0.00,
    total_earned   NUMERIC(12, 2) NOT NULL DEFAULT 0.00,
    total_paid_out NUMERIC(12, 2) NOT NULL DEFAULT 0.00,
    created_at     TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at     TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (target_type, target_id)
);

-- Every credit or payout is recorded here (immutable ledger)
CREATE TABLE IF NOT EXISTS wallet_transactions (
    id          SERIAL PRIMARY KEY,
    wallet_id   INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    txn_type    VARCHAR(20) NOT NULL CHECK (txn_type IN ('credit', 'payout')),
    amount      NUMERIC(12, 2) NOT NULL,
    description TEXT NOT NULL,
    booking_id  INTEGER REFERENCES bookings(id) ON DELETE SET NULL,
    payment_id  INTEGER REFERENCES payments(id) ON DELETE SET NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_wallet_transactions_wallet_id
    ON wallet_transactions (wallet_id);

-- Provider/business requests withdrawal to their M-Pesa number
CREATE TABLE IF NOT EXISTS payout_requests (
    id           SERIAL PRIMARY KEY,
    wallet_id    INTEGER NOT NULL REFERENCES wallets(id) ON DELETE CASCADE,
    amount       NUMERIC(12, 2) NOT NULL,
    phone_number TEXT NOT NULL,
    status       VARCHAR(20) NOT NULL DEFAULT 'pending'
                     CHECK (status IN ('pending', 'approved', 'rejected', 'paid')),
    notes        TEXT,
    created_at   TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at   TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_payout_requests_wallet_id
    ON payout_requests (wallet_id);
