-- Rename the table
ALTER TABLE channel_joins RENAME TO user_channels;

-- Rename the index
ALTER INDEX IF EXISTS idx_channel_joins_user_id RENAME TO idx_user_channels_user_id;

-- Add the nullable read marker column
ALTER TABLE user_channels ADD COLUMN last_message_read UUID REFERENCES messages(id) ON DELETE SET NULL;
