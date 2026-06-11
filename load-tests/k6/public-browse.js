// Load test for the unauthenticated "browsing" path: a visitor landing on
// the site, searching, browsing categories, and opening provider/business
// profiles. No login required.
//
// Run:
//   k6 run load-tests/k6/public-browse.js
//   k6 run -e BASE_URL=https://api.mtaalink.com load-tests/k6/public-browse.js
//
// Tune the load shape with stages below, or override on the CLI:
//   k6 run --vus 50 --duration 2m load-tests/k6/public-browse.js

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { Trend } from 'k6/metrics';
import { BASE_URL } from './config.js';

const searchDuration = new Trend('search_duration', true);

export const options = {
  scenarios: {
    browsing: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '30s', target: 10 },
        { duration: '1m', target: 30 },
        { duration: '1m', target: 30 },
        { duration: '30s', target: 0 },
      ],
    },
  },
  thresholds: {
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<800'],
    search_duration: ['p(95)<800'],
  },
};

const SEARCH_TERMS = ['plumbing', 'cleaning', 'electrical', 'tutoring', 'catering', 'beauty'];
const LOCATIONS = ['Westlands, Nairobi', 'Kasarani, Nairobi', 'CBD, Nairobi', ''];

// Fetch some real provider/business IDs once, shared (read-only) across all VUs,
// so detail-page requests hit real records instead of 404s.
export function setup() {
  const providers = http.get(`${BASE_URL}/service_providers/listProviders`).json('providers') || [];
  const businesses = http.get(`${BASE_URL}/categories/businesses/by-category`).json('businesses') || [];

  return {
    providerIds: providers.slice(0, 20).map((p) => p.id),
    businessIds: [...new Set(businesses.map((b) => b.business_id))].slice(0, 20),
  };
}

function randomItem(arr) {
  return arr[Math.floor(Math.random() * arr.length)];
}

export default function (data) {
  group('homepage / categories', () => {
    const res = http.get(`${BASE_URL}/categories/allCategories`);
    check(res, { 'categories: status 200': (r) => r.status === 200 });
  });

  group('search', () => {
    const q = randomItem(SEARCH_TERMS);
    const location = randomItem(LOCATIONS);
    let qs = `q=${encodeURIComponent(q)}&page=1&per_page=12`;
    if (location) qs += `&location=${encodeURIComponent(location)}`;

    const res = http.get(`${BASE_URL}/search?${qs}`);
    searchDuration.add(res.timings.duration);
    check(res, { 'search: status 200': (r) => r.status === 200 });
  });

  group('listings', () => {
    const provRes = http.get(`${BASE_URL}/service_providers/listProviders`);
    check(provRes, { 'list providers: status 200': (r) => r.status === 200 });

    const bizRes = http.get(`${BASE_URL}/categories/businesses/by-category`);
    check(bizRes, { 'list businesses: status 200': (r) => r.status === 200 });
  });

  group('profile detail', () => {
    if (data.providerIds.length > 0) {
      const id = randomItem(data.providerIds);
      const res = http.get(`${BASE_URL}/service_providers/${id}`);
      check(res, { 'provider detail: status 200': (r) => r.status === 200 });
    }
    if (data.businessIds.length > 0) {
      const id = randomItem(data.businessIds);
      const res = http.get(`${BASE_URL}/businesses/${id}`);
      check(res, { 'business detail: status 200': (r) => r.status === 200 });
    }
  });

  sleep(Math.random() * 2 + 1); // think time: 1-3s
}
