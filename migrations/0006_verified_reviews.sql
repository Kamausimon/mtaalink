-- Link each review to the completed booking that earned it.
-- NULL means the review predates this migration (legacy data).
ALTER TABLE reviews
    ADD COLUMN IF NOT EXISTS verified_booking_id INTEGER
        REFERENCES bookings(id) ON DELETE SET NULL;
