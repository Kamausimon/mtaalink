-- Expand booking status to support the two-step completion flow:
-- pending_confirmation: provider has marked done, awaiting client confirmation
-- disputed: client has raised a dispute against the completion claim
ALTER TABLE bookings DROP CONSTRAINT IF EXISTS bookings_status_check;
ALTER TABLE bookings
    ADD CONSTRAINT bookings_status_check
    CHECK (status IN ('pending', 'confirmed', 'cancelled', 'completed',
                      'pending_confirmation', 'disputed'));

-- Store the dispute reason separately from cancellation reason
ALTER TABLE bookings ADD COLUMN IF NOT EXISTS dispute_reason TEXT;
