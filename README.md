# pavkhemerak-api

Rust/Axum backend API powering [pavkhemerak.dev](https://pavkhemerak.dev). Serves the blog content, GitHub activity feed, and security tooling endpoints.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Framework | [Axum](https://github.com/tokio-rs/axum) 0.8 |
| Runtime | [Tokio](https://tokio.rs) |
| Database | SQLite via [sqlx](https://github.com/launchbadge/sqlx) |
| HTTP Client | [reqwest](https://github.com/seanmonstar/reqwest) |
| Serialization | [serde](https://serde.rs) + serde_json |

## API Endpoints

### Health
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/health` | Server status, version, uptime |

### Blog (Public)
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/blog/posts` | List posts (paginated, filterable) |
| `GET` | `/api/blog/posts/{slug}` | Get full post by slug |
| `GET` | `/api/blog/categories` | List all categories |

**Query Parameters for `/api/blog/posts`:**
- `page` — Page number (default: 1)
- `per_page` — Items per page (default: 10, max: 50)
- `category` — Filter by category name (e.g. `CYBERSECURITY`)

### Blog (Admin — requires `x-api-key` header)
| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/blog/posts` | Create a new post |
| `PUT` | `/api/blog/posts/{slug}` | Update an existing post |
| `DELETE` | `/api/blog/posts/{slug}` | Delete a post |

### GitHub
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/github/activity` | Recent GitHub events |

### Tools
| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/tools/ping?host=` | Ping a host |
| `GET` | `/api/tools/etherscan?address=` | Analyze Ethereum address for bot patterns |

## Getting Started

### Prerequisites
- [Rust](https://rustup.rs/) (1.75+ recommended)
- SQLite (bundled via `libsqlite3-sys`)

### Setup

```bash
# Clone and enter the backend directory
cd backend

# Copy and configure environment variables
cp .env.example .env
# Edit .env with your values

# Build and run
cargo run
```

The server starts on `http://localhost:3001` by default. The SQLite database is auto-created at `data/pavkhemerak.db` on first run, and seeded with 5 sample blog posts.

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `PORT` | `3001` | Server port |
| `DATABASE_URL` | `sqlite:data/pavkhemerak.db` | SQLite connection string |
| `GITHUB_USERNAME` | `khemerak` | GitHub username for activity feed |
| `ETHERSCAN_API_KEY` | *(empty)* | Etherscan API key (optional) |
| `ADMIN_API_KEY` | `change-me-in-production` | API key for blog admin endpoints |

### Example: Creating a Blog Post

```bash
curl -X POST http://localhost:3001/api/blog/posts \
  -H "Content-Type: application/json" \
  -H "x-api-key: change-me-in-production" \
  -d '{
    "slug": "my-first-post",
    "title": "My First Post",
    "excerpt": "A brief introduction.",
    "content": "## Hello World\n\nThis is my first blog post.",
    "date": "2024-11-01",
    "readTime": "3 min read",
    "category": "GENERAL",
    "categoryColor": "border-outline-variant text-on-surface-variant",
    "tags": ["intro", "blog"]
  }'
```

## Project Structure

```
backend/
├── Cargo.toml              # Dependencies
├── .env                    # Environment config (git-ignored)
├── src/
│   ├── main.rs             # Entry point: server, CORS, routing
│   ├── config.rs           # Environment variable loader
│   ├── db.rs               # SQLite pool init + migrations
│   ├── errors.rs           # Unified API error type
│   ├── seed.rs             # Auto-seed initial blog posts
│   ├── models/
│   │   ├── mod.rs
│   │   └── blog.rs         # BlogPost structs (DB row, API DTOs)
│   └── routes/
│       ├── mod.rs           # Route registration
│       ├── health.rs        # GET /api/health
│       ├── blog.rs          # Blog CRUD
│       ├── github.rs        # GitHub activity proxy
│       └── tools.rs         # Ping + Etherscan analyzer
└── data/                   # SQLite database (git-ignored)
```

## Deployment

The backend is designed to run as a standalone binary. For production:

1. Build a release binary: `cargo build --release`
2. Set `ADMIN_API_KEY` to a strong random value
3. Configure `ETHERSCAN_API_KEY` if using the analyzer
4. Run behind a reverse proxy (Nginx) with HTTPS
5. Point the frontend's `BACKEND_URL` to the API domain

### Docker (future)

A `Dockerfile` and `docker-compose.yml` are planned for Phase 5 (CI/CD & Automation) of the project roadmap.
