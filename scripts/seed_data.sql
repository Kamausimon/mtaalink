-- MtaaLink seed data
-- Called by seed.ps1 with psql -v variables for user IDs.
-- Variables: c1id, c2id (clients), p1id-p5id (providers), b1id, b2id (businesses)

-- ── Provider profiles ──────────────────────────────────────────────────────────

UPDATE providers SET
    service_name = 'Kamau Plumbing Services',
    category     = 'Plumbing',
    location     = 'Westlands, Nairobi',
    phone_number = '0712345678',
    email        = 'john.plumber@mtaatest.com',
    approved     = true
WHERE user_id = :p1id;

UPDATE providers SET
    service_name = 'Mary Hair Studio',
    category     = 'Beauty & Wellness',
    location     = 'Kilimani, Nairobi',
    phone_number = '0723456789',
    email        = 'mary.hair@mtaatest.com',
    approved     = true
WHERE user_id = :p2id;

UPDATE providers SET
    service_name = 'Otieno Electrical Works',
    category     = 'Electrical',
    location     = 'Kileleshwa, Nairobi',
    phone_number = '0734567890',
    email        = 'peter.elec@mtaatest.com',
    approved     = true
WHERE user_id = :p3id;

UPDATE providers SET
    service_name = 'Grace Home Cleaning',
    category     = 'Cleaning',
    location     = 'South B, Nairobi',
    phone_number = '0745678901',
    email        = 'grace.clean@mtaatest.com',
    approved     = true
WHERE user_id = :p4id;

UPDATE providers SET
    service_name = 'Achieng Tutoring Centre',
    category     = 'Education & Tutoring',
    location     = 'Umoja, Nairobi',
    phone_number = '0756789012',
    email        = 'samuel.tutor@mtaatest.com',
    approved     = true
WHERE user_id = :p5id;

-- ── Business profiles ──────────────────────────────────────────────────────────

UPDATE businesses SET
    description    = 'Professional cleaning company serving homes and offices across Nairobi with eco-friendly products',
    category       = 'Cleaning',
    location       = 'Nairobi CBD',
    license_number = 'NCC/2023/1001',
    krapin         = 'P051234567W',
    phone_number   = '0700111222',
    email          = 'cleanpro@mtaatest.com',
    verified       = true
WHERE user_id = :b1id;

UPDATE businesses SET
    description    = 'Authorised repairs for laptops, phones, and networking equipment. Fast turnaround guaranteed.',
    category       = 'Technology',
    location       = 'Ngara, Nairobi',
    license_number = 'NCC/2023/2002',
    krapin         = 'P059876543W',
    phone_number   = '0700333444',
    email          = 'techfix@mtaatest.com',
    verified       = true
WHERE user_id = :b2id;

-- ── Services ───────────────────────────────────────────────────────────────────

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Pipe Repair', 'Fix leaking or burst pipes, includes materials for minor jobs', 1500.00, 60,
       (SELECT id FROM categories WHERE name = 'Plumbing' LIMIT 1)
FROM providers p WHERE p.user_id = :p1id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Drain Unblocking', 'Clear blocked sinks, toilets, and outdoor drains', 2000.00, 90,
       (SELECT id FROM categories WHERE name = 'Plumbing' LIMIT 1)
FROM providers p WHERE p.user_id = :p1id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Water Heater Installation', 'Supply and fit solar or electric water heaters', 8500.00, 180,
       (SELECT id FROM categories WHERE name = 'Plumbing' LIMIT 1)
FROM providers p WHERE p.user_id = :p1id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Box Braids', 'Full head box braids using quality Kanekalon extensions', 3500.00, 240,
       (SELECT id FROM categories WHERE name = 'Hair Styling' LIMIT 1)
FROM providers p WHERE p.user_id = :p2id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Wash and Blow Dry', 'Shampoo, deep condition, and blow dry', 800.00, 60,
       (SELECT id FROM categories WHERE name = 'Hair Styling' LIMIT 1)
FROM providers p WHERE p.user_id = :p2id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Locs Retwist', 'Retwist and style dreadlocks', 1500.00, 120,
       (SELECT id FROM categories WHERE name = 'Locs & Dreadlocks' LIMIT 1)
FROM providers p WHERE p.user_id = :p2id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Socket and Switch Replacement', 'Replace faulty electrical sockets or light switches', 1200.00, 45,
       (SELECT id FROM categories WHERE name = 'Electrical' LIMIT 1)
FROM providers p WHERE p.user_id = :p3id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Full House Wiring', 'Complete electrical wiring for new builds or rewires', 45000.00, 480,
       (SELECT id FROM categories WHERE name = 'Electrical' LIMIT 1)
FROM providers p WHERE p.user_id = :p3id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Standard House Clean', 'Regular cleaning for up to 3 bedrooms', 2500.00, 180,
       (SELECT id FROM categories WHERE name = 'Cleaning' LIMIT 1)
FROM providers p WHERE p.user_id = :p4id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Deep Clean', 'Thorough deep clean including inside appliances and cabinets', 5000.00, 360,
       (SELECT id FROM categories WHERE name = 'Cleaning' LIMIT 1)
FROM providers p WHERE p.user_id = :p4id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'KCSE Maths Tutoring', '1-on-1 sessions targeting the full KCSE maths syllabus', 1500.00, 90,
       (SELECT id FROM categories WHERE name = 'KCSE Preparation' LIMIT 1)
FROM providers p WHERE p.user_id = :p5id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT p.id, 'provider', 'Physics Group Class', 'Small group physics revision, max 5 students', 800.00, 90,
       (SELECT id FROM categories WHERE name = 'KCSE Preparation' LIMIT 1)
FROM providers p WHERE p.user_id = :p5id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT b.id, 'business', 'Office Deep Clean', 'Full deep clean of office premises including toilets and kitchen', 12000.00, 480,
       (SELECT id FROM categories WHERE name = 'Cleaning' LIMIT 1)
FROM businesses b WHERE b.user_id = :b1id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT b.id, 'business', 'Weekly Office Cleaning', 'Scheduled weekly cleaning for offices up to 200 sqm', 6000.00, 240,
       (SELECT id FROM categories WHERE name = 'Cleaning' LIMIT 1)
FROM businesses b WHERE b.user_id = :b1id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT b.id, 'business', 'Laptop Repair', 'Diagnose and repair hardware or software issues', 3500.00, 120,
       (SELECT id FROM categories WHERE name = 'Computer & Laptop Repair' LIMIT 1)
FROM businesses b WHERE b.user_id = :b2id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT b.id, 'business', 'Phone Screen Replacement', 'Replace cracked or damaged screens, most models supported', 2500.00, 60,
       (SELECT id FROM categories WHERE name = 'Phone Repair' LIMIT 1)
FROM businesses b WHERE b.user_id = :b2id;

INSERT INTO services (target_id, target_type, title, description, price, duration, category_id)
SELECT b.id, 'business', 'WiFi Setup and Configuration', 'Install and configure home or office WiFi network', 4000.00, 120,
       (SELECT id FROM categories WHERE name = 'Network & WiFi Setup' LIMIT 1)
FROM businesses b WHERE b.user_id = :b2id;

-- ── Business branches ──────────────────────────────────────────────────────────

INSERT INTO business_branches (business_id, name, address, phone)
SELECT b.id, 'CleanPro CBD', 'Tom Mboya Street, Nairobi CBD', '0700111222'
FROM businesses b WHERE b.user_id = :b1id;

INSERT INTO business_branches (business_id, name, address, phone)
SELECT b.id, 'CleanPro Westlands', 'Westlands Road, Westlands', '0700111333'
FROM businesses b WHERE b.user_id = :b1id;

INSERT INTO business_branches (business_id, name, address, phone)
SELECT b.id, 'TechFix Ngara', 'Ngara Road, Ngara', '0700333444'
FROM businesses b WHERE b.user_id = :b2id;

INSERT INTO business_branches (business_id, name, address, phone)
SELECT b.id, 'TechFix Westlands', 'Westlands Mall, Ground Floor', '0700333555'
FROM businesses b WHERE b.user_id = :b2id;

-- ── Reviews ────────────────────────────────────────────────────────────────────

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c1id, 'provider', p.id, 5,
    'John fixed our burst pipe in under an hour. Very professional and cleaned up after himself. Highly recommended!'
FROM providers p WHERE p.user_id = :p1id;

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c2id, 'provider', p.id, 4,
    'Good plumber, arrived on time. Pricing is fair. Will use again.'
FROM providers p WHERE p.user_id = :p1id;

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c1id, 'provider', p.id, 5,
    'Mary is absolutely amazing! My box braids lasted 7 weeks and got so many compliments. Will definitely be back.'
FROM providers p WHERE p.user_id = :p2id;

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c2id, 'provider', p.id, 5,
    'Super skilled and very fast. Great attention to detail with my locs.'
FROM providers p WHERE p.user_id = :p2id;

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c1id, 'provider', p.id, 4,
    'Peter sorted our electrical fault the same day. Very knowledgeable and explains things clearly.'
FROM providers p WHERE p.user_id = :p3id;

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c2id, 'provider', p.id, 5,
    'Grace is thorough and trustworthy. The house was spotless. Already booked her for next month.'
FROM providers p WHERE p.user_id = :p4id;

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c1id, 'provider', p.id, 4,
    'Grace did a great job. Only minor thing was she arrived 15 min late, but the cleaning itself was excellent.'
FROM providers p WHERE p.user_id = :p4id;

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c1id, 'provider', p.id, 5,
    'My son went from a D to a B+ in maths within two terms. Samuel is patient and very effective. Worth every shilling!'
FROM providers p WHERE p.user_id = :p5id;

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c2id, 'business', b.id, 5,
    'CleanPro transformed our office. The team is professional, punctual, and uses quality products. Highly recommended.'
FROM businesses b WHERE b.user_id = :b1id;

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c1id, 'business', b.id, 5,
    'TechFix repaired my laptop screen the same day. Fair price and they stand behind their work.'
FROM businesses b WHERE b.user_id = :b2id;

INSERT INTO reviews (reviewer_id, target_type, target_id, rating, comment)
SELECT :c2id, 'business', b.id, 4,
    'Fast phone repair, good communication. The shop gets busy but they manage the queue well.'
FROM businesses b WHERE b.user_id = :b2id;

-- ── Bookings ───────────────────────────────────────────────────────────────────

INSERT INTO bookings (client_id, target_type, target_id, service_description, scheduled_time, status, duration)
SELECT :c1id, 'provider', p.id,
    'Fix leaking pipe under the kitchen sink',
    now() - interval '14 days', 'completed', 60
FROM providers p WHERE p.user_id = :p1id;

INSERT INTO bookings (client_id, target_type, target_id, service_description, scheduled_time, status, duration)
SELECT :c1id, 'provider', p.id,
    'Full head box braids with Kanekalon hair',
    now() - interval '5 days', 'completed', 240
FROM providers p WHERE p.user_id = :p2id;

INSERT INTO bookings (client_id, target_type, target_id, service_description, scheduled_time, status, duration)
SELECT :c1id, 'provider', p.id,
    'Replace faulty socket in the living room and check fuse board',
    now() + interval '2 days', 'confirmed', 60
FROM providers p WHERE p.user_id = :p3id;

INSERT INTO bookings (client_id, target_type, target_id, service_description, scheduled_time, status, duration)
SELECT :c2id, 'provider', p.id,
    'Standard clean for 2-bedroom apartment before moving in',
    now() + interval '1 day', 'pending', 180
FROM providers p WHERE p.user_id = :p4id;

INSERT INTO bookings (client_id, target_type, target_id, service_description, scheduled_time, status, duration)
SELECT :c2id, 'business', b.id,
    'Weekly office cleaning for our 150 sqm workspace',
    now() + interval '3 days', 'confirmed', 240
FROM businesses b WHERE b.user_id = :b1id;

INSERT INTO bookings (client_id, target_type, target_id, service_description, scheduled_time, status, duration)
SELECT :c1id, 'business', b.id,
    'Laptop screen replacement, Dell Inspiron 15',
    now() - interval '2 days', 'completed', 120
FROM businesses b WHERE b.user_id = :b2id;
