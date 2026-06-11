// Fixed-concurrency sweep: hits the same mix of public endpoints as
// public-browse.js, but at a constant VU count for a fixed duration,
// so each run reports steady-state numbers for that concurrency level.
//
// Run once per concurrency level:
//   k6 run -e VUS=10  -e DURATION=20s load-tests/k6/concurrency-sweep.js
//   k6 run -e VUS=25  -e DURATION=20s load-tests/k6/concurrency-sweep.js
//   k6 run -e VUS=50  -e DURATION=20s load-tests/k6/concurrency-sweep.js
//   k6 run -e VUS=100 -e DURATION=20s load-tests/k6/concurrency-sweep.js

import http from 'k6/http';
import { check, group, sleep } from 'k6';
import { BASE_URL } from './config.js';

const VUS = parseInt(__ENV.VUS || '10', 10);
const DURATION = __ENV.DURATION || '20s';

export const options = {
  vus: VUS,
  duration: DURATION,
  thresholds: {
    http_req_failed: ['rate<0.01'],
  },
};

const SEARCH_TERMS = ['plumbing', 'cleaning', 'electrical', 'tutoring', 'catering', 'beauty'];

function randomItem(arr) {
  return arr[Math.floor(Math.random() * arr.length)];
}

export default function () {
  group('categories', () => {
    const res = http.get(`${BASE_URL}/categories/allCategories`);
    check(res, { 'categories: status 200': (r) => r.status === 200 });
  });

  group('search', () => {
    const q = randomItem(SEARCH_TERMS);
    const res = http.get(`${BASE_URL}/search?q=${encodeURIComponent(q)}&page=1&per_page=12`);
    check(res, { 'search: status 200': (r) => r.status === 200 });
  });

  group('listings', () => {
    const provRes = http.get(`${BASE_URL}/service_providers/listProviders`);
    check(provRes, { 'list providers: status 200': (r) => r.status === 200 });
  });

  sleep(0.5);
}
