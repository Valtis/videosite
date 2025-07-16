CREATE VIEW active_uploads AS
SELECT 
    user_id, 
    resource_id,
    file_size,
    created_at,
    updated_at
FROM user_uploads
WHERE 
    deleted_at IS NULL;