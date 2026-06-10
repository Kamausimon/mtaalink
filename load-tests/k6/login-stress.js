// Stress test for the login endpoint specifically. Argon2 password hashing
// is CPU-bound by design, so login is usually the most expensive endpoint
// per-request — this isolates it from the rest of the API.
//
// Requires an existing, verified test account:
//   k6 run -e TEST_EMAIL=client@example.com -e TEST_PASSWORD=secret123 load-tests/k6/login-stress.js

import http from 'k6/http';
import { check, sleep, fail } from 'k6';
import { BASE_URL, TEST_EMAIL, TEST_PASSWORD, JSON_HEADERS } from './config.js';

export const options = {
  scenarios: {
    login: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '20s', target: 5 },
        { duration: '40s', target: 20 },
        { duration: '20s', target: 0 },
      ],
    },
  },
  thresholds: {
    http_req_failed: ['rate<0.01'],
    // Argon2 hashing is slow on purpose; this threshold is generous.
    http_req_duration: ['p(95)<2000'],
  },
};

export default function () {
  if (!TEST_EMAIL || !TEST_PASSWORD) {
    fail('Set TEST_EMAIL and TEST_PASSWORD env vars to an existing verified account before running this test.');
  }

  const res = http.post(
    `${BASE_URL}/auth/login`,
    JSON.stringify({ email: TEST_EMAIL, password: TEST_PASSWORD }),
    JSON_HEADERS,
  );

  check(res, {
    'login: status 200': (r) => r.status === 200,
    'login: returns token': (r) => !!r.json('token'),
  });

  sleep(1);
}
