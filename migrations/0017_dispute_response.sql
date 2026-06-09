-- Provider can submit their side of the dispute before admin rules
ALTER TABLE bookings ADD COLUMN IF NOT EXISTS dispute_response TEXT;
