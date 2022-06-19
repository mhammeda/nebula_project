CREATE TABLE IF NOT EXISTS posts (
    id UUID NOT NULL PRIMARY KEY,

    community VARCHAR(24) NOT NULL,
    parent UUID,
    author_username VARCHAR(24) NOT NULL,
    author_host VARCHAR(259) NOT NULL,

    title VARCHAR(100) NOT NULL,
    content JSONB NOT NULL,
    created BIGINT NOT NULL,
    modified BIGINT NOT NULL,

    FOREIGN KEY (community) REFERENCES communities(id) ON DELETE CASCADE,
    FOREIGN KEY (parent) REFERENCES posts(id) ON DELETE CASCADE,
    FOREIGN KEY (author_username, author_host) REFERENCES users(username, host) ON DELETE CASCADE
);
