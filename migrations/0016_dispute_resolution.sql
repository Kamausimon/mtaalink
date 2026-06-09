-- Allow admin to store a resolution note on disputed bookings
ALTER TABLE bookings ADD COLUMN IF NOT EXISTS admin_resolution TEXT;
