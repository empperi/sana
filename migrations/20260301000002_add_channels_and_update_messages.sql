-- Rename user_id to id in users table for consistency
ALTER TABLE users RENAME COLUMN user_id TO id;

-- Create channels table
CREATE TABLE IF NOT EXISTS channels (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL UNIQUE,
    is_private BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Drop existing messages table to recreate with new schema
DROP TABLE IF EXISTS messages;

-- Recreate messages table with UUIDs and foreign keys
CREATE TABLE messages (
    id UUID PRIMARY KEY,
    channel_id UUID NOT NULL REFERENCES channels(id),
    user_id UUID NOT NULL REFERENCES users(id),
    seq BIGINT NOT NULL UNIQUE,
    content TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL
);
