CREATE TABLE IF NOT EXISTS users (
    username VARCHAR(24) NOT NULL,
    -- domain names have a max length of 253 characters + 6 characters for colon and port
    host VARCHAR(259) NOT NULL,
    PRIMARY KEY (username, host)
);
