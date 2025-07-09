CREATE VIEW active_users AS
SELECT 
    id,
    email,
    display_name,
    password_hash,
    created_at,
    updated_at
FROM app_user
WHERE deleted_at IS NULL;