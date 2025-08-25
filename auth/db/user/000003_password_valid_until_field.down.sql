-- Restore the previous state of the active_users view
DROP VIEW IF EXISTS active_users;
CREATE active_users AS
SELECT 
    id,
    email,
    display_name,
    password_hash,
    created_at,
    updated_at
FROM app_user
WHERE deleted_at IS NULL;

-- remove the column from the table
ALTER TABLE app_user
DROP COLUMN password_valid_until;