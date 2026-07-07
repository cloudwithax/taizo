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

-- poll votes
CREATE TABLE IF NOT EXISTS poll_votes (
    message_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    choice_index INT NOT NULL,
    PRIMARY KEY (message_id, user_id)
);

-- polls
CREATE TABLE IF NOT EXISTS polls (
    message_id BIGINT PRIMARY KEY,
    channel_id BIGINT NOT NULL,
    user_id BIGINT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);

-- honeypot channels (auto-ban anyone who chats)
CREATE TABLE IF NOT EXISTS honeypots (
    guild_id BIGINT PRIMARY KEY,
    channel_id BIGINT NOT NULL,
    rotate_daily BOOLEAN NOT NULL DEFAULT false,
    last_rotated DATE
);

-- reaction role messages
CREATE TABLE IF NOT EXISTS reaction_roles (
    message_id BIGINT PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL,
    mode TEXT NOT NULL DEFAULT 'normal',
    max_roles INT,
    role_duration INT,
    created_by BIGINT NOT NULL,
    title TEXT NOT NULL DEFAULT 'reaction roles',
    description TEXT
);

-- emoji-role pairs for reaction roles
CREATE TABLE IF NOT EXISTS reaction_role_pairs (
    id SERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES reaction_roles(message_id) ON DELETE CASCADE,
    emoji TEXT NOT NULL,
    role_id BIGINT NOT NULL
);

-- ticket configuration per guild
CREATE TABLE IF NOT EXISTS ticket_config (
    guild_id BIGINT PRIMARY KEY,
    category_id BIGINT NOT NULL,
    support_role_id BIGINT NOT NULL,
    panel_channel_id BIGINT,
    panel_message_id BIGINT,
    log_channel_id BIGINT,
    allow_user_close BOOLEAN NOT NULL DEFAULT true,
    close_action TEXT NOT NULL DEFAULT 'delete'
);

-- tickets
CREATE TABLE IF NOT EXISTS tickets (
    id SERIAL PRIMARY KEY,
    guild_id BIGINT NOT NULL,
    channel_id BIGINT NOT NULL UNIQUE,
    creator_id BIGINT NOT NULL,
    number INT NOT NULL,
    status TEXT NOT NULL DEFAULT 'open',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at TIMESTAMPTZ,
    closed_by BIGINT,
    UNIQUE(guild_id, number)
);

-- track user role assignments (for unique, limit, temp, verify modes)
CREATE TABLE IF NOT EXISTS reaction_role_users (
    id SERIAL PRIMARY KEY,
    message_id BIGINT NOT NULL REFERENCES reaction_roles(message_id) ON DELETE CASCADE,
    user_id BIGINT NOT NULL,
    role_id BIGINT NOT NULL,
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    UNIQUE(message_id, user_id, role_id)
);
