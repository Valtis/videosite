TRUNCATE TABLE user_quota;
TRUNCATE TABLE user_uploads;

INSERT INTO user_quota (
    user_id, 
    upload_quota, 
    created_at, 
    updated_at
) VALUES (
    'f47ac10b-58cc-4372-a567-0e02b2c3d479', 
    5368709120, 
    NOW(), 
    NOW()
);

