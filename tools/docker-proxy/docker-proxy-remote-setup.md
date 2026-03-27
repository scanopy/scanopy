# Docker API Remote Setup Guide

How to expose Docker's API on remote hosts for use with Scanopy's DockerProxy credential.

> **Warning:** Exposing the Docker API grants full root-equivalent access to the host. Only do this on trusted networks for dev/testing. Always use TLS with client certificates in any shared environment.

## Portainer Host

Portainer's agent protocol is separate from the raw Docker API. To expose the Docker API itself:

### Option 1: TCP socket via Docker daemon config

On the Portainer host:

```bash
# Edit Docker daemon config
sudo nano /etc/docker/daemon.json
```

```json

```

```bash
# If systemd manages Docker, override the -H flag conflict
sudo mkdir -p /etc/systemd/system/docker.service.d
cat <<EOF | sudo tee /etc/systemd/system/docker.service.d/override.conf
[Service]
ExecStart=
ExecStart=/usr/bin/dockerd
EOF

sudo systemctl daemon-reload
sudo systemctl restart docker
```

### Option 2: nginx reverse proxy (with TLS)

This is the recommended approach — it doesn't require reconfiguring the Docker daemon.

```bash
sudo apt install nginx

cat <<'EOF' | sudo tee /etc/nginx/sites-available/docker-api
server {
    listen 2376 ssl;

    ssl_certificate /etc/docker/certs/server-cert.pem;
    ssl_certificate_key /etc/docker/certs/server-key.pem;
    ssl_client_certificate /etc/docker/certs/ca.pem;
    ssl_verify_client on;

    location / {
        proxy_pass http://unix:/var/run/docker.sock:/;
        proxy_set_header Host $host;
        proxy_read_timeout 300;
        proxy_buffering off;
    }
}
EOF

sudo ln -s /etc/nginx/sites-available/docker-api /etc/nginx/sites-enabled/
sudo nginx -t && sudo systemctl reload nginx
```

See the "Generate TLS Certificates" section below for cert generation.

## Proxmox VM/LXC

Docker doesn't run on the Proxmox host itself — it runs inside VMs or LXC containers.

### VM with Docker

Standard Linux setup (see "Generic Linux" below). The VM's IP is directly accessible from your network.

### LXC Container with Docker

1. Ensure the LXC container is **privileged** or has the `nesting` and `keyctl` features enabled:

```bash
# In Proxmox host shell
pct set <CTID> --features nesting=1,keyctl=1
pct restart <CTID>
```

2. Install Docker inside the LXC container:

```bash
# Inside the container
curl -fsSL https://get.docker.com | sh
```

3. Expose the API using one of the methods from "Generic Linux" below.

The container's IP (visible in Proxmox UI or via `pct exec <CTID> -- hostname -I`) is what you'll use in Scanopy's DockerProxy host field.

## Generic Linux

### Without TLS (dev only, trusted network)

```bash
# Quick: socat proxy (foreground, Ctrl+C to stop)
socat TCP-LISTEN:2376,reuseaddr,fork UNIX-CONNECT:/var/run/docker.sock

# Persistent: Docker daemon config
sudo nano /etc/docker/daemon.json
```

```json
{
  "hosts": ["unix:///var/run/docker.sock", "tcp://0.0.0.0:2376"]
}
```

```bash
sudo systemctl daemon-reload && sudo systemctl restart docker

# Verify
curl http://<HOST_IP>:2376/version
```

### With TLS (recommended)

#### Generate TLS Certificates

```bash
CERT_DIR=/etc/docker/certs
sudo mkdir -p "$CERT_DIR"

# CA
sudo openssl genrsa -out "$CERT_DIR/ca-key.pem" 4096
sudo openssl req -new -x509 -days 365 -key "$CERT_DIR/ca-key.pem" \
    -sha256 -out "$CERT_DIR/ca.pem" -subj "/CN=Docker CA"

# Server cert (replace HOST_IP with actual IP)
HOST_IP=192.168.4.126
sudo openssl genrsa -out "$CERT_DIR/server-key.pem" 4096
sudo openssl req -new -key "$CERT_DIR/server-key.pem" \
    -subj "/CN=$HOST_IP" -out "$CERT_DIR/server.csr"

echo "subjectAltName = IP:$HOST_IP,IP:127.0.0.1" > /tmp/ext.cnf
echo "extendedKeyUsage = serverAuth" >> /tmp/ext.cnf

sudo openssl x509 -req -days 365 -sha256 \
    -in "$CERT_DIR/server.csr" \
    -CA "$CERT_DIR/ca.pem" -CAkey "$CERT_DIR/ca-key.pem" -CAcreateserial \
    -out "$CERT_DIR/server-cert.pem" -extfile /tmp/ext.cnf

# Client cert
sudo openssl genrsa -out "$CERT_DIR/client-key.pem" 4096
sudo openssl req -new -key "$CERT_DIR/client-key.pem" \
    -subj "/CN=Docker Client" -out "$CERT_DIR/client.csr"

echo "extendedKeyUsage = clientAuth" > /tmp/client-ext.cnf

sudo openssl x509 -req -days 365 -sha256 \
    -in "$CERT_DIR/client.csr" \
    -CA "$CERT_DIR/ca.pem" -CAkey "$CERT_DIR/ca-key.pem" -CAcreateserial \
    -out "$CERT_DIR/client-cert.pem" -extfile /tmp/client-ext.cnf

# Clean up
sudo rm -f "$CERT_DIR"/*.csr /tmp/ext.cnf /tmp/client-ext.cnf
```

#### Configure Docker daemon with TLS

```bash
sudo nano /etc/docker/daemon.json
```

```json
{
  "hosts": ["unix:///var/run/docker.sock", "tcp://0.0.0.0:2376"],
  "tls": true,
  "tlsverify": true,
  "tlscacert": "/etc/docker/certs/ca.pem",
  "tlscert": "/etc/docker/certs/server-cert.pem",
  "tlskey": "/etc/docker/certs/server-key.pem"
}
```

```bash
sudo systemctl daemon-reload && sudo systemctl restart docker

# Verify
curl --cacert /etc/docker/certs/ca.pem \
     --cert /etc/docker/certs/client-cert.pem \
     --key /etc/docker/certs/client-key.pem \
     https://<HOST_IP>:2376/version
```

#### Using in Scanopy

In the DockerProxy credential form, enter:
- **Host:** `<HOST_IP>`
- **Port:** `2376`
- **CA Certificate:** contents of `ca.pem`
- **Client Certificate:** contents of `client-cert.pem`
- **Client Key:** contents of `client-key.pem`

## Security Notes

- The Docker API provides **root-equivalent access** to the host. Anyone who can reach the API can run arbitrary containers, mount host filesystems, and escalate to root.
- **Never** expose the Docker API on `0.0.0.0` without TLS and client certificate verification.
- Use firewall rules to restrict access to known IPs even with TLS enabled.
- For production, consider Docker's built-in TLS (`dockerd --tlsverify`) or a proper reverse proxy with authentication.
- Rotate certificates regularly — the examples above use 365-day validity for convenience.
