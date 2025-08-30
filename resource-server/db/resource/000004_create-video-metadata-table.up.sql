CREATE TABLE video_metadata (
    resource_id UUID REFERENCES app_resource(id) ON DELETE CASCADE,
    width INTEGER NOT NULL,
    height INTEGER NOT NULL,
    duration_seconds REAL NOT NULL,
    bit_rate INTEGER NOT NULL,
    frame_rate REAL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (resource_id, width, height)
);