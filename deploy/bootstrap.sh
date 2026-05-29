#!/usr/bin/env bash
# Bootstrap a fresh Ubuntu 26.04 VPS for ArtXL.
#
# Run as root on the freshly-installed box:
#
#   scp deploy/bootstrap.sh root@VPS:/tmp/
#   ssh root@VPS bash /tmp/bootstrap.sh
#
# Optionally pass the GitHub Actions deploy public key inline so the script
# wires it into ~jxlart/.ssh/authorized_keys for you:
#
#   DEPLOY_KEY="$(cat deploy_key.pub)" ssh root@VPS bash /tmp/bootstrap.sh
#
# The script is idempotent — running it again after a partial failure is safe.

set -euo pipefail

APP_USER=jxlart
APP_DIR=/opt/jxlart
REPO_URL="${REPO_URL:-https://github.com/Toaster192/jpgxl-art.git}"
DEPLOY_KEY="${DEPLOY_KEY:-}"

if [[ $EUID -ne 0 ]]; then
  echo "bootstrap.sh: must run as root" >&2
  exit 1
fi

echo "==> apt update + base packages"
export DEBIAN_FRONTEND=noninteractive
apt-get update
apt-get install -y --no-install-recommends \
  build-essential cmake git curl ca-certificates pkg-config \
  libhwy-dev libbrotli-dev liblcms2-dev patchelf \
  ufw sudo

echo "==> create $APP_USER user"
if ! id "$APP_USER" &>/dev/null; then
  useradd --create-home --shell /bin/bash --comment "ArtXL service" "$APP_USER"
fi

echo "==> install rust toolchain (as $APP_USER)"
sudo -u "$APP_USER" bash -lc '
  set -euo pipefail
  if [[ ! -x "$HOME/.cargo/bin/cargo" ]]; then
    curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs |
      sh -s -- --default-toolchain stable --profile minimal -y
  fi
'

echo "==> clone / update repo at $APP_DIR"
mkdir -p "$APP_DIR"
chown "$APP_USER:$APP_USER" "$APP_DIR"
if [[ ! -d "$APP_DIR/.git" ]]; then
  sudo -u "$APP_USER" git clone "$REPO_URL" "$APP_DIR"
else
  sudo -u "$APP_USER" git -C "$APP_DIR" pull --ff-only
fi

echo "==> build jxl_from_tree (make setup) and release binary"
# make setup clones libjxl v0.11.2 and builds it; expect ~5-10min on first run.
sudo -u "$APP_USER" bash -lc "
  set -euo pipefail
  cd '$APP_DIR'
  make setup
  ~/.cargo/bin/cargo build --release
"

echo "==> install systemd unit"
install -m 644 "$APP_DIR/deploy/jxlart.service" /etc/systemd/system/jxlart.service
systemctl daemon-reload
systemctl enable --now jxlart

echo "==> sudoers rule for deploy (jxlart can restart its own service)"
cat > /etc/sudoers.d/jxlart << 'EOF'
# Lets the jxlart user restart its own service from the GitHub deploy hook
# without a password prompt. Visudo-validated; no other sudo rights.
jxlart ALL=(root) NOPASSWD: /usr/bin/systemctl restart jxlart, /usr/bin/systemctl status jxlart, /usr/bin/systemctl is-active jxlart
EOF
chmod 440 /etc/sudoers.d/jxlart
visudo -c -f /etc/sudoers.d/jxlart

if [[ -n "$DEPLOY_KEY" ]]; then
  echo "==> wire deploy key into authorized_keys"
  sudo -u "$APP_USER" mkdir -p "/home/$APP_USER/.ssh"
  sudo -u "$APP_USER" chmod 700 "/home/$APP_USER/.ssh"
  AUTH="/home/$APP_USER/.ssh/authorized_keys"
  if ! sudo -u "$APP_USER" grep -qxF "$DEPLOY_KEY" "$AUTH" 2>/dev/null; then
    echo "$DEPLOY_KEY" | sudo -u "$APP_USER" tee -a "$AUTH" >/dev/null
  fi
  sudo -u "$APP_USER" chmod 600 "$AUTH"
fi

echo "==> firewall: SSH only"
# Cloudflare Tunnel doesn't need inbound ports; the only thing we expose is
# SSH for deploys. The artxl listener binds 0.0.0.0:3000 but ufw drops
# anything that isn't loopback or already-established.
ufw allow 22/tcp comment "SSH"
ufw default deny incoming
ufw default allow outgoing
ufw --force enable

echo
echo "==> bootstrap done."
echo
echo "Local sanity check (wait ~5min for the gallery pre-render):"
echo "    systemctl status jxlart"
echo "    curl -fsS http://localhost:3000/api/generate | head -c 120"
echo
echo "Next, from the Cloudflare dashboard:"
echo "  1. Zero Trust → Networks → Tunnels → Create a tunnel → name it jxl-art"
echo "  2. Copy the install command it shows you and run it on this VPS"
echo "  3. In the tunnel's 'Public Hostnames' tab: add jxl-art.toaster.work"
echo "     → Service: http://localhost:3000"
echo "  4. Cloudflare will create the CNAME automatically; remove the old A"
echo "     record for jxl-art.toaster.work if it's still there."
echo
echo "Then in GitHub repo settings → Secrets and variables → Actions:"
echo "  VPS_HOST         = 176.102.64.46"
echo "  VPS_USER         = jxlart"
echo "  SSH_PRIVATE_KEY  = (the private key matching the deploy public key)"
