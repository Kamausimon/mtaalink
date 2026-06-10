// Load test for a logged-in user's "dashboard session": login once, then
// repeatedly hit the authenticated endpoints a real session would poll
// (profile, dashboard summary, bookings, notifications, messages, favorites).
//
// Requires an existing, verified test account (any role). Provide its
// credentials via env vars:
//
//   k6 run -e TEST_EMAIL=client@example.com -e TEST_PASSWORD=secret123 load-tests/k6/auth-flow.js
//
// All VUs share a single login token (obtained once in setup()), so this
// test measures read-path load rather than login throughput. See
// login-stress.js for hammering the login endpoint itself.

import http from 'k6/http';
import { check, group, sleep, fail } from 'k6';
import { BASE_URL, TEST_EMAIL, TEST_PASSWORD, JSON_HEADERS, authHeaders } from './config.js';

export const options = {
  scenarios: {
    dashboard_session: {
      executor: 'ramping-vus',
      startVUs: 0,
      stages: [
        { duration: '30s', target: 10 },
        { duration: '1m', target: 25 },
        { duration: '1m', target: 25 },
        { duration: '30s', target: 0 },
      ],
    },
  },
  thresholds: {
    http_req_failed: ['rate<0.01'],
    http_req_duration: ['p(95)<800'],
  },
};

export function setup() {
  if (!TEST_EMAIL || !TEST_PASSWORD) {
    fail('Set TEST_EMAIL and TEST_PASSWORD env vars to an existing verified account before running this test.');
  }

  const res = http.post(
    `${BASE_URL}/auth/login`,
    JSON.stringify({ email: TEST_EMAIL, password: TEST_PASSWORD }),
    JSON_HEADERS,
  );

  check(res, { 'login: status 200': (r) => r.status === 200 });
  const token = res.json('token');
  if (!token) fail('Login did not return a token — check TEST_EMAIL/TEST_PASSWORD.');

  return { token };
}

export default function (data) {
  const auth = authHeaders(data.token);

  group('me + dashboard', () => {
    const me = http.get(`${BASE_URL}/auth/me`, auth);
    check(me, { 'me: status 200': (r) => r.status === 200 });

    const dash = http.get(`${BASE_URL}/dashboard`, auth);
    check(dash, { 'dashboard: status 200': (r) => r.status === 200 });
  });

  group('bookings', () => {
    const res = http.get(`${BASE_URL}/bookings/getBookings/me`, auth);
    check(res, { 'my bookings: status 200': (r) => r.status === 200 });
  });

  group('notifications', () => {
    const list = http.get(`${BASE_URL}/notifications`, auth);
    check(list, { 'notifications: status 200': (r) => r.status === 200 });

    const unread = http.get(`${BASE_URL}/notifications/unread-count`, auth);
    check(unread, { 'unread count: status 200': (r) => r.status === 200 });
  });

  group('messages + favorites', () => {
    const conv = http.get(`${BASE_URL}/messages/conversations`, auth);
    check(conv, { 'conversations: status 200': (r) => r.status === 200 });

    const favs = http.get(`${BASE_URL}/favorites/getFavorites`, auth);
    check(favs, { 'favorites: status 200': (r) => r.status === 200 });
  });

  sleep(Math.random() * 2 + 1); // think time: 1-3s
}
