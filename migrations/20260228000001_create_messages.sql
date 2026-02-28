CREATE TABLE messages (
    id VARCHAR PRIMARY KEY,
    channel VARCHAR NOT NULL,
    seq BIGINT NOT NULL UNIQUE,
    username VARCHAR NOT NULL,
    content TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL
);