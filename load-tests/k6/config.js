// Shared config for all k6 scripts in this directory.
// Override via environment variables, e.g.:
//   k6 run -e BASE_URL=http://localhost:7878 -e TEST_EMAIL=foo@bar.com -e TEST_PASSWORD=secret smoke.js

export const BASE_URL = __ENV.BASE_URL || 'http://localhost:7878';

// Credentials for an existing, verified test account (used by auth-flow.js).
export const TEST_EMAIL = __ENV.TEST_EMAIL || '';
export const TEST_PASSWORD = __ENV.TEST_PASSWORD || '';

export const JSON_HEADERS = { headers: { 'Content-Type': 'application/json' } };

export function authHeaders(token) {
  return { headers: { 'Content-Type': 'application/json', Authorization: `Bearer ${token}` } };
}
