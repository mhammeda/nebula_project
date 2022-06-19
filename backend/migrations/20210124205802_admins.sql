CREATE TABLE IF NOT EXISTS admins (
    username VARCHAR(24) NOT NULL,
    host VARCHAR(259) NOT NULL,

    PRIMARY KEY (username, host),
    FOREIGN KEY (username, host) REFERENCES users(username, host) ON DELETE CASCADE
);
