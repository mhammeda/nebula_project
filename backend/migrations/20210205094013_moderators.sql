CREATE TABLE IF NOT EXISTS moderators (
    username VARCHAR(24) NOT NULL,
    host VARCHAR(259) NOT NULL,
    community VARCHAR(24) NOT NULL,

    FOREIGN KEY (username, host) REFERENCES users(username, host) ON DELETE CASCADE,
    FOREIGN KEY (community) REFERENCES communities(id) ON DELETE CASCADE,

    PRIMARY KEY(username, host, community)
);
