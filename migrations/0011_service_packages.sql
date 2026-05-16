-- A package bundles multiple services at a discounted price
CREATE TABLE IF NOT EXISTS service_packages (
    id          SERIAL PRIMARY KEY,
    target_type VARCHAR(50) NOT NULL CHECK (target_type IN ('provider', 'business')),
    target_id   INTEGER NOT NULL,
    name        VARCHAR(255) NOT NULL,
    description TEXT,
    price       NUMERIC(10, 2) NOT NULL,
    is_active   BOOLEAN NOT NULL DEFAULT true,
    created_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP,
    updated_at  TIMESTAMP WITHOUT TIME ZONE DEFAULT CURRENT_TIMESTAMP
);

-- Which services are included in each package
CREATE TABLE IF NOT EXISTS service_package_items (
    package_id INTEGER NOT NULL REFERENCES service_packages(id) ON DELETE CASCADE,
    service_id INTEGER NOT NULL REFERENCES services(id) ON DELETE CASCADE,
    PRIMARY KEY (package_id, service_id)
);

CREATE INDEX IF NOT EXISTS idx_service_packages_target
    ON service_packages (target_type, target_id);
