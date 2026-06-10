# Load testing with k6

## Install k6

k6 isn't installed in this environment. Pick one:

- **winget (Windows):** `winget install k6.k6`
- **Chocolatey:** `choco install k6`
- **Manual:** download the binary from https://k6.io/docs/get-started/installation/

Verify with `k6 version`.

## Scripts

All scripts live in `load-tests/k6/` and read config from environment variables
(see `config.js`). Make sure your backend (`cargo run`) is running first.

| Script | What it tests | Auth needed? |
| --- | --- | --- |
| `smoke.js` | Quick sanity check of core public endpoints (1 VU, 5 iterations) | No |
| `public-browse.js` | Visitor flow: search, categories, listings, profile pages, ramping to 30 VUs | No |
| `auth-flow.js` | Logged-in session: `/auth/me`, dashboard, bookings, notifications, messages, favorites, ramping to 25 VUs | Yes |
| `login-stress.js` | Hammers `/auth/login` alone (Argon2 hashing is CPU-bound and usually the slowest endpoint) | Yes |

## Running

```sh
# Smoke test against local backend
k6 run load-tests/k6/smoke.js

# Public browsing load test
k6 run load-tests/k6/public-browse.js

# Authenticated flows — needs a real, verified test account
k6 run -e TEST_EMAIL=client@example.com -e TEST_PASSWORD=yourpassword load-tests/k6/auth-flow.js
k6 run -e TEST_EMAIL=client@example.com -e TEST_PASSWORD=yourpassword load-tests/k6/login-stress.js

# Point at a different environment
k6 run -e BASE_URL=https://api.mtaalink.com load-tests/k6/public-browse.js

# Override the default load shape
k6 run --vus 50 --duration 2m load-tests/k6/public-browse.js
```

## Notes

- `public-browse.js` and `auth-flow.js` only call read (`GET`) endpoints, so
  they're safe to run repeatedly without polluting data. There's no
  write-path test (e.g. creating bookings/messages) included — that would
  need a dedicated test account/database since it leaves real rows behind.
- Use a **test account**, not a real user, for `TEST_EMAIL`/`TEST_PASSWORD`.
- `public-browse.js` fetches real provider/business IDs in `setup()` so
  profile-detail requests hit real records. If your DB is empty, those
  checks are simply skipped.
- Thresholds (`http_req_duration`, `http_req_failed`) are starting points —
  tune them once you have a baseline for your hardware/DB.
