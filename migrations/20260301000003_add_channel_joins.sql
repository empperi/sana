-- Create channel_joins table
CREATE TABLE IF NOT EXISTS channel_joins (
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    channel_id UUID NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    PRIMARY KEY (user_id, channel_id)
);

-- Index for faster lookups of channels for a user
CREATE INDEX IF NOT EXISTS idx_channel_joins_user_id ON channel_joins(user_id);

-- Ensure all existing users are joined to #General (00000000-0000-0000-0000-000000000001)
INSERT INTO channel_joins (user_id, channel_id)
SELECT id, '00000000-0000-0000-0000-000000000001'::UUID FROM users
ON CONFLICT DO NOTHING;
