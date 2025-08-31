CREATE TABLE chunk_information(
    object_name UUID NOT NULL,
    part_number INT NOT NULL,
    e_tag TEXT NOT NULL,
    user_id UUID NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    PRIMARY KEY (object_name, part_number)
);