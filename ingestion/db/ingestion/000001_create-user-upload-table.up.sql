CREATE TABLE user_uploads (
    user_id uuid NOT NULL,
    resource_id uuid NOT NULL,
    file_size BIGINT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    deleted_at TIMESTAMP WITH TIME ZONE NULL,
    PRIMARY KEY (user_id, resource_id)
);