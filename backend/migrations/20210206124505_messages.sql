CREATE TABLE IF NOT EXISTS messages (
    id UUID NOT NULL PRIMARY KEY,

    sender_username VARCHAR(24) NOT NULL,
    sender_host VARCHAR(259) NOT NULL,

    receiver_username VARCHAR(24) NOT NULL,
    receiver_host VARCHAR(259) NOT NULL,

    title TEXT NOT NULL,
    content JSONB NOT NULL,
    timestamp BIGINT NOT NULL,
    read BOOLEAN NOT NULL,

    CHECK (NOT((sender_username = receiver_username) AND (sender_host = receiver_host))),

    FOREIGN KEY (sender_username, sender_host) REFERENCES users(username, host) ON DELETE CASCADE,
    FOREIGN KEY (receiver_username, receiver_host) REFERENCES users(username, host) ON DELETE CASCADE
);
