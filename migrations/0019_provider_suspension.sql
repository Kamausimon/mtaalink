-- Allow admin to suspend providers and businesses
ALTER TABLE providers  ADD COLUMN IF NOT EXISTS suspended_until TIMESTAMPTZ;
ALTER TABLE businesses ADD COLUMN IF NOT EXISTS suspended_until TIMESTAMPTZ;
