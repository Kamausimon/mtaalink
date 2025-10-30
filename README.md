md
# MtaaLink API Backend

## Overview

MtaaLink is a service marketplace platform connecting service providers with clients. The backend is built with Rust using the Axum web framework, offering a robust, type-safe API with PostgreSQL database integration.

## Key Features & Benefits

*   **User Management:** Authentication, registration, profile management.
*   **Service Providers:** Create profiles, manage availability, list services.
*   **Service Listings:** Create, search, and manage service offerings.
*   **Bookings:** Schedule and manage service appointments.
*   **Robust & Type-Safe:** Built with Rust and Axum for reliability and maintainability.
*   **PostgreSQL Integration:** Utilizes PostgreSQL for data persistence.
*   **Well-Structured Codebase:** Organized into modules for clarity and scalability.

## Prerequisites & Dependencies

Before you begin, ensure you have the following installed:

*   **Rust:** Rust toolchain (including Cargo) - Install from [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)
*   **PostgreSQL:** PostgreSQL database server - Install from [https://www.postgresql.org/download/](https://www.postgresql.org/download/)
*   **Docker (Optional):** For containerization and simplified setup.
*   **Cargo:** Rust's package manager (comes with Rust installation).

## Installation & Setup Instructions

1.  **Clone the Repository:**

    ```bash
    git clone https://github.com/Kamausimon/mtaalink.git
    cd mtaalink
    ```

2.  **Install Dependencies:**

    ```bash
    cargo build
    ```

3.  **Set up the PostgreSQL Database:**

    *   Create a new database named `mtaalink` (or customize in `.env`).
    *   Create the necessary tables by running the database migrations. Instructions on setting up migrations will be added soon.

4.  **Configure Environment Variables:**

    *   Create a `.env` file in the root directory based on the example below:

        ```
        DATABASE_URL="postgres://user:password@host:port/mtaalink"
        JWT_SECRET="your_jwt_secret"
        # Add any other configuration variables here
        ```

    *   Replace placeholders with your actual database credentials and JWT secret.  **Important:** Use a strong and secure secret key for JWT!

5.  **Run the Application:**

    ```bash
    cargo run
    ```

    This will start the MtaaLink API backend.

## Usage Examples & API Documentation

API documentation is currently under development. Once available, it will provide detailed information on endpoints, request/response formats, and authentication. Stay tuned for updates!

For now, refer to the `src/routes` directory to understand the available API endpoints:

*   `/routes/auth.rs`: Authentication routes (login, registration).
*   `/routes/businesses.rs`: Routes for managing businesses.
*   `/routes/bookings.rs`: Routes for managing bookings.

## Configuration Options

The application's behavior can be configured using environment variables defined in the `.env` file.

| Variable       | Description                                                               | Default Value |
| -------------- | ------------------------------------------------------------------------- | ------------- |
| `DATABASE_URL` | The connection string for the PostgreSQL database.                         |               |
| `JWT_SECRET`   | The secret key used for signing JSON Web Tokens (JWT).                    |               |
| `PORT`         | The port on which the application will listen.                             | `3000`          |
| `LOG_LEVEL`    | The log level for the application (e.g., `debug`, `info`, `warn`, `error`). | `info`          |

## Contributing Guidelines

We welcome contributions to the MtaaLink project! To contribute:

1.  Fork the repository.
2.  Create a new branch for your feature or bug fix.
3.  Implement your changes, ensuring they are well-tested and documented.
4.  Submit a pull request with a clear description of your changes.

Please follow these guidelines:

*   Write clean, well-documented code.
*   Follow the existing code style and conventions.
*   Include unit tests for new features and bug fixes.
*   Ensure your changes do not break existing functionality.

## License Information

License information is not yet specified. This will be updated soon.

## Acknowledgments

The MtaaLink project utilizes the following open-source libraries and frameworks:

*   **Rust:** The programming language.
*   **Axum:** The web framework.
*   **Tokio:** The asynchronous runtime.
*   **PostgreSQL:** The database system.

We would like to express our gratitude to the developers and maintainers of these excellent tools.
