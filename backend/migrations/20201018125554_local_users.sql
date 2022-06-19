CREATE TABLE IF NOT EXISTS local_users (
    username VARCHAR(24) NOT NULL,
    host VARCHAR(259) NOT NULL,

    hash VARCHAR(128) NOT NULL, -- Argon2 hash roughly ~90 chars but dependant on config
    recovery_hash VARCHAR(128) NOT NULL,
    created BIGINT NOT NULL,
    session BYTEA,

    PRIMARY KEY (username, host),
    FOREIGN KEY (username, host) REFERENCES users(username, host) ON DELETE CASCADE
);
