ALTER TABLE providers ADD COLUMN IF NOT EXISTS onboarding_completed BOOLEAN NOT NULL DEFAULT FALSE;
ALTER TABLE businesses ADD COLUMN IF NOT EXISTS onboarding_completed BOOLEAN NOT NULL DEFAULT FALSE;

-- Backfill: registration never sets service_name/description for providers, or
-- description for businesses — only the onboarding flow does. Any row that
-- already has these filled in was onboarded before this column existed.
UPDATE providers SET onboarding_completed = TRUE WHERE service_name IS NOT NULL;
UPDATE businesses SET onboarding_completed = TRUE WHERE description IS NOT NULL;
