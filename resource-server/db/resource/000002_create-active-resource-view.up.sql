CREATE VIEW active_resources AS
SELECT 
    id, 
    user_id,
    is_public,
    resource_name,
    resource_type,
    resource_status,
    origin_file_path,
    base_directory,
    created_at,
    updated_at
FROM app_resource
WHERE 
    deleted_at IS NULL;