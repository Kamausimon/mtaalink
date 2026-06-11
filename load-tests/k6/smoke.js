// Smoke test — 1 VU, a handful of iterations, hits the most important
// public endpoints to verify the API is up and responding correctly
// before running heavier load tests.
//
// Run:
//   k6 run load-tests/k6/smoke.js
//   k6 run -e BASE_URL=https://api.mtaalink.com load-tests/k6/smoke.js

import http from 'k6/http';
import { check, sleep } from 'k6';
import { BASE_URL } from './config.js';

export const options = {
  vus: 1,
  iterations: 5,
  thresholds: {
    http_req_failed: ['rate==0'],
    http_req_duration: ['p(95)<1000'],
  },
};

export default function () {
  const endpoints = [
    ['/categories/allCategories', 'categories'],
    ['/service_providers/listProviders', 'providers list'],
    ['/categories/businesses/by-category', 'businesses list'],
    ['/search?q=plumb', 'search'],
  ];

  for (const [path, name] of endpoints) {
    const res = http.get(`${BASE_URL}${path}`);
    check(res, {
      [`${name}: status is 200`]: (r) => r.status === 200,
      [`${name}: has body`]: (r) => r.body && r.body.length > 0,
    });
  }

  sleep(1);
}
