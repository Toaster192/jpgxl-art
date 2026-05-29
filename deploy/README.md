# Deployment

Production runs at <https://jxl-art.toaster.work/> on a single Ubuntu 26.04
VPS, fronted by a Cloudflare Tunnel (no public ports beyond SSH). GitHub
Actions deploys on every push to `main` via SSH.

```
GitHub Actions ─SSH (key)─▶ jxlart@VPS:/opt/jxlart
                                  │
                                  ▼  systemd: jxlart.service
                            artxl binary on :3000
                                  ▲
                                  │  cloudflared (outbound tunnel)
                                  │
            Cloudflare edge ◀── HTTPS ── user (jxl-art.toaster.work)

ufw blocks all inbound except SSH; :3000 is only reachable from loopback,
which is all cloudflared needs.
```

## First-time provision (after a VPS reinstall)

1. **Provision the VPS.** Reinstall as Ubuntu 26.04, add your SSH public key
   during the provider's setup so you can `ssh root@VPS` without a password.

2. **Run the bootstrap.** From your laptop:

   ```bash
   scp deploy/bootstrap.sh root@176.102.64.46:/tmp/
   ssh root@176.102.64.46 bash /tmp/bootstrap.sh
   ```

   This is idempotent and takes ~10 minutes on first run (libjxl build is the
   long part). It creates the `jxlart` user, installs Rust + libjxl build
   deps, clones the repo to `/opt/jxlart`, runs `make setup` + `cargo build
   --release`, installs the systemd unit, and locks the firewall to SSH only.

   To wire the deploy key in the same step (skip if you'd rather do it by
   hand later):

   ```bash
   DEPLOY_KEY="$(cat deploy_key.pub)" ssh root@176.102.64.46 bash /tmp/bootstrap.sh
   ```

3. **Sanity check.** `ssh root@VPS systemctl status jxlart` should show
   `active (running)` within ~5 minutes (the gallery pre-render runs before
   the listener binds). `curl http://localhost:3000/api/generate | head -c 80`
   on the VPS should return an NDJSON line.

4. **Set up the Cloudflare Tunnel.** This part can't be scripted from the
   VPS without an API token; do it in the dashboard:
   - Zero Trust → Networks → Tunnels → **Create a tunnel** → name `jxl-art`.
   - The dashboard shows an install command like `sudo cloudflared service
     install <token>` — copy and run it on the VPS. Installs `cloudflared`
     as a systemd service with the tunnel credentials baked in.
   - In the tunnel's **Public Hostnames** tab: add
     `jxl-art.toaster.work` → Service `http://localhost:3000`.
   - Cloudflare creates the CNAME automatically. Delete the leftover
     `jxl-art.toaster.work` A record (no longer used).

5. **Verify end-to-end.** `curl -I https://jxl-art.toaster.work/` should
   return `200`. If you get a Cloudflare error page, check that
   `cloudflared` is running on the VPS (`systemctl status cloudflared`).

## GitHub Actions secrets

In repo settings → Secrets and variables → Actions:

| Secret            | Value                                              |
|-------------------|----------------------------------------------------|
| `VPS_HOST`        | `176.102.64.46`                                    |
| `VPS_USER`        | `jxlart`                                           |
| `SSH_PRIVATE_KEY` | Private key whose public key is in `~jxlart/.ssh/authorized_keys` |

Generate the key locally and keep the private half out of the repo:

```bash
ssh-keygen -t ed25519 -f deploy_key -C "github-actions@jxlart" -N ""
# put deploy_key.pub into ~jxlart/.ssh/authorized_keys (or pass via DEPLOY_KEY)
# paste deploy_key contents into the SSH_PRIVATE_KEY secret
shred -u deploy_key
```

## What gets deployed

`.github/workflows/deploy.yml` SSHes in as `jxlart` and:

1. `git pull origin main` in `/opt/jxlart`
2. `cargo build --release` (build first — failure leaves the running service untouched)
3. `sudo systemctl restart jxlart` (jxlart user has a narrow sudoers rule for this one command)
4. Polls `http://localhost:3000/api/generate` for up to 6 min — gallery
   pre-render delays the listener.

If the build step fails, the workflow aborts and the running service stays
on the previous binary. If the readiness probe times out, the workflow fails
loud; you'd want to `ssh root@VPS journalctl -u jxlart -n 200` to see why.

## Files

- `bootstrap.sh` — one-shot fresh-box setup. Idempotent.
- `jxlart.service` — systemd unit, installed to `/etc/systemd/system/`. Runs
  the binary as `jxlart`, sandboxed (`ProtectSystem=full`, `PrivateTmp`, etc.)
  but keeps the subprocess access to `./jxl_from_tree` working.

## Operational notes

- **Logs.** `journalctl -u jxlart -f` for live tail; `journalctl -u
  cloudflared -f` for tunnel state.
- **Manual restart.** `ssh jxlart@VPS sudo systemctl restart jxlart`.
- **Tunnel down.** Cloudflare shows a 1033 page; check
  `systemctl status cloudflared`.
- **Hardening you might add later.** Disable root SSH login + password auth
  (`PermitRootLogin no`, `PasswordAuthentication no` in
  `/etc/ssh/sshd_config`) — kept on by default so the bootstrap path stays
  open. If you do this, keep a console-rescue path open in your VPS
  provider's dashboard.
