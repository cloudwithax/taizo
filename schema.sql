-- economy system
CREATE TABLE IF NOT EXISTS economy (
    user_id BIGINT PRIMARY KEY,
    wallet BIGINT NOT NULL DEFAULT 0,
    bank BIGINT NOT NULL DEFAULT 0,
    worth BIGINT NOT NULL DEFAULT 0
);

-- warnings
CREATE TABLE IF NOT EXISTS warnings (
    id SERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    moderator_id BIGINT NOT NULL,
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- welcome messages per guild
CREATE TABLE IF NOT EXISTS welcome (
    guild_id BIGINT PRIMARY KEY,
    channel_id BIGINT NOT NULL,
    message TEXT NOT NULL DEFAULT 'welcome [mention] to [server]!'
);

-- leave messages per guild
CREATE TABLE IF NOT EXISTS leave (
    guild_id BIGINT PRIMARY KEY,
    channel_id BIGINT NOT NULL,
    message TEXT NOT NULL DEFAULT 'goodbye [user]!'
);
