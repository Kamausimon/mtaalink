// MtaaLink seed script — creates realistic provider and business accounts
// Usage: node scripts/seed.js
// Prerequisites: backend running at http://localhost:7878

const BASE_URL = process.env.API_URL ?? "http://localhost:7878";
const SEED_PASSWORD = "SeedTest@2026";

let passed = 0;
let failed = 0;
let registrationCount = 0;

async function sleep(ms) {
  return new Promise((r) => setTimeout(r, ms));
}

async function api(method, path, body, token, retries = 8) {
  const headers = { "Content-Type": "application/json", Accept: "application/json" };
  if (token) headers["Authorization"] = `Bearer ${token}`;

  for (let attempt = 0; attempt <= retries; attempt++) {
    const res = await fetch(`${BASE_URL}${path}`, {
      method,
      headers,
      body: body ? JSON.stringify(body) : undefined,
    });

    if (res.status === 429) {
      const wait = 14000;
      process.stdout.write(`\n     [429] waiting ${wait / 1000}s… `);
      await sleep(wait);
      continue;
    }

    const json = await res.json().catch(() => ({}));
    if (!res.ok) throw new Error(json.error ?? `HTTP ${res.status} ${path}`);
    return json;
  }

  throw new Error(`Still rate-limited after ${retries} retries on ${path}`);
}

async function register(username, email, role, extra = {}) {
  registrationCount++;
  const res = await api("POST", "/auth/register", {
    username,
    email,
    password: SEED_PASSWORD,
    confirm_password: SEED_PASSWORD,
    role,
    ...extra,
  });
  return res.token;
}

async function onboardProvider(token, data) {
  await api("POST", "/service_providers/onboard", data, token);
}

async function onboardBusiness(token, data) {
  await api("POST", "/businesses/onboard", data, token);
}

async function createService(token, data) {
  await api("POST", "/services/createService", data, token);
}

async function run(label, fn) {
  try {
    await fn();
    console.log(`  ✓  ${label}`);
    passed++;
  } catch (err) {
    console.log(`  ✗  ${label} — ${err.message}`);
    failed++;
  }
}

// ─────────────────────────────────────────────────────────────────────────────
// PROVIDERS
// ─────────────────────────────────────────────────────────────────────────────

const PROVIDERS = [
  {
    user: { username: "johnkamau", email: "john.kamau@seedtest.mtaalink", role: "provider" },
    profile: {
      service_name: "Kamau Plumbing Services",
      service_description:
        "Professional plumber with 10+ years experience across Nairobi. I handle pipe installation, repairs, water heater fitting, drain unblocking, and all plumbing emergencies. Same-day service available.",
      category: "Plumbing",
      location: "Westlands, Nairobi",
      phone_number: "0712100001",
      email: "john.kamau@seedtest.mtaalink",
    },
    services: [
      { title: "Pipe Repair & Leak Fixing", description: "Fix burst or leaking pipes quickly. Includes all fittings.", price: 1500, duration: 60 },
      { title: "Water Heater Installation", description: "Supply and fit solar or electric water heaters.", price: 8500, duration: 180 },
      { title: "Drain Unblocking", description: "Clear blocked drains and sinks using professional equipment.", price: 2000, duration: 90 },
      { title: "Full Bathroom Plumbing", description: "Complete bathroom plumbing installation for new builds or renovations.", price: 25000, duration: 480 },
    ],
  },
  {
    user: { username: "saranjeri", email: "sarah.njeri@seedtest.mtaalink", role: "provider" },
    profile: {
      service_name: "Sarah's Sparkle Cleaning",
      service_description:
        "Thorough, reliable home and office cleaning service based in Kasarani. I use eco-friendly products and guarantee satisfaction. Available weekdays and weekends.",
      category: "Cleaning",
      location: "Kasarani, Nairobi",
      phone_number: "0712100002",
      email: "sarah.njeri@seedtest.mtaalink",
    },
    services: [
      { title: "Regular Home Cleaning", description: "Weekly or bi-weekly cleaning for 2–4 bedroom homes.", price: 2500, duration: 180 },
      { title: "Deep Clean", description: "Thorough top-to-bottom cleaning including inside appliances.", price: 5500, duration: 360 },
      { title: "Move-In/Out Cleaning", description: "Leave the property spotless for new tenants or handover.", price: 7000, duration: 480 },
      { title: "Office Cleaning", description: "After-hours or weekend office cleaning.", price: 3500, duration: 240 },
    ],
  },
  {
    user: { username: "peterodhiambo", email: "peter.odhiambo@seedtest.mtaalink", role: "provider" },
    profile: {
      service_name: "Odhiambo Electrical Works",
      service_description:
        "Licensed electrician covering all residential and light commercial electrical work in Nairobi. I handle wiring, installations, fault finding, and security lighting. No job too small.",
      category: "Electrical",
      location: "South B, Nairobi",
      phone_number: "0712100003",
      email: "peter.odhiambo@seedtest.mtaalink",
    },
    services: [
      { title: "Fault Finding & Repair", description: "Diagnose and fix electrical faults, tripped circuits, and power outages.", price: 1800, duration: 90 },
      { title: "New Socket & Switch Installation", description: "Install additional sockets or replace faulty switches.", price: 1200, duration: 60 },
      { title: "Security & Flood Lighting", description: "Install motion-sensor security lights and flood lights.", price: 4500, duration: 180 },
      { title: "Full House Wiring", description: "Complete electrical wiring for new builds.", price: 45000, duration: 960 },
    ],
  },
  {
    user: { username: "gracewanjiku", email: "grace.wanjiku@seedtest.mtaalink", role: "provider" },
    profile: {
      service_name: "Grace Wanjiku — Math & Science Tutor",
      service_description:
        "Experienced secondary school teacher offering private tutoring in Mathematics, Physics, and Chemistry. KCSE specialist. Proven track record — 90% of my students improve by at least 2 grades.",
      category: "Secondary School Tutoring",
      location: "Kilimani, Nairobi",
      phone_number: "0712100004",
      email: "grace.wanjiku@seedtest.mtaalink",
    },
    services: [
      { title: "KCSE Mathematics (Form 3–4)", description: "Comprehensive KCSE Math coaching with past papers. Per session.", price: 1500, duration: 90 },
      { title: "KCSE Physics", description: "Form 3 & 4 Physics tuition. Practical and theory.", price: 1500, duration: 90 },
      { title: "KCSE Chemistry", description: "Form 3 & 4 Chemistry — organic, inorganic, and calculations.", price: 1500, duration: 90 },
      { title: "Holiday Intensive Programme", description: "5-day holiday crash course for all KCSE sciences.", price: 12000, duration: 300 },
    ],
  },
  {
    user: { username: "jamesmwangi", email: "james.mwangi@seedtest.mtaalink", role: "provider" },
    profile: {
      service_name: "Mwangi Painters & Decorators",
      service_description:
        "Quality painting and decoration for homes and offices across Nairobi. I use premium Crown and Dulux paints. Clean, professional finish guaranteed. Free colour consultation included.",
      category: "Painting & Decoration",
      location: "Embakasi, Nairobi",
      phone_number: "0712100005",
      email: "james.mwangi@seedtest.mtaalink",
    },
    services: [
      { title: "Interior Room Painting", description: "Single room interior painting, walls and ceiling. Paint included.", price: 8000, duration: 480 },
      { title: "Exterior House Painting", description: "Full exterior painting for 3–4 bedroom bungalow.", price: 35000, duration: 960 },
      { title: "Texture & Feature Wall", description: "Decorative texture or feature wall finishes.", price: 6500, duration: 360 },
      { title: "Office Painting", description: "Commercial office painting, after hours if needed.", price: 15000, duration: 720 },
    ],
  },
  {
    user: { username: "aliceachieng", email: "alice.achieng@seedtest.mtaalink", role: "provider" },
    profile: {
      service_name: "Alice — Natural Hair Specialist",
      service_description:
        "Natural hair specialist offering braiding, loc care, and natural hair treatments. I come to you anywhere in Nairobi. Using only quality, chemical-free products. Book in advance — slots fill fast!",
      category: "Hair Styling",
      location: "Westlands, Nairobi",
      phone_number: "0712100006",
      email: "alice.achieng@seedtest.mtaalink",
    },
    services: [
      { title: "Box Braids (Medium)", description: "Medium-sized box braids. Hair included.", price: 3500, duration: 300 },
      { title: "Knotless Braids", description: "Knotless box braids — lighter and more natural look.", price: 4500, duration: 360 },
      { title: "Loc Maintenance & Retwist", description: "Loc retwist and moisturising treatment.", price: 2500, duration: 180 },
      { title: "Natural Hair Wash & Style", description: "Wash, deep condition, and style for natural hair.", price: 2000, duration: 150 },
    ],
  },
  {
    user: { username: "davidkiprotich", email: "david.kiprotich@seedtest.mtaalink", role: "provider" },
    profile: {
      service_name: "Kiprotich Custom Carpentry",
      service_description:
        "Skilled carpenter and joiner with 15 years experience. I build custom furniture, fit doors and windows, and do kitchen cabinet installations. All work guaranteed for 2 years.",
      category: "Carpentry & Joinery",
      location: "Industrial Area, Nairobi",
      phone_number: "0712100007",
      email: "david.kiprotich@seedtest.mtaalink",
    },
    services: [
      { title: "Custom Wardrobe", description: "Built-in wardrobe to your dimensions and finish.", price: 28000, duration: 720 },
      { title: "Kitchen Cabinet Installation", description: "Supply and fit kitchen cabinets — upper and lower units.", price: 55000, duration: 960 },
      { title: "Door Fitting", description: "Supply and hang interior or exterior door including frame.", price: 8500, duration: 240 },
      { title: "Custom Bed Frame", description: "Build a solid wood bed frame to your design.", price: 18000, duration: 480 },
    ],
  },
  {
    user: { username: "mercywairimu", email: "mercy.wairimu@seedtest.mtaalink", role: "provider" },
    profile: {
      service_name: "Mercy — Personal Trainer",
      service_description:
        "Certified personal trainer offering home-based fitness sessions across Nairobi. Specialising in weight loss, strength training, and postnatal fitness. Flexible schedule, results guaranteed.",
      category: "Personal Training",
      location: "Lavington, Nairobi",
      phone_number: "0712100008",
      email: "mercy.wairimu@seedtest.mtaalink",
    },
    services: [
      { title: "Single Training Session", description: "1-hour personal training session at your home or compound.", price: 2500, duration: 60 },
      { title: "Monthly Package (12 sessions)", description: "3 sessions per week for a month. Includes nutrition plan.", price: 25000, duration: 60 },
      { title: "Postnatal Fitness Programme", description: "6-week programme designed for new mothers.", price: 18000, duration: 60 },
      { title: "Weight Loss Challenge (8 weeks)", description: "Structured 8-week programme with weekly check-ins.", price: 30000, duration: 60 },
    ],
  },
  {
    user: { username: "samuelodundo", email: "samuel.odundo@seedtest.mtaalink", role: "provider" },
    profile: {
      service_name: "Odundo Tech Repairs",
      service_description:
        "Computer and phone repair technician with 8 years experience. Same-day repairs where possible. I come to you or you drop off. Genuine parts used. 3-month warranty on all repairs.",
      category: "Computer & Laptop Repair",
      location: "Nairobi CBD",
      phone_number: "0712100009",
      email: "samuel.odundo@seedtest.mtaalink",
    },
    services: [
      { title: "Laptop Screen Replacement", description: "Genuine screen replacement for all laptop models.", price: 8000, duration: 120 },
      { title: "Virus & Malware Removal", description: "Full system clean, antivirus install, and tune-up.", price: 2500, duration: 60 },
      { title: "Data Recovery", description: "Recover lost files from crashed hard drives or formatted disks.", price: 5000, duration: 180 },
      { title: "Phone Screen Repair", description: "Screen replacement for iPhone, Samsung, Tecno, and more.", price: 4500, duration: 90 },
    ],
  },
  {
    user: { username: "fatimahassan", email: "fatima.hassan@seedtest.mtaalink", role: "provider" },
    profile: {
      service_name: "Fatima Hassan — Makeup Artist",
      service_description:
        "Professional makeup artist for weddings, graduations, events, and photoshoots. Based in South C but travel across Nairobi. Airbrush and traditional techniques. Book 2 weeks in advance for weekends.",
      category: "Makeup & Beauty",
      location: "South C, Nairobi",
      phone_number: "0712100010",
      email: "fatima.hassan@seedtest.mtaalink",
    },
    services: [
      { title: "Bridal Makeup", description: "Full bridal glam including trial session. Travel included.", price: 12000, duration: 120 },
      { title: "Event Makeup", description: "Party, graduation, or photoshoot makeup.", price: 4500, duration: 60 },
      { title: "Gele & Makeup (Full Look)", description: "Gele headwrap tying plus full makeup look.", price: 7000, duration: 90 },
      { title: "Makeup Lesson (1-on-1)", description: "Learn makeup application techniques for everyday looks.", price: 5000, duration: 120 },
    ],
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// BUSINESSES
// ─────────────────────────────────────────────────────────────────────────────

const BUSINESSES = [
  {
    user: { username: "brightclean", email: "admin@brightclean.seedtest.mtaalink", role: "business" },
    profile: {
      business_name: "BrightHome Cleaning Solutions",
      description:
        "Nairobi's most trusted commercial and residential cleaning company. We provide regular, deep-clean, post-construction, and move-in/out cleaning with trained, vetted staff. Fully insured. Corporate contracts available.",
      category: "Cleaning",
      location: "Upper Hill, Nairobi",
      license_number: "NRB/BUS/2021/08841",
      krapin: "A012345678P",
      phone_number: "0720200001",
      email: "admin@brightclean.seedtest.mtaalink",
      website: "https://brightclean.co.ke",
    },
    services: [
      { title: "Office Cleaning (Monthly Contract)", description: "Daily after-hours office cleaning. Price per day.", price: 4500, duration: 240 },
      { title: "Residential Deep Clean", description: "Full deep clean for 3-bed house. Includes kitchen, bathrooms, all rooms.", price: 9500, duration: 480 },
      { title: "Post-Construction Cleaning", description: "Remove construction dust and debris from newly built or renovated property.", price: 18000, duration: 720 },
      { title: "Carpet Shampooing", description: "Professional carpet and upholstery deep cleaning.", price: 6000, duration: 300 },
    ],
  },
  {
    user: { username: "secureguardke", email: "admin@secureguard.seedtest.mtaalink", role: "business" },
    profile: {
      business_name: "SecureGuard Kenya Ltd",
      description:
        "Licensed security company providing manned guarding, CCTV installation, access control, and alarm systems. Serving residential estates, commercial premises, and events across Kenya since 2015.",
      category: "Security & Guarding",
      location: "Westlands, Nairobi",
      license_number: "NRB/BUS/2015/03317",
      krapin: "A098765432B",
      phone_number: "0720200002",
      email: "admin@secureguard.seedtest.mtaalink",
    },
    services: [
      { title: "Manned Guard (Monthly)", description: "Trained uniformed guard. Day or night shift, monthly contract.", price: 28000, duration: 43200 },
      { title: "CCTV System Installation", description: "4-camera HD CCTV system with remote viewing. Parts and labour.", price: 45000, duration: 480 },
      { title: "Electric Fence Installation", description: "Energised electric fence for perimeter security.", price: 85000, duration: 960 },
      { title: "Event Security", description: "Uniformed security team for events. Per guard per day.", price: 4500, duration: 720 },
    ],
  },
  {
    user: { username: "chefmasters", email: "admin@chefmasters.seedtest.mtaalink", role: "business" },
    profile: {
      business_name: "ChefMasters Catering & Events",
      description:
        "Premium catering company for weddings, corporate functions, birthday parties, and all events. Our experienced chefs create custom menus for Kenyan, continental, and Indian cuisine. Minimum 30 guests.",
      category: "Wedding Catering",
      location: "Runda, Nairobi",
      license_number: "NRB/BUS/2018/05529",
      krapin: "A011223344C",
      phone_number: "0720200003",
      email: "admin@chefmasters.seedtest.mtaalink",
    },
    services: [
      { title: "Wedding Catering (Per Head)", description: "Full wedding catering — 3-course meal, service, equipment hire.", price: 3500, duration: 720 },
      { title: "Corporate Lunch Buffet", description: "Buffet lunch setup for 30–200 people. Kenyan or continental menu.", price: 85000, duration: 300 },
      { title: "Birthday Party Package", description: "Snacks, finger foods, and dessert station for 30–80 guests.", price: 55000, duration: 360 },
      { title: "Nyama Choma Package", description: "Full nyama choma spread with accompaniments for 20+ guests.", price: 45000, duration: 300 },
    ],
  },
  {
    user: { username: "techfixnairobi", email: "admin@techfix.seedtest.mtaalink", role: "business" },
    profile: {
      business_name: "TechFix Solutions Ltd",
      description:
        "End-to-end IT solutions for small and medium businesses in Nairobi. We handle computer repairs, network setup, server installation, CCTV, and ongoing IT support contracts. Fast turnaround, competitive rates.",
      category: "Computer & Laptop Repair",
      location: "Upperhill, Nairobi",
      license_number: "NRB/BUS/2019/06631",
      krapin: "A055667788D",
      phone_number: "0720200004",
      email: "admin@techfix.seedtest.mtaalink",
    },
    services: [
      { title: "IT Support Contract (Monthly)", description: "Monthly IT support retainer for up to 10 workstations.", price: 18000, duration: 480 },
      { title: "Office Network Setup", description: "Full LAN/WiFi setup for office up to 20 users.", price: 35000, duration: 720 },
      { title: "CCTV Installation (8 cameras)", description: "8-camera HD CCTV with NVR and remote access.", price: 75000, duration: 720 },
      { title: "Bulk Laptop Repair", description: "Assessment and repair for 5+ laptops. Per laptop.", price: 6000, duration: 120 },
    ],
  },
  {
    user: { username: "greenthumbke", email: "admin@greenthumb.seedtest.mtaalink", role: "business" },
    profile: {
      business_name: "GreenThumb Garden Services",
      description:
        "Professional landscaping and garden maintenance company serving Nairobi's residential estates and commercial properties. From lawn mowing to full garden design and installation. Monthly contracts available.",
      category: "Gardening & Landscaping",
      location: "Karen, Nairobi",
      license_number: "NRB/BUS/2020/07742",
      krapin: "A077889900E",
      phone_number: "0720200005",
      email: "admin@greenthumb.seedtest.mtaalink",
    },
    services: [
      { title: "Monthly Lawn Maintenance", description: "Fortnightly lawn mowing, edging, and general tidy. Per visit.", price: 3500, duration: 180 },
      { title: "Garden Design & Installation", description: "Full garden redesign with planting plan, soil prep, and installation.", price: 120000, duration: 1440 },
      { title: "Tree Pruning & Removal", description: "Safe pruning or removal of trees and large shrubs.", price: 15000, duration: 360 },
      { title: "Irrigation System Installation", description: "Drip or sprinkler system for lawn and garden areas.", price: 45000, duration: 480 },
    ],
  },
];

// ─────────────────────────────────────────────────────────────────────────────
// MAIN
// ─────────────────────────────────────────────────────────────────────────────

async function seedProvider({ user, profile, services }) {
  await run(`Register ${user.username}`, async () => {
    const token = await register(user.username, user.email, user.role, {
      service_description: profile.service_description,
    });

    await run(`Onboard ${profile.service_name}`, () => onboardProvider(token, profile));

    // Need the provider_id — get it from the API
    const meRes = await api("GET", "/auth/me", null, token);
    const providerId = meRes.user?.id;

    if (providerId) {
      for (const svc of services) {
        await run(`  Service: ${svc.title}`, () =>
          createService(token, {
            ...svc,
            target_type: "provider",
            target_id: providerId,
          })
        );
      }
    }
  });
}

async function seedBusiness({ user, profile, services }) {
  await run(`Register ${user.username}`, async () => {
    const token = await register(user.username, user.email, user.role, {
      business_name: profile.business_name,
    });

    await run(`Onboard ${profile.business_name}`, () => onboardBusiness(token, profile));

    const meRes = await api("GET", "/auth/me", null, token);
    const userId = meRes.user?.id;

    if (userId) {
      // Get the business ID
      const providersRes = await api("GET", `/service_providers/getProviderData?provider_id=${userId}`, null, token)
        .catch(() => null);

      // Try to get business id via dashboard
      const dash = await api("GET", "/dashboard", null, token).catch(() => null);
      const businessId = dash?.business_id;

      if (businessId) {
        for (const svc of services) {
          await run(`  Service: ${svc.title}`, () =>
            createService(token, {
              ...svc,
              target_type: "business",
              target_id: businessId,
            })
          );
        }
      }
    }
  });
}

async function main() {
  console.log(`\nMtaaLink Seed Script`);
  console.log(`API: ${BASE_URL}`);
  console.log(`Password for all seed accounts: ${SEED_PASSWORD}\n`);

  // Check backend is reachable
  try {
    const res = await fetch(`${BASE_URL}/`);
    if (!res.ok) throw new Error(`Backend returned ${res.status}`);
    console.log(`✓  Backend reachable at ${BASE_URL}\n`);
  } catch {
    console.error(`✗  Cannot reach backend at ${BASE_URL}`);
    console.error(`   Make sure "cargo run" is running first.\n`);
    process.exit(1);
  }

  console.log("── Seeding providers ─────────────────────────────────────────");
  for (const provider of PROVIDERS) {
    await seedProvider(provider);
  }

  console.log("\n── Seeding businesses ────────────────────────────────────────");
  for (const business of BUSINESSES) {
    await seedBusiness(business);
  }

  console.log(`\n─────────────────────────────────────────────────────────────`);
  console.log(`Done. ${passed} passed, ${failed} failed.`);
  console.log(`\nAll seed accounts use password: ${SEED_PASSWORD}`);
  console.log(`You can log in as any of these to test the provider experience.\n`);
}

main().catch((err) => {
  console.error("Seed failed:", err.message);
  process.exit(1);
});
