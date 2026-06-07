-- MtaaLink category seed — run this once against your database
-- Usage: psql $DATABASE_URL -f scripts/seed_categories.sql

-- ── Parent categories ─────────────────────────────────────────────────────────
INSERT INTO categories (name, parent_id) VALUES
  ('Home Services',        NULL),
  ('Education & Tutoring', NULL),
  ('Beauty & Wellness',    NULL),
  ('Events & Catering',    NULL),
  ('Business Services',    NULL),
  ('Technology',           NULL),
  ('Health & Fitness',     NULL)
ON CONFLICT DO NOTHING;

-- ── Home Services subcategories ───────────────────────────────────────────────
WITH parent AS (SELECT id FROM categories WHERE name = 'Home Services')
INSERT INTO categories (name, parent_id)
SELECT sub.name, parent.id FROM parent, (VALUES
  ('Plumbing'),
  ('Electrical'),
  ('Cleaning'),
  ('Painting & Decoration'),
  ('Carpentry & Joinery'),
  ('Security & Guarding'),
  ('Gardening & Landscaping'),
  ('Moving & Relocation'),
  ('Pest Control'),
  ('AC & Appliance Repair')
) AS sub(name)
ON CONFLICT DO NOTHING;

-- ── Education subcategories ───────────────────────────────────────────────────
WITH parent AS (SELECT id FROM categories WHERE name = 'Education & Tutoring')
INSERT INTO categories (name, parent_id)
SELECT sub.name, parent.id FROM parent, (VALUES
  ('Primary School Tutoring'),
  ('Secondary School Tutoring'),
  ('KCPE Preparation'),
  ('KCSE Preparation'),
  ('University Level'),
  ('Music Lessons'),
  ('Language Lessons'),
  ('Driving School')
) AS sub(name)
ON CONFLICT DO NOTHING;

-- ── Beauty subcategories ──────────────────────────────────────────────────────
WITH parent AS (SELECT id FROM categories WHERE name = 'Beauty & Wellness')
INSERT INTO categories (name, parent_id)
SELECT sub.name, parent.id FROM parent, (VALUES
  ('Hair Styling'),
  ('Makeup & Beauty'),
  ('Nail Care'),
  ('Massage & Spa'),
  ('Barber'),
  ('Locs & Dreadlocks')
) AS sub(name)
ON CONFLICT DO NOTHING;

-- ── Events subcategories ──────────────────────────────────────────────────────
WITH parent AS (SELECT id FROM categories WHERE name = 'Events & Catering')
INSERT INTO categories (name, parent_id)
SELECT sub.name, parent.id FROM parent, (VALUES
  ('Wedding Catering'),
  ('Corporate Events'),
  ('Event Photography'),
  ('DJ & Entertainment'),
  ('Tent & Decor Hire'),
  ('Cake & Pastry')
) AS sub(name)
ON CONFLICT DO NOTHING;

-- ── Business Services subcategories ──────────────────────────────────────────
WITH parent AS (SELECT id FROM categories WHERE name = 'Business Services')
INSERT INTO categories (name, parent_id)
SELECT sub.name, parent.id FROM parent, (VALUES
  ('Accounting & Bookkeeping'),
  ('Legal Services'),
  ('Marketing & Branding'),
  ('Printing & Design'),
  ('Courier & Delivery')
) AS sub(name)
ON CONFLICT DO NOTHING;

-- ── Technology subcategories ──────────────────────────────────────────────────
WITH parent AS (SELECT id FROM categories WHERE name = 'Technology')
INSERT INTO categories (name, parent_id)
SELECT sub.name, parent.id FROM parent, (VALUES
  ('Computer & Laptop Repair'),
  ('Phone Repair'),
  ('Network & WiFi Setup'),
  ('CCTV Installation'),
  ('Web & App Development'),
  ('Data Recovery')
) AS sub(name)
ON CONFLICT DO NOTHING;

-- ── Health subcategories ──────────────────────────────────────────────────────
WITH parent AS (SELECT id FROM categories WHERE name = 'Health & Fitness')
INSERT INTO categories (name, parent_id)
SELECT sub.name, parent.id FROM parent, (VALUES
  ('Personal Training'),
  ('Yoga & Pilates'),
  ('Home Nursing'),
  ('Nutritionist')
) AS sub(name)
ON CONFLICT DO NOTHING;

SELECT COUNT(*) AS total_categories FROM categories;
