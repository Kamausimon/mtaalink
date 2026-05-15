-- Tracks whether a 24-hour reminder has been sent for this booking.
-- The background reminder task sets this to true after sending.
ALTER TABLE bookings
    ADD COLUMN IF NOT EXISTS reminder_sent BOOLEAN NOT NULL DEFAULT false;

-- Index used by the reminder query (bookings in the upcoming 23-25h window)
CREATE INDEX IF NOT EXISTS idx_bookings_reminder
    ON bookings (scheduled_time)
    WHERE reminder_sent = false
      AND status NOT IN ('cancelled', 'completed');
