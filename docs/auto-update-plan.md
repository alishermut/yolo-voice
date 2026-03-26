# Auto-Update Plan for Private GitHub Repository

## Problem

The current updater endpoint (`https://github.com/alishermut/yolo_voice/releases/latest/download/latest.json`) works only when the repo is public. Private repos return 404 for unauthenticated requests, causing updates to fail silently.

---

## Approaches Evaluated

### 1. GitHub Releases (Public Repo) — Current Setup

Tauri fetches `latest.json`, compares versions, downloads the binary, verifies the Minisign signature.

- **Status:** Works only with public repos
- **Limitation:** Private repos return 404 for unauthenticated release asset URLs

### 2. PAT / GitHub App Token in Client Headers

Tauri v2 supports custom headers at runtime:

```rust
app.updater_builder()
    .header("Authorization", "Bearer ghp_xxxx")
    .build()?.check().await?;
```

| Pros | Cons |
|------|------|
| No server needed | Token embedded in binary (credential leak risk) |
| Simple implementation | GitHub download URLs redirect and don't honor Bearer tokens |
| | Token rotation breaks all existing installations |

**Verdict:** Not recommended due to token exposure and GitHub URL redirect architecture.

### 3. Self-Hosted Update Server (Proxy)

A server authenticates with GitHub API server-side and proxies release assets to the client.

```
[Tauri App] → HTTPS → [Your Proxy] → GitHub API (with PAT) → [Private Releases]
```

**Ready-made:** [vonPB/tauri-update-server](https://github.com/vonPB/tauri-update-server) — Rust server with Docker image.

```json
// tauri.conf.json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://updates.yolovoice.app/myapp/stable/{{target}}/{{arch}}/{{current_version}}"
      ]
    }
  }
}
```

| Pros | Cons |
|------|------|
| PAT never leaves the server | Must host and maintain a server (~$5/mo) |
| Full control (channels, rollouts, analytics) | Extra latency from proxy hop |
| Can add device-level auth | |

### 4. GitHub Actions CI/CD Pipeline

Handles the **build and signing** side. Required secrets:

| Secret | Purpose |
|--------|---------|
| `TAURI_SIGNING_PRIVATE_KEY` | Updater signature key |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Key password |
| `AZURE_CLIENT_ID/SECRET/TENANT_ID` | Windows code signing (Azure Trusted Signing) |

`tauri-apps/tauri-action@v0` automates build, release creation, and `latest.json` upload. Does not solve the distribution problem for private repos on its own.

### 5. S3 / Azure Blob / Cloudflare R2

Upload artifacts to object storage in CI, point updater there.

```yaml
- name: Upload to R2
  run: |
    aws s3 cp latest.json s3://yolovoice-updates/latest.json \
      --endpoint-url https://$CF_ACCOUNT_ID.r2.cloudflarestorage.com
    aws s3 cp "$NSIS_ZIP" "s3://yolovoice-updates/$NSIS_ZIP_NAME" \
      --endpoint-url https://$CF_ACCOUNT_ID.r2.cloudflarestorage.com
```

| Pros | Cons |
|------|------|
| CDN-backed, highly available | Extra CI complexity (upload step, URL rewriting) |
| Decoupled from GitHub | Need proxy layer for access control |
| R2 has zero egress fees | |

### 6. GitHub Packages / npm Registry

**Not recommended.** Designed for Node modules and Docker images, not desktop installers. Tauri's updater cannot natively interact with npm/OCI registries. Auth required, 256 MB size limit for npm.

### 7. Cloudflare Workers + R2 (Recommended)

Two variants:

**A) Worker proxies GitHub Releases:** Use [egoist/tauri-updater](https://github.com/egoist/tauri-updater) — clone, set `GITHUB_TOKEN` secret via `wrangler secret put`, deploy.

**B) Worker serves from R2:** Upload artifacts to R2 in CI, serve via a Worker with optional API key auth.

```toml
# wrangler.toml
name = "yolovoice-updates"
main = "src/worker.ts"
compatibility_date = "2024-01-01"

[[r2_buckets]]
binding = "UPDATES_BUCKET"
bucket_name = "yolovoice-updates"
```

```json
// tauri.conf.json
{
  "plugins": {
    "updater": {
      "endpoints": [
        "https://yolovoice-updates.workers.dev/latest.json"
      ],
      "pubkey": "EXISTING_PUBLIC_KEY"
    }
  }
}
```

| Pros | Cons |
|------|------|
| $0/month on free tier (100K req/day, 10 GB, zero egress) | Cloudflare vendor lock-in |
| No server to maintain | Workers free tier has 10ms CPU limit |
| GitHub PAT stays in Workers secrets | |
| Global CDN for fast downloads | |
| Optional auth via X-Api-Key header | |

### 8. CrabNebula Cloud (Commercial)

Built by the Tauri team's company. Fully managed CDN + update server + metrics. Handles private distribution natively. Paid service.

---

## Comparison Table

| Approach | Private Repo | Server | Cost | Setup | Security | Maintenance |
|----------|:---:|:---:|---:|:---:|:---:|:---:|
| GitHub Releases (public) | No | No | Free | Low | Good | None |
| PAT in client | Partial | No | Free | Low | Poor | Low |
| Self-hosted proxy | Yes | VPS | ~$5/mo | Medium | Good | Medium |
| S3/R2 public bucket | Yes | No | ~$1/mo | Medium | Good | Low |
| **Cloudflare Workers + R2** | **Yes** | **No** | **Free** | **Medium** | **Very Good** | **Very Low** |
| CrabNebula Cloud | Yes | No | Paid | Low | Very Good | None |

---

## Recommendation: Cloudflare Workers + R2

### Why

1. **$0/month** on free tier (100K requests/day, 10 GB storage, zero egress fees)
2. **No server to maintain** — Cloudflare manages infrastructure
3. **GitHub PAT stays in Workers secrets** — never embedded in the binary
4. **Minimal changes to existing workflow** — add one R2 upload step to GitHub Actions, deploy a small Worker, update `tauri.conf.json` endpoint
5. **Global CDN** — fast downloads worldwide
6. **Optional auth** via `X-Api-Key` header (Tauri's updater supports custom headers natively)

### Implementation Steps

1. Create a Cloudflare account and an R2 bucket (`yolovoice-updates`)
2. Generate R2 API tokens (S3-compatible) and add as GitHub Actions secrets
3. Add an R2 upload step to `.github/workflows/release.yml` after the build
4. Modify `latest.json` generation to use Worker URLs instead of GitHub URLs
5. Deploy a Cloudflare Worker (~40 lines) to serve from R2 with optional API key validation
6. Update `tauri.conf.json` updater endpoint to the Worker URL
7. Optionally add `X-Api-Key` header in the Rust updater builder for access control

### Runner-Up

If keeping everything on GitHub is preferred, the [vonPB/tauri-update-server](https://github.com/vonPB/tauri-update-server) Rust proxy deploys easily to Fly.io (~$5/mo) and requires no changes to the release artifact flow.

### Security Checklist

- [ ] Updater signing keys stored as GitHub Actions encrypted secrets
- [ ] GitHub PAT (fine-grained, read-only Contents) stored in Workers secrets
- [ ] HTTPS enforced for all update traffic
- [ ] Optional: API key header for update endpoint access control
- [ ] Optional: Windows code signing via Azure Trusted Signing
- [ ] Minisign signature verification enabled (already configured)
