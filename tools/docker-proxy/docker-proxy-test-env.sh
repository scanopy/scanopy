#!/bin/bash
set -euo pipefail

# Docker Proxy Test Environment — exposes local Docker socket on TCP via nginx
# Usage: tools/docker-proxy-test-env.sh up [--tls] [--port PORT]
#        tools/docker-proxy-test-env.sh down [--clean]
#        tools/docker-proxy-test-env.sh status

WORK_DIR=/tmp/docker-proxy-test-env
PID_FILE="$WORK_DIR/nginx.pid"
CONF_FILE="$WORK_DIR/nginx.conf"
CERT_DIR="$WORK_DIR/certs"
STATE_FILE="$WORK_DIR/state"
DOCKER_SOCK=/var/run/docker.sock
DEFAULT_PORT=2376

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

check_deps() {
    local missing=()

    if ! command -v nginx &>/dev/null; then
        missing+=("nginx")
    fi

    if ! command -v openssl &>/dev/null; then
        missing+=("openssl")
    fi

    if [ ${#missing[@]} -gt 0 ]; then
        printf "${RED}Missing dependencies:${NC} %s\n" "${missing[*]}"
        echo ""
        for dep in "${missing[@]}"; do
            case "$dep" in
                nginx)   echo "  brew install nginx    # macOS" ;;
                openssl) echo "  brew install openssl  # macOS" ;;
            esac
        done
        exit 1
    fi

    if [ ! -S "$DOCKER_SOCK" ]; then
        printf "${RED}Docker socket not found at %s${NC}\n" "$DOCKER_SOCK"
        echo "Is Docker running?"
        exit 1
    fi
}

generate_certs() {
    mkdir -p "$CERT_DIR"

    echo "Generating TLS certificates..."

    # CA
    openssl genrsa -out "$CERT_DIR/ca-key.pem" 4096 2>/dev/null
    openssl req -new -x509 -days 365 -key "$CERT_DIR/ca-key.pem" \
        -sha256 -out "$CERT_DIR/ca.pem" \
        -subj "/CN=Docker Proxy Test CA" 2>/dev/null

    # Server cert
    openssl genrsa -out "$CERT_DIR/server-key.pem" 4096 2>/dev/null
    openssl req -new -key "$CERT_DIR/server-key.pem" \
        -subj "/CN=localhost" -out "$CERT_DIR/server.csr" 2>/dev/null

    cat > "$CERT_DIR/server-ext.cnf" <<EOF
subjectAltName = DNS:localhost,IP:127.0.0.1
extendedKeyUsage = serverAuth
EOF

    openssl x509 -req -days 365 -sha256 \
        -in "$CERT_DIR/server.csr" \
        -CA "$CERT_DIR/ca.pem" -CAkey "$CERT_DIR/ca-key.pem" -CAcreateserial \
        -out "$CERT_DIR/server-cert.pem" \
        -extfile "$CERT_DIR/server-ext.cnf" 2>/dev/null

    # Client cert
    openssl genrsa -out "$CERT_DIR/client-key.pem" 4096 2>/dev/null
    openssl req -new -key "$CERT_DIR/client-key.pem" \
        -subj "/CN=Docker Proxy Test Client" -out "$CERT_DIR/client.csr" 2>/dev/null

    cat > "$CERT_DIR/client-ext.cnf" <<EOF
extendedKeyUsage = clientAuth
EOF

    openssl x509 -req -days 365 -sha256 \
        -in "$CERT_DIR/client.csr" \
        -CA "$CERT_DIR/ca.pem" -CAkey "$CERT_DIR/ca-key.pem" -CAcreateserial \
        -out "$CERT_DIR/client-cert.pem" \
        -extfile "$CERT_DIR/client-ext.cnf" 2>/dev/null

    # Clean up CSRs and temp files
    rm -f "$CERT_DIR"/*.csr "$CERT_DIR"/*.cnf "$CERT_DIR"/*.srl

    printf "  ${GREEN}✓${NC} CA cert:      %s\n" "$CERT_DIR/ca.pem"
    printf "  ${GREEN}✓${NC} Server cert:  %s\n" "$CERT_DIR/server-cert.pem"
    printf "  ${GREEN}✓${NC} Server key:   %s\n" "$CERT_DIR/server-key.pem"
    printf "  ${GREEN}✓${NC} Client cert:  %s\n" "$CERT_DIR/client-cert.pem"
    printf "  ${GREEN}✓${NC} Client key:   %s\n" "$CERT_DIR/client-key.pem"
}

generate_nginx_conf() {
    local port="$1"
    local tls="$2"

    cat > "$CONF_FILE" <<EOF
worker_processes 1;
pid $PID_FILE;
error_log $WORK_DIR/error.log warn;

events {
    worker_connections 64;
}

http {
    access_log $WORK_DIR/access.log;

    upstream docker {
        server unix:$DOCKER_SOCK;
    }

    server {
        listen 127.0.0.1:${port}$([ "$tls" = "true" ] && echo " ssl");
EOF

    if [ "$tls" = "true" ]; then
        cat >> "$CONF_FILE" <<EOF

        ssl_certificate $CERT_DIR/server-cert.pem;
        ssl_certificate_key $CERT_DIR/server-key.pem;
        ssl_client_certificate $CERT_DIR/ca.pem;
        ssl_verify_client on;

        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;
EOF
    fi

    cat >> "$CONF_FILE" <<EOF

        location / {
            proxy_pass http://docker;
            proxy_set_header Host \$host;
            proxy_set_header X-Real-IP \$remote_addr;
            proxy_read_timeout 300;
            proxy_connect_timeout 10;

            # Required for streaming endpoints (logs, events, attach)
            proxy_buffering off;
            chunked_transfer_encoding on;
        }
    }
}
EOF
}

print_pem_contents() {
    echo ""
    printf "${BOLD}Certificate PEM contents (copy-paste into Scanopy):${NC}\n"

    echo ""
    printf "${CYAN}── CA Certificate ──${NC}\n"
    cat "$CERT_DIR/ca.pem"

    echo ""
    printf "${CYAN}── Client Certificate ──${NC}\n"
    cat "$CERT_DIR/client-cert.pem"

    echo ""
    printf "${CYAN}── Client Key ──${NC}\n"
    cat "$CERT_DIR/client-key.pem"
}

cmd_up() {
    local port="$DEFAULT_PORT"
    local tls=false

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --tls) tls=true; shift ;;
            --port) port="$2"; shift 2 ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    check_deps

    # Check if already running
    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        printf "${YELLOW}Proxy already running${NC} (PID $(cat "$PID_FILE"))\n"
        echo "Run '$0 down' first."
        exit 1
    fi

    mkdir -p "$WORK_DIR"

    echo ""
    printf "${BOLD}Docker Proxy Test Environment${NC}\n"
    echo "=============================="

    # Generate TLS certs if needed
    if [ "$tls" = true ]; then
        generate_certs
        echo ""
    fi

    # Generate nginx config
    generate_nginx_conf "$port" "$tls"

    # Start nginx
    echo "Starting nginx proxy..."
    nginx -c "$CONF_FILE"

    # Wait briefly for nginx to start
    sleep 1

    if [ -f "$PID_FILE" ] && kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        local pid
        pid=$(cat "$PID_FILE")
        printf "  ${GREEN}✓${NC} nginx started (PID %s)\n" "$pid"
    else
        printf "  ${RED}✗${NC} nginx failed to start\n"
        echo "Check logs: $WORK_DIR/error.log"
        exit 1
    fi

    # Save state for status command
    echo "port=$port" > "$STATE_FILE"
    echo "tls=$tls" >> "$STATE_FILE"

    # Print summary
    local scheme="http"
    [ "$tls" = true ] && scheme="https"

    echo ""
    printf "${BOLD}Proxy URL:${NC} %s://127.0.0.1:%s\n" "$scheme" "$port"
    echo ""

    if [ "$tls" = true ]; then
        print_pem_contents
        echo ""
        printf "${BOLD}Verify with:${NC}\n"
        printf "  curl --cacert %s --cert %s --key %s https://127.0.0.1:%s/version\n" \
            "$CERT_DIR/ca.pem" "$CERT_DIR/client-cert.pem" "$CERT_DIR/client-key.pem" "$port"
    else
        printf "${BOLD}Verify with:${NC}\n"
        printf "  curl http://127.0.0.1:%s/version\n" "$port"
    fi

    echo ""
    printf "${GREEN}Docker proxy is ready.${NC}\n"
}

cmd_down() {
    local clean=false

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --clean) clean=true; shift ;;
            *) echo "Unknown option: $1"; exit 1 ;;
        esac
    done

    echo "Stopping Docker proxy..."

    if [ -f "$PID_FILE" ]; then
        local pid
        pid=$(cat "$PID_FILE")
        if kill -0 "$pid" 2>/dev/null; then
            nginx -c "$CONF_FILE" -s stop 2>/dev/null || kill "$pid"
            printf "  ${GREEN}✓${NC} nginx stopped (PID %s)\n" "$pid"
        else
            echo "  nginx was not running"
        fi
    else
        echo "  No PID file found"
    fi

    # Clean up config and logs
    rm -f "$CONF_FILE" "$PID_FILE" "$STATE_FILE"
    rm -f "$WORK_DIR"/access.log "$WORK_DIR"/error.log

    if [ "$clean" = true ]; then
        echo "Cleaning up certificates..."
        rm -rf "$CERT_DIR"
    fi

    # Remove work dir if empty
    rmdir "$WORK_DIR" 2>/dev/null || true

    echo ""
    printf "${GREEN}Docker proxy stopped.${NC}\n"
    if [ "$clean" = false ] && [ -d "$CERT_DIR" ]; then
        echo "Certificates kept in $CERT_DIR (use --clean to remove)"
    fi
}

cmd_status() {
    echo "Docker Proxy Test Environment"
    echo "=============================="

    if [ ! -f "$PID_FILE" ] || ! kill -0 "$(cat "$PID_FILE")" 2>/dev/null; then
        printf "Status:  ${RED}stopped${NC}\n"
        if [ -d "$CERT_DIR" ]; then
            printf "Certs:   ${YELLOW}present${NC} (%s)\n" "$CERT_DIR"
        fi
        return
    fi

    local pid
    pid=$(cat "$PID_FILE")
    local port="$DEFAULT_PORT"
    local tls="false"

    if [ -f "$STATE_FILE" ]; then
        # shellcheck source=/dev/null
        source "$STATE_FILE"
    fi

    local scheme="http"
    [ "$tls" = "true" ] && scheme="https"

    printf "Status:  ${GREEN}running${NC} (PID %s)\n" "$pid"
    printf "URL:     %s://127.0.0.1:%s\n" "$scheme" "$port"
    printf "TLS:     %s\n" "$tls"

    if [ -d "$CERT_DIR" ]; then
        printf "Certs:   %s\n" "$CERT_DIR"
    fi
}

case "${1:-}" in
    up)
        shift
        cmd_up "$@"
        ;;
    down)
        shift
        cmd_down "$@"
        ;;
    status)
        cmd_status
        ;;
    *)
        echo "Usage: $0 {up|down|status}"
        echo ""
        echo "  up [--tls] [--port PORT]  — Start Docker API proxy via nginx"
        echo "  down [--clean]            — Stop proxy (--clean removes certs too)"
        echo "  status                    — Show current proxy state"
        echo ""
        echo "Examples:"
        echo "  $0 up                     # HTTP proxy on port 2376"
        echo "  $0 up --tls               # HTTPS with mTLS on port 2376"
        echo "  $0 up --tls --port 2377   # HTTPS on custom port"
        echo "  $0 down                   # Stop, keep certs"
        echo "  $0 down --clean           # Stop, remove certs"
        exit 1
        ;;
esac
