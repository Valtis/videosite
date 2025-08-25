-- alter the table
ALTER TABLE app_user
ADD COLUMN password_valid_until TIMESTAMPTZ;

-- update the active_users view to include the new field
DROP VIEW IF EXISTS active_users;
CREATE VIEW active_users AS
SELECT 
    id,
    email,
    display_name,
    password_hash,
    password_valid_until,
    created_at,
    updated_at
FROM app_user
WHERE deleted_at IS NULL AND (password_valid_until IS NULL OR password_valid_until > NOW()); 


