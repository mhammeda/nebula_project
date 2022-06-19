CREATE TABLE IF NOT EXISTS images (
    id UUID NOT NULL PRIMARY KEY,
    content BYTEA NOT NULL,
    author_username VARCHAR(24) NOT NULL,
    author_host VARCHAR(259) NOT NULL,
    FOREIGN KEY (author_username, author_host) REFERENCES users(username, host) ON DELETE CASCADE
);
