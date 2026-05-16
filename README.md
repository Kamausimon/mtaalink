# MtaaLink API

A Kenyan marketplace backend that connects **clients**, **service providers**, and **businesses**. Built with Rust/Axum for high performance and low resource usage.

---

## Features

- **Authentication** — JWT-based auth with password reset via email
- **Bookings** — Full booking lifecycle with status management and rescheduling
- **M-Pesa Payments** — STK Push via Safaricom Daraja API, automatic booking confirmation on payment
- **SMS Notifications** — Africa's Talking integration for booking and payment alerts
- **Real-Time Messaging** — WebSocket-based chat with persistent message history
- **In-App Notifications** — Persistent notification feed with real-time WebSocket push
- **Search & Discovery** — PostgreSQL full-text search + haversine geo-proximity
- **Provider Availability** — Weekly schedule management with open-slot generator
- **Analytics Dashboard** — Bookings over time, revenue, top services, repeat client rate
- **Wallet & Payouts** — Earnings ledger auto-credited on payment, payout request flow
- **Verified Reviews** — Reviews gated behind a completed booking
- **Service Packages** — Bundle multiple services at a discounted price
- **Post Feed & Interactions** — Provider/business posts with likes and comments
- **Favourites Alerts** — Real-time notifications when a followed provider posts
- **Booking Reminders** — Automated 24-hour reminder SMS + notification
- **File Storage** — Local disk or AWS S3, switchable via env var

---

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust 2024 edition |
| Web framework | Axum 0.7 |
| Database | PostgreSQL 17 via SQLx 0.7 (compile-time checked queries) |
| Auth | JWT (jsonwebtoken) + Argon2 password hashing |
| Email | lettre (SMTP — Gmail, Mailgun, SendGrid, etc.) |
| SMS | Africa's Talking REST API |
| Payments | Safaricom M-Pesa Daraja API (STK Push) |
| Storage | Local filesystem or AWS S3 (manual AWS v4 signing, no SDK required) |
| WebSocket | Axum built-in WS + tokio broadcast channels |
| Rate limiting | tower-governor |
| Migrations | sqlx migrate (auto-run at startup) |

---

## Project Structure

```
src/
├── main.rs                  # Server setup, middleware, route mounting
├── errors.rs                # AppError enum, AppResult type alias
├── extractors/
│   ├── current_user.rs      # JWT extractor → CurrentUser { user_id: i32 }
│   └── administrator.rs     # Admin middleware guard
├── routes/
│   ├── auth.rs              # Register, login, password reset
│   ├── clients.rs           # Client profile
│   ├── service_providers.rs # Provider profile & photos
│   ├── businesses.rs        # Business profile & photos
│   ├── bookings.rs          # Booking lifecycle
│   ├── payments.rs          # M-Pesa STK Push & callback
│   ├── reviews.rs           # Reviews + replies (verified)
│   ├── services.rs          # Services CRUD
│   ├── packages.rs          # Service packages
│   ├── posts.rs             # Feed posts + likes/comments
│   ├── messages.rs          # Messaging
│   ├── favorites.rs         # Favourites
│   ├── notifications.rs     # Notification feed
│   ├── availability.rs      # Provider schedule & slots
│   ├── analytics.rs         # Provider/business analytics
│   ├── wallet.rs            # Earnings & payout requests
│   ├── search.rs            # Full-text + geo search
│   ├── locations.rs         # Provider/business locations
│   ├── categories.rs        # Service categories
│   ├── admin.rs             # Admin operations
│   ├── dashboard.rs         # Summary dashboard
│   └── ws.rs                # WebSocket endpoint
└── utils/
    ├── jwt.rs               # Token creation/verification
    ├── email.rs             # SMTP email sender
    ├── sms.rs               # Africa's Talking SMS
    ├── mpesa.rs             # Daraja STK Push
    ├── storage.rs           # Local/S3 storage backend
    ├── notifications.rs     # DB + WS notification helpers
    ├── wallet.rs            # Wallet credit utility
    ├── reminders.rs         # Background reminder task
    ├── ws_state.rs          # WebSocket connection registry
    ├── image_upload.rs      # Multipart image parsing
    └── attachments.rs       # Post attachment handling

migrations/
├── 0001_initial_schema.sql        # Full 23-table schema
├── 0002_add_review_replies.sql
├── 0003_add_content_flags.sql
├── 0004_add_payments.sql
├── 0005_search_indexes.sql
├── 0006_verified_reviews.sql
├── 0007_notifications.sql
├── 0008_booking_reminder_sent.sql
├── 0009_wallet.sql
├── 0010_client_profile.sql
├── 0011_service_packages.sql
└── 0012_post_interactions.sql
```

---

## Prerequisites

- [Rust](https://rustup.rs/) (stable, 2024 edition)
- PostgreSQL 14+
- (Optional) An [Africa's Talking](https://africastalking.com) account for SMS
- (Optional) A [Safaricom Daraja](https://developer.safaricom.co.ke) account for M-Pesa
- (Optional) AWS S3 bucket for file storage

---

## Quick Start

```bash
# 1. Clone the repository
git clone https://github.com/Kamausimon/mtaalink.git
cd mtaalink

# 2. Copy the example env file and fill in your values
cp .env.example .env

# 3. Create the database
psql -U postgres -c "CREATE DATABASE mtaalink;"

# 4. Run the server — migrations apply automatically on startup
cargo run
```

The API will be available at `http://localhost:7878`.

---

## Environment Variables

Copy `.env.example` to `.env` and configure:

### Required

| Variable | Description |
|---|---|
| `DATABASE_URL` | PostgreSQL connection string |
| `JWT_SECRET` | Secret key for signing JWTs — generate with: `openssl rand -hex 64` |

### Server

| Variable | Default | Description |
|---|---|---|
| `PORT` | `7878` | Port the server listens on |
| `FRONTEND_URL` | `http://localhost:3000` | Allowed CORS origin |

### Email (password reset)

| Variable | Description |
|---|---|
| `SMTP_HOST` | SMTP server hostname (e.g. `smtp.gmail.com`) |
| `SMTP_PORT` | SMTP port (usually `587` for TLS) |
| `SMTP_USER` | SMTP login username |
| `SMTP_PASSWORD` | SMTP password or app password |
| `FROM_EMAIL` | Sender address |
| `FROM_NAME` | Sender display name |
| `APP_URL` | Base URL used to build password reset links |

### SMS — Africa's Talking

| Variable | Description |
|---|---|
| `AT_ENV` | `sandbox` or `production` |
| `AT_API_KEY` | Your Africa's Talking API key |
| `AT_USERNAME` | Your AT username (`sandbox` for testing) |
| `AT_SENDER_ID` | Optional approved sender ID |

### M-Pesa — Daraja API

| Variable | Description |
|---|---|
| `MPESA_ENV` | `sandbox` or `production` |
| `MPESA_CONSUMER_KEY` | Daraja app consumer key |
| `MPESA_CONSUMER_SECRET` | Daraja app consumer secret |
| `MPESA_SHORTCODE` | Business shortcode (sandbox: `174379`) |
| `MPESA_PASSKEY` | Lipa Na M-Pesa passkey |
| `MPESA_CALLBACK_URL` | Public HTTPS URL for Safaricom to POST results to |

### File Storage

| Variable | Default | Description |
|---|---|---|
| `STORAGE_BACKEND` | `local` | `local` or `s3` |
| `AWS_REGION` | — | S3 bucket region |
| `AWS_S3_BUCKET` | — | S3 bucket name |
| `AWS_ACCESS_KEY_ID` | — | AWS access key |
| `AWS_SECRET_ACCESS_KEY` | — | AWS secret key |
| `AWS_S3_BASE_URL` | — | Public base URL for uploaded files |

---

## API Reference

All endpoints return JSON. Authenticated endpoints require:
```
Authorization: Bearer <jwt_token>
```

Error responses always follow this shape:
```json
{ "message": "Description of what went wrong" }
```

---

### Authentication `/auth`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/auth/register` | No | Register a new user |
| `POST` | `/auth/login` | No | Login — returns JWT token |
| `GET` | `/auth/me` | Yes | Get current user info |
| `POST` | `/auth/forgot-password` | No | Send password reset email |
| `POST` | `/auth/reset-password` | No | Reset password with token from email |

> `/register`, `/login`, `/forgot-password` are rate-limited to **5 requests / minute per IP**.

---

### Client Profiles `/clients`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/clients/uploadProfilePicture` | Yes | Upload profile picture (multipart) |
| `GET` | `/clients/me/profile` | Yes | Get own profile (phone, bio, location) |
| `PUT` | `/clients/me/profile` | Yes | Update phone, bio, or location (omit fields to keep existing values) |

---

### Service Providers `/service_providers`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/service_providers/createProvider` | Yes | Create provider profile |
| `GET` | `/service_providers/listProviders` | No | List all approved providers |
| `GET` | `/service_providers/getProvider/:id` | No | Get provider by ID |
| `POST` | `/service_providers/updateProvider` | Yes | Update provider profile |
| `POST` | `/service_providers/uploadProfilePhoto` | Yes | Upload profile photo |
| `POST` | `/service_providers/uploadCoverPhoto` | Yes | Upload cover photo |

---

### Businesses `/businesses`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/businesses/createBusiness` | Yes | Create business profile |
| `GET` | `/businesses/listBusinesses` | No | List all verified businesses |
| `GET` | `/businesses/getBusiness/:id` | No | Get business by ID |
| `POST` | `/businesses/updateBusiness` | Yes | Update business profile |
| `POST` | `/businesses/uploadLogo` | Yes | Upload logo |
| `POST` | `/businesses/uploadProfilePhoto` | Yes | Upload profile photo |
| `POST` | `/businesses/uploadCoverPhoto` | Yes | Upload cover photo |

---

### Services `/services`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/services/createService` | Yes | Create a service |
| `GET` | `/services/getServices` | No | List services `?target_type=&target_id=` |
| `GET` | `/services/:id` | No | Get service by ID |
| `PUT` | `/services/:id` | Yes | Update service |
| `DELETE` | `/services/:id` | Yes | Delete service |

---

### Service Packages `/packages`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/packages` | Yes | Create a package with bundled services |
| `GET` | `/packages/:id` | No | Get package with included services and individual pricing |
| `PUT` | `/packages/:id` | Yes (owner) | Update name, description, price, or active state |
| `DELETE` | `/packages/:id` | Yes (owner) | Delete package |
| `POST` | `/packages/:id/items` | Yes (owner) | Add a service to the package |
| `DELETE` | `/packages/:id/items` | Yes (owner) | Remove a service from the package |

---

### Bookings `/bookings`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/bookings/createBooking` | Yes | Create a booking |
| `GET` | `/bookings/getBookings/me` | Yes | Client's own bookings `?status=&target_type=` |
| `GET` | `/bookings/getBookings/received` | Yes | Bookings received as a provider/business |
| `GET` | `/bookings/:id` | Yes | Get booking by ID |
| `POST` | `/bookings/:id/status` | Yes (owner) | Update booking status |
| `POST` | `/bookings/:id/reschedule` | Yes (client) | Reschedule a booking |
| `POST` | `/bookings/:id/delete` | Yes (client) | Delete a booking |

---

### Payments `/payments`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/payments/initiate` | Yes | Initiate M-Pesa STK Push prompt |
| `POST` | `/payments/mpesa/callback` | No | M-Pesa result callback (called by Safaricom) |
| `GET` | `/payments/booking/:booking_id` | Yes | Get payment status for a booking |

On a successful payment: booking is auto-confirmed, provider/business wallet is credited, and a notification is pushed.

---

### Reviews `/reviews`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/reviews/createReviews` | Yes | Create a verified review (requires a completed booking) |
| `GET` | `/reviews/getReviews` | No | Get reviews `?target_type=&target_id=` |
| `GET` | `/reviews/rankProviders` | No | Providers ranked by average rating |
| `GET` | `/reviews/rankBusinesses` | No | Businesses ranked by average rating |
| `GET` | `/reviews/getReviewAggById` | No | Aggregate rating for one provider/business |
| `POST` | `/reviews/:id/replyReview` | Yes | Reply to a review (one reply per user per review) |

Reviews carry a `"verified": true` flag when backed by a completed booking.

---

### Provider Availability `/availability`

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/availability/provider/:id` | No | Get provider's weekly schedule |
| `PUT` | `/availability/provider/:id` | Yes (owner) | Set/replace weekly schedule |
| `GET` | `/availability/provider/:id/slots` | No | Open slots for a date `?date=YYYY-MM-DD&slot_minutes=60` |

**Set schedule request body:**
```json
[
  { "day": "Monday",   "start_time": "09:00", "end_time": "17:00", "is_available": true },
  { "day": "Saturday", "is_available": false }
]
```

---

### Favorites `/favorites`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/favorites/addFavorite` | Yes | Add a provider/business to favourites |
| `DELETE` | `/favorites/removeFavorite` | Yes | Remove from favourites |
| `GET` | `/favorites/getFavorites` | Yes | List own favourites |

Users receive a notification when a favourited provider/business creates a new post.

---

### Posts & Feed `/posts`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/posts/createPosts` | Yes | Create a post (notifies all favouriters) |
| `GET` | `/posts/getAllPosts` | No | List all posts |
| `GET` | `/posts/getPost/:id` | No | Get post by ID |
| `GET` | `/posts/provider/:id/posts` | No | Posts by provider |
| `GET` | `/posts/business/:id/posts` | No | Posts by business |
| `POST` | `/posts/updatePost/:id` | Yes | Update a post |
| `POST` | `/posts/deletePost/:id` | Yes | Delete a post |
| `POST` | `/posts/:id/like` | Yes | Like a post |
| `DELETE` | `/posts/:id/like` | Yes | Unlike a post |
| `GET` | `/posts/:id/comments` | No | Get comments and total like count |
| `POST` | `/posts/:id/comments` | Yes | Add a comment |
| `DELETE` | `/posts/:id/comments/:comment_id` | Yes | Delete own comment |

---

### Messages `/messages`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/messages/sendMessage` | Yes | Send a message (pushes instantly to receiver's WebSocket) |
| `GET` | `/messages/getMessages` | Yes | Get message history |
| `POST` | `/messages/markMessagesAsRead` | Yes | Mark messages as read |
| `GET` | `/messages/unreadMessagesCount` | Yes | Get unread message count |
| `GET` | `/messages/conversations` | Yes | List all conversations |

---

### Notifications `/notifications`

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/notifications` | Yes | List notifications `?unread_only=true&page=1&per_page=20` |
| `GET` | `/notifications/unread-count` | Yes | Unread count (for bell badge) |
| `POST` | `/notifications/:id/read` | Yes | Mark one notification as read |
| `POST` | `/notifications/read-all` | Yes | Mark all as read |
| `DELETE` | `/notifications/:id` | Yes | Delete a notification |

**Notification types:** `booking_created`, `booking_confirmed`, `booking_cancelled`, `booking_reminder`, `payment_received`, `payment_failed`, `new_message`, `new_review`, `review_reply`, `new_post`

---

### Analytics `/analytics`

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/analytics/:target_type/:target_id` | Yes (owner) | Analytics dashboard `?days=30` |

Response includes: booking counts by status, total revenue, average rating, bookings over time (daily), revenue over time (daily), top 10 services by bookings and revenue, and repeat client rate.

---

### Wallet `/wallet`

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/wallet/:target_type/:target_id` | Yes (owner) | Balance, total earned, total paid out |
| `GET` | `/wallet/:target_type/:target_id/transactions` | Yes (owner) | Paginated ledger of credits and payouts |
| `POST` | `/wallet/:target_type/:target_id/payout` | Yes (owner) | Request a payout to M-Pesa number |
| `GET` | `/wallet/:target_type/:target_id/payouts` | Yes (owner) | List payout requests and statuses |

Wallets are created automatically and credited when an M-Pesa payment completes.

---

### Search `/search`

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/search` | No | Search providers and businesses |

| Query param | Description |
|---|---|
| `q` | Free-text query matched against name, description, category |
| `type` | `provider` or `business` — omit to search both |
| `category` | Partial, case-insensitive category filter |
| `lat` + `lng` | User coordinates for geo-proximity |
| `radius_km` | Search radius in km (default `10`) |
| `page` + `per_page` | Pagination (max 100 per page) |

Results are sorted by text relevance (when `q` is given), then average rating. Each result includes a `distance_km` field when geo params are provided.

---

### Locations `/locations`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/locations/provider` | Yes | Add provider location |
| `GET` | `/locations/provider/:id` | No | Get provider locations |
| `PUT` | `/locations/provider/:id` | Yes | Update provider location |
| `DELETE` | `/locations/provider/:id` | Yes | Delete provider location |
| `POST` | `/locations/business` | Yes | Add business branch |
| `GET` | `/locations/business/:id` | No | Get business branches |
| `PUT` | `/locations/business/:id` | Yes | Update business branch |
| `DELETE` | `/locations/business/:id` | Yes | Delete business branch |

---

### Categories `/categories`

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/categories/getCategories` | No | List all categories |
| `POST` | `/categories/assignCategories` | Yes | Assign categories to a provider/business |

---

### Attachments `/attachments`

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/attachments/upload` | Yes | Upload a file attachment for a post |

---

### Admin `/admin`

All admin routes require an admin account.

| Method | Path | Description |
|---|---|---|
| `GET` | `/admin/dashboard` | Platform-wide stats: users, bookings, revenue, payout totals |
| `GET` | `/admin/users` | List all users |
| `POST` | `/admin/delete_user` | Delete a user |
| `GET` | `/admin/categories` | List all categories |
| `POST` | `/admin/create_category` | Create a category |
| `POST` | `/admin/create_parent_category` | Create a parent category |
| `POST` | `/admin/delete_category` | Delete a category |
| `GET` | `/admin/userAnalytics` | User growth and role breakdown |
| `POST` | `/admin/flagContent` | Flag a piece of content for review |
| `POST` | `/admin/resolveFlag` | Resolve a content flag |
| `GET` | `/admin/moderateReviews` | List flagged reviews ordered by flag count |
| `GET` | `/admin/payouts` | List all pending payout requests |
| `POST` | `/admin/payouts/:id/approve` | Approve a payout request |
| `POST` | `/admin/payouts/:id/reject` | Reject a payout and refund the balance |

---

### WebSocket `/ws`

```
GET /ws?token=<jwt_token>
```

Connect with the JWT as a query parameter (browsers cannot set `Authorization` headers on WebSocket connections).

On successful connection the server sends:
```json
{ "event": "connected", "user_id": 42 }
```

**Events pushed by the server:**

| Event | Trigger |
|---|---|
| `new_message` | A message is sent to this user |
| `notification` | Any notification is created for this user (booking updates, payments, reviews, posts) |

**Example (JavaScript):**
```javascript
const ws = new WebSocket(`ws://localhost:7878/ws?token=${jwt}`);

ws.onmessage = (e) => {
  const { event, data } = JSON.parse(e.data);

  if (event === 'new_message') {
    // data: { id, sender_id, content, target_type, target_id, created_at }
    renderMessage(data);
  }

  if (event === 'notification') {
    // data: { notif_type, title, body, target_type, target_id }
    showNotificationBanner(data);
  }
};
```

---

## Background Tasks

### Booking Reminders

A background task starts with the server and runs every **15 minutes**. It scans for bookings scheduled 23–25 hours from now that have not yet been reminded, then:

1. Creates an in-app notification for the client
2. Sends an SMS if the client has a phone on file (stored from M-Pesa payment)
3. Sets `reminder_sent = true` on the booking to prevent duplicates

No configuration required — fully automatic.

---

## Database Migrations

Migrations in `migrations/` apply automatically at server startup via `sqlx migrate`. They are numbered, ordered, and idempotent.

To run manually:
```bash
cargo sqlx migrate run
```

---

## File Storage

### Local (default)

Uploaded files are saved under `uploads/` and served at `/uploads/<path>`.

```env
STORAGE_BACKEND=local
```

### AWS S3

```env
STORAGE_BACKEND=s3
AWS_REGION=us-east-1
AWS_S3_BUCKET=your-bucket-name
AWS_ACCESS_KEY_ID=...
AWS_SECRET_ACCESS_KEY=...
AWS_S3_BASE_URL=https://your-bucket.s3.us-east-1.amazonaws.com
```

No AWS SDK is required — S3 request signing is implemented directly using SHA-256/HMAC.

---

## Rate Limiting

| Scope | Limit | Applied to |
|---|---|---|
| Auth (strict) | 5 requests / minute per IP | `/auth/login`, `/auth/register`, `/auth/forgot-password` |
| Global | 100 requests / minute per IP | All other endpoints |

Exceeded limits return `429 { "message": "Too many requests" }`.

---

## Deployment Notes

- Set `MPESA_CALLBACK_URL` to a publicly reachable HTTPS URL — Safaricom cannot reach `localhost`
- Use a reverse proxy (nginx / Caddy) to terminate TLS before forwarding to port `7878`
- Set `FRONTEND_URL` to your production frontend origin for CORS
- Generate a strong `JWT_SECRET`: `openssl rand -hex 64`
- Set `AT_ENV=production` and `MPESA_ENV=production` for live traffic
- The server binds to `127.0.0.1` by default — the reverse proxy should forward to it

---

## License

MIT
