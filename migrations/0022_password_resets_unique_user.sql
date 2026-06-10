-- Ensure ON CONFLICT (user_id) upserts in forgot_password work correctly
DELETE FROM password_resets a USING password_resets b
    WHERE a.id < b.id AND a.user_id = b.user_id;

ALTER TABLE password_resets ADD CONSTRAINT password_resets_user_id_key UNIQUE (user_id);
