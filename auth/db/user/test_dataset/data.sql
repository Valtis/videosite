-- OBVIOUSLY, ONLY FOR USE IN DEVELOPMENT ENVIRONMENTS
-- TRUNCATING REAL DATABASE IS NOT EXACTLY A GOOD IDEA
TRUNCATE TABLE app_user;

INSERT INTO app_user (
    id,
    email,
    display_name,
    password_hash,
    created_at,
    updated_at,
    deleted_at
) VALUES (
    'f47ac10b-58cc-4372-a567-0e02b2c3d479',
    'test_user@localhost',
    'Test User',
    -- 'password'
    '$scrypt$ln=17,r=8,p=1$/Oy6Vf7OXfUQnHTc5u0b5A$i8TF7kkx6s3TllEIXHN7/O2UNP7CYaLPhoflvkNI8Cg',
    now(),
    now(),
    NULL
);

INSERT INTO app_user (
    id,
    email,
    display_name,
    password_hash,
    created_at,
    updated_at,
    deleted_at
) VALUES (
    'abcdef01-2345-6789-abcd-ef0123456789',
    'test_user2@localhost',
    'Test User 2',
    -- 'password'
    '$scrypt$ln=17,r=8,p=1$/Oy6Vf7OXfUQnHTc5u0b5A$i8TF7kkx6s3TllEIXHN7/O2UNP7CYaLPhoflvkNI8Cg',
    now(),
    now(),
    now()
);
