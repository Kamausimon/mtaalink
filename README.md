# MtaaLink API

Hyperlocal Kenyan service marketplace backend. Connects clients with service providers and businesses — bookings, M-Pesa payments, real-time messaging, reviews, analytics, and more.

Built with **Rust / Axum 0.7 / SQLx 0.7 / PostgreSQL**.

---

## Table of Contents

- [Quick Start](#quick-start)
- [Environment Variables](#environment-variables)
- [Authentication](#authentication)
- [API Reference](#api-reference)
  - [Auth](#auth)
  - [Dashboard](#dashboard)
  - [Clients](#clients)
  - [Service Providers](#service-providers)
  - [Businesses](#businesses)
  - [Services](#services)
  - [Service Packages](#service-packages)
  - [Bookings](#bookings)
  - [Payments (M-Pesa)](#payments-m-pesa)
  - [Wallet & Earnings](#wallet--earnings)
  - [Reviews](#reviews)
  - [Search & Discovery](#search--discovery)
  - [Categories](#categories)
  - [Locations](#locations)
  - [Availability](#availability)
  - [Analytics](#analytics)
  - [Posts & Interactions](#posts--interactions)
  - [Messages](#messages)
  - [Notifications](#notifications)
  - [Favorites](#favorites)
  - [Admin](#admin)
  - [WebSocket](#websocket)
- [User Roles](#user-roles)
- [Error Format](#error-format)

---

## Quick Start

```bash
# 1. Copy and fill environment variables
cp .env.example .env

# 2. Run the server — migrations apply automatically on startup
cargo run

# Server starts on http://127.0.0.1:7878
```

---

## Environment Variables

| Variable | Required | Description |
|---|---|---|
| `DATABASE_URL` | Yes | `postgres://user:pass@host:5432/db` |
| `JWT_SECRET` | Yes | Secret key for signing JWTs |
| `FRONTEND_URL` | No | CORS allowed origin (default: `http://localhost:3000`) |
| `PORT` | No | Server port (default: `7878`) |
| `SMTP_HOST` | No | SMTP server for email |
| `SMTP_PORT` | No | SMTP port (default: 587) |
| `SMTP_USER` | No | SMTP username |
| `SMTP_PASS` | No | SMTP password |
| `SMTP_FROM` | No | From address for outgoing emails |
| `AT_API_KEY` | No | Africa's Talking API key (SMS) |
| `AT_USERNAME` | No | Africa's Talking username |
| `AT_SENDER_ID` | No | SMS sender ID |
| `MPESA_CONSUMER_KEY` | No | M-Pesa consumer key |
| `MPESA_CONSUMER_SECRET` | No | M-Pesa consumer secret |
| `MPESA_SHORTCODE` | No | M-Pesa business shortcode |
| `MPESA_PASSKEY` | No | M-Pesa passkey |
| `MPESA_CALLBACK_URL` | No | Public HTTPS URL for M-Pesa callback |
| `AWS_ACCESS_KEY_ID` | No | S3 file storage key |
| `AWS_SECRET_ACCESS_KEY` | No | S3 file storage secret |
| `AWS_REGION` | No | S3 region |
| `S3_BUCKET` | No | S3 bucket name |

---

## Authentication

All protected endpoints require a `Bearer` token in the `Authorization` header:

```
Authorization: Bearer <jwt_token>
```

Tokens are returned on `/auth/register` and `/auth/login`.

---

## API Reference

### Auth

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/auth/register` | No | Register a new user |
| `POST` | `/auth/login` | No | Login and get JWT |
| `GET` | `/auth/me` | Yes | Get current user info |
| `POST` | `/auth/forgot-password` | No | Request password reset email |
| `POST` | `/auth/reset-password` | No | Reset password with token |

**Register body:**
```json
{
  "username": "john",
  "email": "john@example.com",
  "password": "secret123",
  "role": "client"
}
```
Roles: `client`, `provider`, `business`

---

### Dashboard

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/dashboard` | Yes | Role-aware dashboard summary |

Returns upcoming bookings and unread notifications for all users. For providers/businesses also returns: wallet balance, total earned, and pending bookings count.

---

### Clients

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/clients/me/profile` | Yes | Get own client profile |
| `PUT` | `/clients/me/profile` | Yes | Update own client profile |
| `POST` | `/clients/uploadProfilePicture` | Yes | Upload profile picture |

---

### Service Providers

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/service_providers/onboard` | Yes | Complete provider onboarding |
| `GET` | `/service_providers/listProviders` | Yes | List providers |
| `POST` | `/service_providers/updateProfile` | Yes | Update provider profile |
| `POST` | `/service_providers/uploadProfilePhoto` | Yes | Upload profile photo |
| `POST` | `/service_providers/uploadCoverPhoto` | Yes | Upload cover photo |
| `GET` | `/service_providers/getProviderData` | Yes | Get own provider data |
| `POST` | `/service_providers/updateAvailability` | Yes | Update single-day availability |
| `POST` | `/service_providers/updateBulkAvailability` | Yes | Update full-week availability |
| `GET` | `/service_providers/getAvailability` | Yes | Get own availability schedule |

**List providers query params:** `?category=plumbing&location=Nairobi`

---

### Businesses

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/businesses/onboard` | Yes | Complete business onboarding |
| `GET` | `/businesses/listBusinesses` | Yes | List businesses |
| `POST` | `/businesses/updateProfile` | Yes | Update business profile |
| `POST` | `/businesses/uploadLogo` | Yes | Upload logo |
| `POST` | `/businesses/uploadProfilePicture` | Yes | Upload profile picture |
| `POST` | `/businesses/uploadCoverPhoto` | Yes | Upload cover photo |

---

### Services

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/services/createService` | Yes | Create a service |
| `GET` | `/services/getServices` | Yes | List services |
| `POST` | `/services/updateService` | Yes | Update a service |
| `POST` | `/services/deleteService` | Yes | Delete a service |

**Create service body:**
```json
{
  "title": "Plumbing repair",
  "description": "Fix leaking pipes",
  "price": 1500,
  "duration": 60,
  "target_type": "provider",
  "target_id": 1,
  "category_id": 3
}
```

---

### Service Packages

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/packages` | Yes | Create a service package |
| `GET` | `/packages/:id` | Yes | Get a package |
| `PUT` | `/packages/:id` | Yes | Update a package |
| `DELETE` | `/packages/:id` | Yes | Delete a package |
| `POST` | `/packages/:id/items` | Yes | Add a service to a package |
| `DELETE` | `/packages/:id/items` | Yes | Remove a service from a package |

---

### Bookings

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/bookings/createBooking` | Yes | Create a new booking |
| `GET` | `/bookings/getBookings/me` | Yes | Get client own bookings |
| `GET` | `/bookings/getBookings/received` | Yes | Get bookings received (provider/business) |
| `GET` | `/bookings/:id` | Yes | Get a single booking |
| `POST` | `/bookings/:id/status` | Yes | Update booking status |
| `POST` | `/bookings/:id/delete` | Yes | Delete a booking (client only) |
| `POST` | `/bookings/:id/reschedule` | Yes | Reschedule a booking |

**Create booking body:**
```json
{
  "target_type": "provider",
  "target_id": 1,
  "service_id": 2,
  "service_description": "Fix kitchen sink",
  "scheduled_time": "2026-06-15T10:00:00",
  "client_phone": "0712345678"
}
```

**Get my bookings query params:** `?status=confirmed&target_type=provider`

**Get received bookings query params:** `?target_type=provider&target_id=1&status=pending`

**Update status body:** `{ "status": "confirmed" }` — valid statuses: `pending`, `confirmed`, `completed`, `cancelled`

On status change to `confirmed` or `cancelled`: client receives email, SMS, and in-app notification.

---

### Payments (M-Pesa)

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/payments/initiate` | Yes | Initiate M-Pesa STK Push |
| `POST` | `/payments/mpesa/callback` | No | M-Pesa callback (Safaricom webhook) |
| `GET` | `/payments/booking/:booking_id` | Yes | Get payment status for a booking |

**Initiate payment body:**
```json
{
  "booking_id": 1,
  "phone_number": "0712345678",
  "amount": 1500
}
```

Accepts phone formats: `07XX`, `+2547XX`, `2547XX`. On successful payment: booking is auto-confirmed and provider/business wallet is credited.

---

### Wallet & Earnings

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/wallet/:target_type/:target_id` | Yes | Get wallet balance |
| `GET` | `/wallet/:target_type/:target_id/transactions` | Yes | Get transaction history |
| `POST` | `/wallet/:target_type/:target_id/payout` | Yes | Request a payout |
| `GET` | `/wallet/:target_type/:target_id/payouts` | Yes | List payout requests |

`target_type`: `provider` or `business`. Only the owner can access their own wallet.

**Request payout body:**
```json
{
  "amount": 5000,
  "phone_number": "0712345678"
}
```

Payouts stay `pending` until an admin approves them via the admin panel.

---

### Reviews

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/reviews/createReviews` | Yes | Leave a review |
| `GET` | `/reviews/getReviews` | No | Get reviews for a target |
| `GET` | `/reviews/rankProviders` | No | Rank all providers by rating |
| `GET` | `/reviews/rankBusinesses` | No | Rank all businesses by rating |
| `GET` | `/reviews/getReviewAggById` | No | Get rating aggregate for one target |
| `POST` | `/reviews/:id/replyReview` | Yes | Owner replies to a review |

**Create review query params:** `?target_type=provider&target_id=1`

**Create review body:**
```json
{ "comment": "Excellent work!", "rating": 5 }
```

Reviews require a `completed` booking with that provider/business. A `verified: true` badge is returned when the booking link is confirmed.

---

### Search & Discovery

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/search` | No | Full-text + geo-proximity search |

**Query params:**

| Param | Type | Description |
|---|---|---|
| `q` | string | Search text (name, description, category) |
| `category` | string | Filter by category name |
| `lat` | float | Your latitude |
| `lng` | float | Your longitude |
| `radius_km` | int | Search radius in km (default: 10) |
| `page` | int | Page number (default: 1) |
| `per_page` | int | Results per page (default: 20, max: 100) |

Returns providers and businesses ranked by full-text relevance, then average rating. Only approved/verified profiles appear.

---

### Categories

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/categories/allCategories` | No | List all categories with parent |
| `GET` | `/categories/allcategories/:id/subcategories` | No | Get subcategories |
| `GET` | `/categories/providers/by-category` | No | Providers filtered by category |
| `GET` | `/categories/businesses/by-category` | No | Businesses filtered by category |
| `POST` | `/categories/assignCategories` | Yes | Assign categories to provider/business |

**Category filter query params:** `?category=1&subcategory=5`

---

### Locations

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/locations/allcounties` | No | List all 47 Kenyan counties |
| `GET` | `/locations/counties/:county_id/constituencies` | No | Constituencies in a county |
| `GET` | `/locations/constituencies/:constituency_id/wards` | No | Wards in a constituency |
| `POST` | `/locations/branches/:business_id/location` | Yes | Add a business branch location |
| `GET` | `/locations/branches/:business_id/locations` | No | Get all branches for a business |
| `GET` | `/locations/branches/location/:id` | No | Get a single branch |
| `POST` | `/locations/branches/location/:id/update` | Yes | Update a branch |
| `POST` | `/locations/branches/location/:id/delete` | Yes | Delete a branch |
| `POST` | `/locations/providers/:provider_id` | Yes | Add a provider location |
| `GET` | `/locations/providers/location/:id` | No | Get a provider location |
| `POST` | `/locations/providers/location/:id/update` | Yes | Update a provider location |
| `POST` | `/locations/providers/location/:id/delete` | Yes | Delete a provider location |
| `GET` | `/locations/search` | No | Search providers/businesses by location |

**Location search query params:** `?target_type=business&county_id=1&constituency_id=2&ward_id=5`

---

### Availability

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/availability/provider/:id` | No | Get provider weekly schedule |
| `PUT` | `/availability/provider/:id` | Yes | Replace full weekly schedule |
| `GET` | `/availability/provider/:id/slots` | No | Get available time slots for a date |

**Get slots query params:** `?date=2026-06-15&slot_minutes=60`

---

### Analytics

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/analytics/:target_type/:target_id` | Yes | Analytics dashboard (owner only) |

**Query params:** `?days=30` (range: 1-365, default: 30)

Returns: booking counts by status, revenue totals, average rating, top services by bookings, repeat client rate, time-series data for charts.

---

### Posts & Interactions

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/posts/createPosts` | Yes | Create a post (provider/business only) |
| `GET` | `/posts/getAllPosts` | No | List all posts |
| `GET` | `/posts/getPost/:id` | No | Get a single post |
| `GET` | `/posts/provider/:id/posts` | No | Posts by a provider |
| `GET` | `/posts/business/:id/posts` | No | Posts by a business |
| `POST` | `/posts/deletePost/:id` | Yes | Delete a post (owner only) |
| `POST` | `/posts/updatePost/:id` | Yes | Update post and attachments (owner only) |
| `POST` | `/posts/:id/like` | Yes | Like a post |
| `DELETE` | `/posts/:id/like` | Yes | Unlike a post |
| `GET` | `/posts/:id/comments` | No | Get comments on a post |
| `POST` | `/posts/:id/comments` | Yes | Add a comment |
| `DELETE` | `/posts/:id/comments/:comment_id` | Yes | Delete own comment |

**Get all posts query params:** `?provider_id=1` or `?business_id=2`

Creating a post notifies all users who have favorited that provider/business.

---

### Messages

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/messages/sendMessage` | Yes | Send a message |
| `GET` | `/messages/getMessages` | Yes | Get conversation messages |
| `POST` | `/messages/markMessagesAsRead` | Yes | Mark messages as read |
| `GET` | `/messages/unreadMessagesCount` | Yes | Get unread message count |
| `GET` | `/messages/conversations` | Yes | List recent conversations |

New messages are delivered in real-time via WebSocket.

---

### Notifications

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/notifications` | Yes | List notifications (paginated) |
| `GET` | `/notifications/unread-count` | Yes | Get unread notification count |
| `POST` | `/notifications/read-all` | Yes | Mark all notifications as read |
| `POST` | `/notifications/:id/read` | Yes | Mark one notification as read |
| `DELETE` | `/notifications/:id` | Yes | Delete a notification |

**List query params:** `?page=1&per_page=20&unread_only=true`

---

### Favorites

| Method | Path | Auth | Description |
|---|---|---|---|
| `POST` | `/favorites/addFavorite` | Yes | Add a provider/business to favorites |
| `GET` | `/favorites/getFavorites` | Yes | Get own favorites |
| `POST` | `/favorites/removeFavorite/:id` | Yes | Remove a favorite |

**Add favorite body:**
```json
{ "target_type": "provider", "target_id": 1 }
```

**Remove favorite:** `POST /favorites/removeFavorite/1?target_type=provider`

---

### Admin

All admin endpoints require `role = admin`.

| Method | Path | Auth | Description |
|---|---|---|---|
| `GET` | `/admin/categories` | Admin | List all categories |
| `POST` | `/admin/create_category` | Admin | Create a subcategory |
| `POST` | `/admin/create_parent_category` | Admin | Create a parent category |
| `POST` | `/admin/delete_category` | Admin | Delete a category |
| `GET` | `/admin/users` | Admin | List all users |
| `POST` | `/admin/delete_user` | Admin | Delete a user |
| `GET` | `/admin/userAnalytics` | Admin | Platform user growth analytics |
| `POST` | `/admin/flagContent` | Admin | Flag content for review |
| `POST` | `/admin/resolveFlag` | Admin | Resolve a content flag |
| `GET` | `/admin/moderateReviews` | Admin | List flagged reviews |
| `GET` | `/admin/payouts` | Admin | List pending payout requests |
| `POST` | `/admin/payouts/:id/approve` | Admin | Approve a payout |
| `POST` | `/admin/payouts/:id/reject` | Admin | Reject payout (refunds balance) |
| `GET` | `/admin/dashboard` | Admin | Platform-wide stats |

---

### WebSocket

Connect at `GET /ws?token=<jwt>` to receive real-time events.

Events pushed to connected clients:

| Event | Trigger |
|---|---|
| `new_message` | Someone sends you a message |
| `new_notification` | Any in-app notification |
| `booking_created` | New booking received (provider/business) |
| `confirmed` | Your booking was confirmed |
| `cancelled` | Your booking was cancelled |
| `payment_received` | Payment completed for your booking |
| `payment_failed` | Payment failed |
| `new_post` | A favorited provider/business posted |
| `new_review` | You received a new review |
| `review_reply` | Someone replied to your review |

---

## User Roles

| Role | Description |
|---|---|
| `client` | Books services, leaves reviews, chats |
| `provider` | Individual service provider (fundi, tutor, mechanic, etc.) |
| `business` | Business with multiple branches |
| `admin` | Platform administrator |

---

## Error Format

All errors return a consistent JSON body:

```json
{
  "error": "Booking not found"
}
```

| Status | Meaning |
|---|---|
| `400` | Bad request or validation error |
| `401` | Missing or invalid token |
| `403` | Forbidden — insufficient permissions |
| `404` | Resource not found |
| `409` | Conflict (e.g. duplicate booking) |
| `429` | Rate limit exceeded (100 req/min per IP) |
| `500` | Internal server error |
