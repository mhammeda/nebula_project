CREATE TABLE IF NOT EXISTS communities (
    id VARCHAR(24) NOT NULL PRIMARY KEY,

    title VARCHAR(100) NOT NULL,
    description TEXT NOT NULL,
    created BIGINT NOT NULL
);
