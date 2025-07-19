CREATE TABLE transfer_quota (
    id SERIAL PRIMARY KEY,
    quota_used BIGINT DEFAULT 0,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);