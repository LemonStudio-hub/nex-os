# Deployment Guide

This document covers how to deploy NexOS to production, including Cloudflare Pages (the default), alternative hosting options, and environment configuration.

## Production Build

Before deploying, build the complete application:

```bash
# 1. Compile Rust to WebAssembly
cargo build --target wasm32-unknown-unknown --release

# 2. Generate JavaScript bindings
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm

# 3. Build the frontend
cd web && npm run build && cd ..
```

The production output is in `web/dist/`:

```
web/dist/
├── index.html                         # Entry HTML
└── assets/
    ├── index-<hash>.js                # Bundled JavaScript (~639 KB)
    ├── index-<hash>.css               # Bundled CSS (~4.5 KB)
    ├── nexos-<hash>.js                # WASM JS glue code (~5.4 KB)
    └── nexos_bg-<hash>.wasm           # WASM binary (~245 KB)
```

The total uncompressed size is approximately **900 KB**. With gzip/brotli compression (automatically applied by most hosts), it's significantly smaller.

## Cloudflare Pages (Default)

NexOS is configured for deployment to [Cloudflare Pages](https://pages.cloudflare.com/) using the Wrangler CLI.

### Prerequisites

1. A [Cloudflare account](https://dash.cloudflare.com/sign-up)
2. A Cloudflare Pages project named `nexos` (create via the Cloudflare dashboard or CLI)
3. Authentication via one of:
   - `CLOUDFLARE_API_TOKEN` environment variable (recommended for CI)
   - `npx wrangler login` (interactive, for local development)

### Manual Deployment

```bash
# Build (if not already done)
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm
cd web && npm run build && cd ..

# Deploy
npx wrangler pages deploy web/dist/ --project-name=nexos
```

### Automated Deployment (CI/CD)

The included GitHub Actions workflow (`.github/workflows/ci.yml`) automatically deploys on every push to `main`.

**Required GitHub Secrets:**
| Secret | Description |
|--------|-------------|
| `CLOUDFLARE_API_TOKEN` | Cloudflare API token with Pages permissions |
| `CLOUDFLARE_ACCOUNT_ID` | Your Cloudflare account ID |

**Pipeline flow:**
1. Push to `main` triggers the CI pipeline
2. Rust tests and WASM build run
3. Frontend type-check and build run
4. If all checks pass, deployment to Cloudflare Pages occurs

### Custom Domain

To use a custom domain:

1. Go to Cloudflare Dashboard → Pages → nexos → Custom domains
2. Add your domain
3. Configure DNS (Cloudflare will provide the records)

### Live URL

The production deployment is available at: **https://nexos.pages.dev**

## Alternative Hosting Options

The production build in `web/dist/` is a standard static site (HTML + JS + CSS + WASM). It can be deployed to any static hosting provider.

### Vercel

```bash
# Install Vercel CLI
npm i -g vercel

# Deploy
cd web
vercel dist/
```

Or connect the GitHub repository in the Vercel dashboard with:
- **Build command**: `cd web && npm run build`
- **Output directory**: `web/dist`

### Netlify

**Option A: CLI**
```bash
# Install Netlify CLI
npm i -g netlify-cli

# Deploy
netlify deploy --dir=web/dist --prod
```

**Option B: Drag and drop**
1. Go to [app.netlify.com/drop](https://app.netlify.com/drop)
2. Drag the `web/dist` folder onto the page

**Option C: Git integration**
Connect the repository and configure:
- **Build command**: `cd web && npm run build`
- **Publish directory**: `web/dist`

### GitHub Pages

```bash
# Build
cargo build --target wasm32-unknown-unknown --release
wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm
cd web && npm run build && cd ..

# Copy to gh-pages branch
git subtree split --prefix web/dist -b gh-pages
git push -f origin gh-pages
```

Then enable GitHub Pages in repository settings, pointing to the `gh-pages` branch.

### Self-Hosted (Nginx)

```nginx
server {
    listen 80;
    server_name nexos.example.com;
    root /var/www/nexos;

    # Required for WASM files
    types {
        application/wasm wasm;
    }

    # SPA fallback
    location / {
        try_files $uri $uri/ /index.html;
    }

    # Cache static assets
    location /assets/ {
        expires 1y;
        add_header Cache-Control "public, immutable";
    }
}
```

Copy the build output:
```bash
sudo cp -r web/dist/* /var/www/nexos/
```

### Self-Hosted (Caddy)

```caddyfile
nexos.example.com {
    root * /var/www/nexos
    file_server
    try_files {path} /index.html

    header /assets/* Cache-Control "public, immutable, max-age=31536000"
}
```

### Docker

```dockerfile
# Build stage
FROM rust:slim as builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install wasm-bindgen-cli --version "0.2.x"

WORKDIR /app
COPY . .
RUN cargo build --target wasm32-unknown-unknown --release
RUN wasm-bindgen --target web --out-dir pkg target/wasm32-unknown-unknown/release/nexos.wasm

FROM node:20-slim as frontend
WORKDIR /app
COPY --from=builder /app/pkg ./pkg
COPY web/ ./web/
RUN cd web && npm ci && npm run build

# Serve stage
FROM nginx:alpine
COPY --from=frontend /app/web/dist /usr/share/nginx/html
COPY <<EOF /etc/nginx/conf.d/default.conf
server {
    listen 80;
    root /usr/share/nginx/html;
    types { application/wasm wasm; }
    location / { try_files \$uri /index.html; }
}
EOF
```

Build and run:
```bash
docker build -t nexos .
docker run -p 8080:80 nexos
```

## Environment Configuration

### WASM MIME Type

Ensure your hosting provider serves `.wasm` files with the correct MIME type:

```
Content-Type: application/wasm
```

Most modern hosts (Cloudflare, Vercel, Netlify) handle this automatically. For self-hosted servers, you may need to add the MIME type explicitly.

### HTTPS

NexOS requires a secure context (HTTPS) for:
- **OPFS** (`navigator.storage.getDirectory()`) — only available over HTTPS or localhost
- **Web Crypto API** (`crypto.subtle.digest`) — only available in secure contexts

Localhost is exempted from the HTTPS requirement during development.

### Headers

Recommended security headers:

```
X-Content-Type-Options: nosniff
X-Frame-Options: DENY
Referrer-Policy: strict-origin-when-cross-origin
Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; style-src 'self' 'unsafe-inline'
```

> **Note**: `'wasm-unsafe-eval'` in the CSP is required for WebAssembly execution. Some browsers also accept `'unsafe-eval'` but `'wasm-unsafe-eval'` is more specific.

### Caching Strategy

| Asset | Cache Duration | Reason |
|-------|---------------|--------|
| `index.html` | No cache / short (5 min) | Entry point, must be fresh |
| `assets/*.js` | Long (1 year) | Content-hashed filenames |
| `assets/*.css` | Long (1 year) | Content-hashed filenames |
| `*.wasm` | Long (1 year) | Content-hashed filenames |

Vite automatically adds content hashes to asset filenames, enabling aggressive caching.

## Monitoring

### Error Tracking

For production error tracking, consider adding:

1. **Sentry** or **LogRocket** for JavaScript errors
2. **Cloudflare Web Analytics** for page views (no cookies, privacy-friendly)
3. **Console error monitoring** — the frontend logs errors via `console.error`

### Performance

Key metrics to monitor:
- **WASM load time**: Time to download and initialize the WASM module
- **First input delay**: Time from page load to first user interaction
- **OPFS availability**: Whether persistence is working

## Troubleshooting

### WASM fails to load

- Verify the `.wasm` file is served with `Content-Type: application/wasm`
- Check browser DevTools → Network tab for 404 or CORS errors
- Ensure the `pkg/` directory was built before the frontend build

### OPFS not available

- Ensure the site is served over HTTPS (or localhost)
- Check browser compatibility: Chrome 86+, Edge 86+, Firefox 111+, Safari 15.2+
- The application falls back to memory-only mode gracefully

### Blank page after deploy

- Check browser DevTools → Console for errors
- Verify all assets are being served (no 404s in Network tab)
- Ensure the build completed successfully (`npm run build` exit code 0)

### Version mismatch errors

- `wasm-bindgen` CLI version must match the crate version in `Cargo.toml`
- Reinstall: `cargo install wasm-bindgen-cli --version <version>`
