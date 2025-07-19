CREATE TABLE audit_event (
    id SERIAL PRIMARY KEY,
    user_id uuid NULL,
    client_ip VARCHAR(255) NOT NULL,
    event_action VARCHAR(255) NOT NULL,
    action_target uuid NULL,
    additional_info jsonb,
    event_timestamp TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT now()
);