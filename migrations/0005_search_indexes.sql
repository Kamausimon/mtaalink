-- Full-text search indexes (GIN on expression must match WHERE clause exactly)
CREATE INDEX IF NOT EXISTS idx_providers_fts
    ON providers
    USING GIN (
        to_tsvector('english',
            coalesce(service_name, '') || ' ' ||
            coalesce(service_description, '') || ' ' ||
            coalesce(category, '')
        )
    );

CREATE INDEX IF NOT EXISTS idx_businesses_fts
    ON businesses
    USING GIN (
        to_tsvector('english',
            coalesce(business_name, '') || ' ' ||
            coalesce(description, '') || ' ' ||
            coalesce(category, '')
        )
    );

-- FK index for geo JOIN performance (PostgreSQL does NOT auto-index FK columns)
CREATE INDEX IF NOT EXISTS idx_provider_locations_provider_id
    ON provider_locations (provider_id);

CREATE INDEX IF NOT EXISTS idx_business_branches_business_id
    ON business_branches (business_id);
