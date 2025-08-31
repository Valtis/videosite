CREATE TABLE chunk_upload(
    object_name UUID PRIMARY KEY,
    aws_upload_id TEXT NOT NULL,
    user_id UUID NOT NULL,
    chunk_size BIGINT NOT NULL,
    file_name TEXT NOT NULL,
    file_integrity_algorithm TEXT,
    file_integrity_hash TEXT,
    received_bytes BIGINT DEFAULT 0, -- used to track quota usage
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    completed_at TIMESTAMPTZ
);

CREATE VIEW active_chunk_upload AS
    SELECT * FROM chunk_upload WHERE completed_at IS NULL;