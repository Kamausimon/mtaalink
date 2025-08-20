MtaaLink API Backend
Overview
MtaaLink is a service marketplace platform connecting service providers with clients. The backend is built with Rust using the Axum web framework, offering a robust, type-safe API with PostgreSQL database integration.

Features
User Management: Authentication, registration, profile management
Service Providers: Create profiles, manage availability, list services
Service Listings: Create, search, and manage service offerings
Bookings: Schedule and manage service appointments
Real-time Messaging: Communication between clients and service providers
Admin Dashboard: User management, category administration
File Uploads: Support for service attachments and user avatars
Technology Stack
Language: Rust
Web Framework: Axum
Database: PostgreSQL with SQLx
Authentication: JWT tokens
Validation: Request validation with the validator crate
File Storage: Local file system with proper organization

API Routes
Authentication
POST /auth/register - Register a new user
POST /auth/login - Login and receive JWT token
POST /auth/verify_email - Verify user email
POST /auth/refresh_token - Get a new access token
User Management
GET /users/me - Get current user profile
PUT /users/update - Update user profile
POST /users/avatar - Upload user avatar
Services
GET /services - List available services with filtering
POST /services - Create a new service
GET /services/:id - Get service details
PUT /services/:id - Update a service
POST /services/:id/attachments - Add attachments to a service
Service Providers
GET /providers - List service providers
GET /providers/:id - Get provider details
POST /providers - Create provider profile
PUT /providers/:id - Update provider profile
POST /providers/availability - Set provider availability
Bookings
GET /bookings - List user bookings
POST /bookings - Create a new booking
PUT /bookings/:id/status - Update booking status
Messages
GET /messages/conversations - List user conversations
GET /messages/:conversation_id - Get conversation messages
POST /messages - Send a new message
PUT /messages/read - Mark messages as read
Admin Routes
GET /admin/categories - List all categories
POST /admin/create_category - Create a new category
POST /admin/create_parent_category - Create parent category with subcategory
POST /admin/delete_category - Delete a category
GET /admin/users - List all users
POST /admin/delete_user - Delete a user
Getting Started
Prerequisites
Rust (latest stable)
PostgreSQL
Cargo

Setup
Clone the repository
Set up environment variables (see .env.example)
Run database migrations:

cargo sqlx migrate run

Start the server
cargo run

