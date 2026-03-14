-- Create the message type enum
CREATE TYPE message_type AS ENUM ('Chat', 'Join');

-- Add the column to messages table
ALTER TABLE messages ADD COLUMN msg_type message_type NOT NULL DEFAULT 'Chat';
