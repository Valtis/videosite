
CREATE TABLE app_resource (
    id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id uuid NOT NULL,
    is_public boolean NOT NULL DEFAULT false, 
    resource_name VARCHAR(255) NOT NULL,
    resource_type VARCHAR(255) NOT NULL,
    resource_status VARCHAR(255) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    deleted_at TIMESTAMP WITH TIME ZONE
);