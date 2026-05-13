-- Initial schema for mtaalink
-- Reflects the live database as of project inception

-- Location hierarchy (no foreign key dependencies)
CREATE TABLE IF NOT EXISTS counties (
    id   SERIAL PRIMARY KEY,
    name TEXT NOT NULL UNIQUE
);

CREATE TABLE IF NOT EXISTS constituencies (
    id        SERIAL PRIMARY KEY,
    name      TEXT NOT NULL,
    county_id INTEGER NOT NULL REFERENCES counties(id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS wards (
    id               SERIAL PRIMARY KEY,
    name             TEXT NOT NULL,
    constituency_id  INTEGER NOT NULL REFERENCES constituencies(id) ON DELETE CASCADE
);

-- Core user table
CREATE TABLE IF NOT EXISTS users (
    id         SERIAL PRIMARY KEY,
    username   TEXT NOT NULL UNIQUE,
    email      TEXT NOT NULL UNIQUE,
    password   TEXT NOT NULL,
    role       TEXT CHECK (role IN ('client', 'provider', 'business')),
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT now()
);

-- Role-specific profile tables
CREATE TABLE IF NOT EXISTS clients (
    id              SERIAL PRIMARY KEY,
    user_id         INTEGER NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    profile_picture TEXT
);

CREATE TABLE IF NOT EXISTS providers (
    id                  SERIAL PRIMARY KEY,
    user_id             INTEGER NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    service_description TEXT,
    service_name        TEXT,
    category            TEXT,
    location            TEXT,
    phone_number        TEXT,
    email               TEXT,
    website             TEXT,
    whatsapp            TEXT,
    profile_photo       TEXT,
    cover_photo         TEXT,
    approved            BOOLEAN DEFAULT false,
    created_at          TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS businesses (
    id             SERIAL PRIMARY KEY,
    user_id        INTEGER NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    business_name  TEXT NOT NULL,
    description    TEXT,
    category       TEXT,
    location       TEXT,
    license_number TEXT,
    krapin         TEXT UNIQUE,
    phone_number   TEXT,
    email          TEXT,
    website        TEXT,
    whatsapp       TEXT,
    profile_photo  TEXT,
    cover_photo    TEXT,
    logo           TEXT,
    verified       BOOLEAN DEFAULT false,
    created_at     TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Admin table
CREATE TABLE IF NOT EXISTS admins (
    id             SERIAL PRIMARY KEY,
    user_id        INTEGER NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
    is_super_admin BOOLEAN DEFAULT false,
    created_at     TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Categories (self-referencing for parent/child hierarchy)
CREATE TABLE IF NOT EXISTS categories (
    id         SERIAL PRIMARY KEY,
    name       TEXT NOT NULL UNIQUE,
    parent_id  INTEGER REFERENCES categories(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Junction tables for categories
CREATE TABLE IF NOT EXISTS provider_categories (
    provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (provider_id, category_id)
);

CREATE TABLE IF NOT EXISTS business_categories (
    business_id INTEGER NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    PRIMARY KEY (business_id, category_id)
);

-- Provider availability schedule
CREATE TABLE IF NOT EXISTS provider_availability (
    id          SERIAL PRIMARY KEY,
    provider_id INTEGER REFERENCES providers(id),
    day         VARCHAR(10) NOT NULL,
    start_time  TIME WITHOUT TIME ZONE NOT NULL,
    end_time    TIME WITHOUT TIME ZONE NOT NULL,
    is_available BOOLEAN
);

COMMENT ON TABLE provider_availability IS 'Stores the availability schedule for service providers by day of week';

-- Location tables
CREATE TABLE IF NOT EXISTS provider_locations (
    id          SERIAL PRIMARY KEY,
    provider_id INTEGER NOT NULL REFERENCES providers(id) ON DELETE CASCADE,
    ward_id     INTEGER NOT NULL REFERENCES wards(id) ON DELETE CASCADE,
    latitude    DOUBLE PRECISION,
    longitude   DOUBLE PRECISION,
    address     TEXT,
    phone       TEXT,
    created_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS business_branches (
    id          SERIAL PRIMARY KEY,
    business_id INTEGER NOT NULL REFERENCES businesses(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    latitude    DOUBLE PRECISION,
    longitude   DOUBLE PRECISION,
    ward_id     INTEGER REFERENCES wards(id) ON DELETE CASCADE,
    address     TEXT,
    phone       TEXT,
    created_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Services
CREATE TABLE IF NOT EXISTS services (
    id          SERIAL PRIMARY KEY,
    target_id   INTEGER NOT NULL,
    target_type VARCHAR(50) NOT NULL DEFAULT 'provider',
    title       VARCHAR(255) NOT NULL,
    description TEXT,
    price       NUMERIC(10, 2),
    duration    INTEGER,
    category_id INTEGER NOT NULL REFERENCES categories(id) ON DELETE CASCADE,
    is_active   BOOLEAN DEFAULT true,
    created_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_services_target_type_target_id
    ON services (target_type, target_id);

-- Posts / feed
CREATE TABLE IF NOT EXISTS posts (
    id          SERIAL PRIMARY KEY,
    provider_id INTEGER REFERENCES providers(id) ON DELETE CASCADE,
    business_id INTEGER REFERENCES businesses(id) ON DELETE CASCADE,
    title       VARCHAR(255),
    content     TEXT NOT NULL,
    created_at  TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITH TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT one_author_only CHECK (
        (provider_id IS NOT NULL AND business_id IS NULL) OR
        (provider_id IS NULL AND business_id IS NOT NULL)
    )
);

-- Attachments (for posts and services)
CREATE TABLE IF NOT EXISTS attachments (
    id          SERIAL PRIMARY KEY,
    file_name   TEXT NOT NULL,
    file_path   TEXT NOT NULL,
    file_type   TEXT NOT NULL CHECK (file_type IN ('image', 'video')),
    target_type TEXT NOT NULL CHECK (target_type IN ('provider', 'business')),
    target_id   INTEGER NOT NULL,
    uploaded_by INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    post_id     INTEGER,
    created_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Bookings
CREATE TABLE IF NOT EXISTS bookings (
    id                  SERIAL PRIMARY KEY,
    client_id           INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type         TEXT NOT NULL CHECK (target_type IN ('provider', 'business')),
    target_id           INTEGER NOT NULL,
    branch_id           INTEGER REFERENCES business_branches(id),
    service_id          INTEGER REFERENCES services(id) ON DELETE SET NULL,
    service_description TEXT,
    scheduled_time      TIMESTAMP WITHOUT TIME ZONE NOT NULL,
    duration            INTEGER DEFAULT 60,
    status              TEXT NOT NULL DEFAULT 'pending'
                            CHECK (status IN ('pending', 'confirmed', 'cancelled', 'completed')),
    created_at          TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at          TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Reviews
CREATE TABLE IF NOT EXISTS reviews (
    id          SERIAL PRIMARY KEY,
    reviewer_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type TEXT NOT NULL CHECK (target_type IN ('provider', 'business')),
    target_id   INTEGER NOT NULL,
    rating      INTEGER NOT NULL CHECK (rating >= 1 AND rating <= 5),
    comment     TEXT,
    created_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Favorites
CREATE TABLE IF NOT EXISTS favorites (
    id          SERIAL PRIMARY KEY,
    user_id     INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type TEXT NOT NULL CHECK (target_type IN ('business', 'provider')),
    target_id   INTEGER NOT NULL,
    created_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    UNIQUE (user_id, target_type, target_id)
);

-- Messaging
CREATE TABLE IF NOT EXISTS messages (
    id          SERIAL PRIMARY KEY,
    sender_id   INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    receiver_id INTEGER NOT NULL,
    target_type TEXT NOT NULL CHECK (target_type IN ('provider', 'business')),
    target_id   INTEGER NOT NULL,
    content     TEXT NOT NULL,
    is_read     BOOLEAN DEFAULT false,
    read_at     TIMESTAMP WITHOUT TIME ZONE,
    created_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Interactions (analytics — who messaged/booked whom)
CREATE TABLE IF NOT EXISTS interactions (
    id               SERIAL PRIMARY KEY,
    user_id          INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    target_type      TEXT NOT NULL CHECK (target_type IN ('provider', 'business')),
    target_id        INTEGER NOT NULL,
    interaction_type TEXT NOT NULL,
    occurred_at      TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Password resets
CREATE TABLE IF NOT EXISTS password_resets (
    id         SERIAL PRIMARY KEY,
    user_id    INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    token      TEXT NOT NULL UNIQUE,
    expires_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT (now() + interval '15 minutes')
);
