<p align="center">
  <img src="https://cdn.prod.website-files.com/68e09cef90d613c94c3671c0/697e805a9246c7e090054706_logo_horizontal_grey.png" alt="Yeti" width="200" />
</p>

---

# Yeti Applications

[![Yeti](https://img.shields.io/badge/Yeti-Application-blue)](https://yetirocks.com)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)
[![React](https://img.shields.io/badge/React-18-61dafb)](https://react.dev)
[![TanStack Router](https://img.shields.io/badge/TanStack_Router-type--safe-ff4154)](https://tanstack.com/router)
[![TanStack Table](https://img.shields.io/badge/TanStack_Table-headless-ff4154)](https://tanstack.com/table)

Web-based application manager for Yeti. Browse all applications in a card grid, drill into any app to explore table data with server-side pagination, view configuration, and manage files. Includes Git integration and SSH deploy key management.

## Features

- **Application Grid** — Card-based overview of all apps with status, extension badges, and table counts
- **Data Browser** — Server-side paginated table viewer (25 rows/page) using FIQL queries, with columns from schema field definitions
- **Database Nav** — Sidebar grouping tables by database with record counts
- **Config Viewer** — Readonly JSON display of app configuration
- **Application CRUD** — List, create, update, and delete applications via REST API
- **File Browser** — Browse, create, edit, and delete application files with path traversal protection
- **Git Integration** — Clone repos, pull updates, check status
- **SSH Key Management** — Generate ED25519 deploy keys for private repos
- **Template Support** — Create apps from the application-template or from scratch
- **Hot Reload** — Changes take effect immediately (Yeti auto-detects new/modified apps)

## Installation

```bash
# Clone into your Yeti applications folder
cd ~/yeti/applications
git clone https://github.com/yetirocks/yeti-applications.git

# Restart Yeti to load the application
# The manager will be available at /yeti-applications/
```

## Web UI

Open your browser to:
```
https://localhost:9996/yeti-applications/
```

### Routes

| Route | View |
|---|---|
| `#/` | Card grid home — all apps sorted by extension status then alphabetically |
| `#/app/{appId}` | Auto-redirects to first data route (or config if no tables) |
| `#/app/{appId}/data/{database}/{table}` | Data view — paginated table browser |
| `#/app/{appId}/config` | Config view — readonly JSON textarea |

### Card Grid (Home)

Displays all applications as cards with status indicator (Running/Disabled), extension badge, and table count. Extensions sort first, then alphabetically.

### Detail View

Clicking an app opens the detail view with:

- **Header bar** — Back button, app name, status dot, extension badge, and Data/Config subnav tabs
- **Database nav sidebar** — Left panel listing tables grouped by database name, each showing a right-aligned record count
- **Data tab** — Server-side paginated table display (25 rows/page) with column headers derived from schema field definitions. Pagination uses FIQL queries (`?pagination=true&limit=25&offset=N`) against the REST endpoint
- **Config tab** — Full-height readonly textarea showing the app's config.yaml as formatted JSON

```
┌──────────────────────────────────────────────────────────┐
│ [Logo]                            Application Manager    │
├──────────────────────────────────────────────────────────┤
│ [← Back]  app-name  ● Running  ext      [Data] [Config] │
├─────────┬────────────────────────────────────────────────┤
│ DB Nav  │  Content                                       │
│         │                                                │
│ yeti-db │  ┌──────┬───────┬────────┬─────────┐          │
│  Role 3 │  │ col  │ col   │ col    │ col     │          │
│  User 4 │  ├──────┼───────┼────────┼─────────┤          │
│         │  │ ...  │ ...   │ ...    │ ...     │          │
│         │  └──────┴───────┴────────┴─────────┘          │
│         │  [← Prev]  Page 1 of 3  [Next →]  25 records  │
└─────────┴────────────────────────────────────────────────┘
```

### Tech Stack

- **[TanStack Router](https://tanstack.com/router)** — File-based, type-safe routing with hash history
- **[TanStack Table](https://tanstack.com/table)** — Headless table with server-side pagination
- **React 18** + **TypeScript** + **Vite**

## API Endpoints

### Applications

```bash
# List all applications
curl -sk https://localhost:9996/yeti-applications/apps
# Response: [{"id": "my-app", "name": "My App", "enabled": true, ...}, ...]

# Get application details
curl -sk https://localhost:9996/yeti-applications/apps/my-app
# Response: {"id": "my-app", "config": {...}, "schema": {...}, ...}

# Create from template
curl -sk -X POST https://localhost:9996/yeti-applications/apps \
  -H "Content-Type: application/json" \
  -d '{"id": "new-app", "name": "New App", "template": true}'

# Create blank
curl -sk -X POST https://localhost:9996/yeti-applications/apps \
  -H "Content-Type: application/json" \
  -d '{"id": "new-app", "name": "New App", "template": false}'

# Update config (YAML merge — only specified keys are changed)
curl -sk -X PUT https://localhost:9996/yeti-applications/apps/my-app \
  -H "Content-Type: application/json" \
  -d '{"config": {"enabled": false}}'

# Delete application (removes directory and clears plugin cache)
curl -sk -X DELETE https://localhost:9996/yeti-applications/apps/my-app
```

### File Browser

```bash
# List directory contents
curl -sk "https://localhost:9996/yeti-applications/files?app=my-app&path=/"
# Response: [{"name": "config.yaml", "type": "file", "size": 234}, ...]

# Read a file
curl -sk "https://localhost:9996/yeti-applications/files?app=my-app&path=/config.yaml"
# Response: {"content": "name: My App\n..."}

# Create/update a file
curl -sk -X POST https://localhost:9996/yeti-applications/files \
  -H "Content-Type: application/json" \
  -d '{"app": "my-app", "path": "/resources/hello.rs", "content": "use yeti_core::prelude::*;\n..."}'

# Delete a file
curl -sk -X DELETE "https://localhost:9996/yeti-applications/files?app=my-app&path=/resources/old.rs"
```

### Schemas

```bash
# Get schema info for an app (tables, fields, database, REST URLs)
curl -sk https://localhost:9996/yeti-applications/schemas/my-app
# Response: {"app_id": "my-app", "tables": [
#   {"name": "User", "database": "my-db", "rest_url": "/my-app/User",
#    "fields": [{"name": "username", "type": "String"}, ...]}, ...]}
```

### Git Operations

```bash
# Check if a repo URL is accessible
curl -sk -X POST https://localhost:9996/yeti-applications/repos \
  -H "Content-Type: application/json" \
  -d '{"action": "check", "url": "https://github.com/yetirocks/my-app.git"}'

# Clone a repository
curl -sk -X POST https://localhost:9996/yeti-applications/repos \
  -H "Content-Type: application/json" \
  -d '{"action": "clone", "url": "https://github.com/yetirocks/my-app.git", "id": "my-app"}'

# Clone with SSH key
curl -sk -X POST https://localhost:9996/yeti-applications/repos \
  -H "Content-Type: application/json" \
  -d '{"action": "clone", "url": "git@github.com:org/repo.git", "id": "my-app", "keyId": "key-1"}'

# Pull latest changes
curl -sk -X POST https://localhost:9996/yeti-applications/repos \
  -H "Content-Type: application/json" \
  -d '{"action": "pull", "id": "my-app"}'

# Check git status
curl -sk -X POST https://localhost:9996/yeti-applications/repos \
  -H "Content-Type: application/json" \
  -d '{"action": "status", "id": "my-app"}'
```

### SSH Deploy Keys

```bash
# List all deploy keys
curl -sk https://localhost:9996/yeti-applications/keys

# Generate a new ED25519 key pair
curl -sk -X POST https://localhost:9996/yeti-applications/keys \
  -H "Content-Type: application/json" \
  -d '{"name": "github-deploy"}'
# Response: {"id": "key-1", "publicKey": "ssh-ed25519 AAAA...", "name": "github-deploy"}

# Delete a key
curl -sk -X DELETE https://localhost:9996/yeti-applications/keys/key-1
```

## Access Control

yeti-applications uses yeti-auth for access control. Configure OAuth rules in config.yaml:

```yaml
extensions:
  - yeti-auth:
      oauth:
        rules:
          - strategy: provider
            pattern: "google"
            role: admin
```

## Project Structure

```
yeti-applications/
├── config.yaml              # App config with yeti-auth extension
├── schema.graphql           # AppValidation table schema
├── resources/
│   ├── apps.rs              # Application CRUD (list, get, create, update, delete)
│   ├── files.rs             # File browser/editor with path traversal protection
│   ├── schemas.rs           # Schema parser (extracts @table directives)
│   ├── repos.rs             # Git operations (check, clone, pull, status)
│   └── keys.rs              # SSH deploy key management (ED25519)
├── source/                  # React/Vite/TanStack source
│   ├── vite.config.ts       # Vite config with TanStack Router plugin
│   └── src/
│       ├── main.tsx         # Entry point — RouterProvider
│       ├── router.tsx       # createRouter with hash history
│       ├── routeTree.gen.ts # Auto-generated route tree
│       ├── api.ts           # Fetch wrapper
│       ├── types.ts         # TypeScript interfaces
│       ├── index.css        # All styles
│       ├── hooks/
│       │   └── useToast.tsx # Toast notifications
│       ├── components/
│       │   ├── AppCard.tsx      # App card for home grid
│       │   ├── DatabaseNav.tsx  # Left sidebar db/table tree with counts
│       │   └── DataTable.tsx    # TanStack Table with server pagination
│       └── routes/
│           ├── __root.tsx               # Root layout (header + outlet)
│           ├── index.tsx                # Home — card grid
│           └── app/$appId/
│               ├── route.tsx            # App layout (header, subnav, sidebar)
│               ├── index.tsx            # Redirect to first data or config
│               ├── config.tsx           # Config textarea
│               └── data/
│                   └── $database.$table.tsx  # Paginated data view
└── web/                     # Built static SPA
    └── index.html
```

## Learn More

- [Yeti Documentation](https://yetirocks.com/docs)
- [Application Configuration](https://yetirocks.com/docs/reference/app-config)
- [Schema Directives](https://yetirocks.com/docs/reference/schema-directives)

---

Built with [Yeti](https://yetirocks.com) - The fast, declarative database platform.
