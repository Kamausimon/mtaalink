-- Make category_id nullable on services — the backend struct already treats it as Option<i32>
-- and the UI doesn't require a category when creating a service.
ALTER TABLE services ALTER COLUMN category_id DROP NOT NULL;
