.PHONY: help build test test-unit clean format generate-schema generate-messages generate-fixtures seed-dev set-plan-community set-plan-starter set-plan-pro set-plan-team set-plan-business set-plan-enterprise test-plan test-merge test-results install-dev-mac install-dev-linux install-dev-windows snmp-up snmp-down snmp-status docker-proxy-up docker-proxy-up-tls docker-proxy-down docker-proxy-status

help:
	@echo "Scanopy Development Commands"
	@echo ""
	@echo "  make fresh-db       - Clean and set up a new database"
	@echo "  make setup-db       - Set up database"
	@echo "  make clean-db       - Clean up database"
	@echo "  make migrate-db     - Run any database migrations"
	@echo "  make seed-dev       - Create dev user after migrate-db (dev@localhost / password123)"
	@echo "  make clean-daemon   - Remove daemon config file"
	@echo "  make dump-db        - Dump database to /scanopy"
	@echo "  make dev-server     - Start server dev environment"
	@echo "  make dev-ui         - Start ui"
	@echo "  make dev-daemon     - Start daemon dev environment"
	@echo "  make dev-container  - Start containerized development environment using docker-compose.test.yml (server + ui + daemon)"
	@echo "  make dev-container-rebuild  - Rebuild and start containerized dev environment"
	@echo "  make dev-container-rebuild-clean  - Rebuild, clean, and start containerized dev environment"
	@echo "  make dev-down       - Stop development containers"
	@echo "  make build          - Build production Docker images (server + daemon)"
	@echo "  make test           - Run all tests (includes integration tests)"
	@echo "  make test-unit      - Run unit tests only (no Docker/database required)"
	@echo "  make lint           - Run all linters"
	@echo "  make format         - Format all code"
	@echo "  make generate-types  - Generate TypeScript types from Rust"
	@echo "  make generate-messages - Generate i18n message functions from messages/*.json"
	@echo "  make generate-fixtures - Regenerate billing-plans.json and features.json from backend"
	@echo "  make generate-schema - Generate database schema diagram (requires tbls)"
	@echo "  make clean          - Clean build artifacts and containers"
	@echo "  make install-dev-mac      - Install development dependencies on macOS"
	@echo "  make install-dev-linux    - Install development dependencies on Linux"
	@echo "  make install-dev-windows  - Install development dependencies on Windows"
	@echo ""
	@echo "Plan Management (sets plan for all organizations):"
	@echo "  make set-plan-community   - Set to Community (free)"
	@echo "  make set-plan-starter     - Set to Starter"
	@echo "  make set-plan-pro         - Set to Pro"
	@echo "  make set-plan-team        - Set to Team"
	@echo "  make set-plan-business    - Set to Business"
	@echo "  make set-plan-enterprise  - Set to Enterprise"
	@echo ""
	@echo "Test Environments:"
	@echo "  make snmp-up         - Start SNMP test environment (6 snmpd containers)"
	@echo "  make snmp-down       - Stop SNMP test environment"
	@echo "  make snmp-status     - Show SNMP test environment status"
	@echo "  make docker-proxy-up - Start Docker proxy test environment (HTTP)"
	@echo "  make docker-proxy-up-tls - Start Docker proxy with TLS"
	@echo "  make docker-proxy-down   - Stop Docker proxy test environment"
	@echo "  make docker-proxy-status - Show Docker proxy status"

fresh-db:
	make clean-db
	make setup-db

setup-db:
	@echo "Setting up PostgreSQL..."
	@docker run -d \
		--name scanopy-postgres \
		-e POSTGRES_USER=postgres \
		-e POSTGRES_PASSWORD=password \
		-e POSTGRES_DB=scanopy \
		-p 5432:5432 \
		postgres:17-alpine || echo "Already running"
	@sleep 3
	@echo "PostgreSQL ready at localhost:5432"

clean-db:
	docker stop scanopy-postgres || true
	docker rm scanopy-postgres || true

migrate-db:
	cd backend && sqlx migrate run --database-url postgresql://postgres:password@localhost:5432/scanopy

seed-dev:
	@echo "Seeding dev database with test user..."
	@docker exec -i scanopy-postgres psql -U postgres -d scanopy < backend/scripts/seed-dev.sql && \
		echo "" && \
		echo "Dev user created! Login with:" && \
		echo "  Email: dev@localhost.com" && \
		echo "  Password: password123"

clean-daemon:
	rm -rf ~/Library/Application\ Support/com.scanopy.daemon

dump-db:
	docker exec -t scanopy-postgres pg_dump -U postgres -d scanopy > ~/dev/scanopy/scanopy.sql  

dev-fresh:
	make fresh-db
	make migrate-db
	@trap 'kill 0' EXIT; \
	cd ui && npm run dev & \
	export DATABASE_URL="postgresql://postgres:password@localhost:5432/scanopy" && \
	cd backend && cargo run --bin server -- --log-level debug --public-url http://localhost:60072

test-merge:
	@if ! git diff --quiet || ! git diff --cached --quiet; then \
		echo "Working tree is dirty. Commit or stash changes first."; \
		echo "  git stash  OR  git add -A && git commit -m 'WIP'"; \
		exit 1; \
	fi
	@current=$$(git branch --show-current); \
	if [ "$$current" = "test" ]; then \
		echo "Already on test branch. Reset first with: git checkout dev && git branch -D test"; \
		exit 1; \
	fi; \
	branches=$$(git worktree list --porcelain | grep '^branch' | sed 's|branch refs/heads/||' | grep -v "$$current" | grep -v '^test$$'); \
	if [ -z "$$branches" ]; then \
		echo "No worktree branches found to merge."; \
		exit 1; \
	fi; \
	echo "Creating test branch from $$current..."; \
	echo "Branches to merge:"; \
	for b in $$branches; do echo "  - $$b"; done; \
	echo ""; \
	git checkout -b test; \
	for branch in $$branches; do \
		echo "Merging $$branch..."; \
		if git merge "$$branch" --no-edit; then \
			echo "  ✓ $$branch merged"; \
		else \
			echo ""; \
			echo "  ✗ $$branch has conflicts. Resolve, then:"; \
			echo "    git add -A && git merge --continue"; \
			remaining=""; \
			skip=true; \
			for b in $$branches; do \
				if [ "$$skip" = false ]; then remaining="$$remaining $$b"; fi; \
				if [ "$$b" = "$$branch" ]; then skip=false; fi; \
			done; \
			if [ -n "$$remaining" ]; then \
				echo "  Then merge remaining branches:"; \
				for b in $$remaining; do echo "    git merge $$b --no-edit"; done; \
			fi; \
			echo "  Then: make generate-types && make test-plan"; \
			exit 1; \
		fi; \
	done; \
	echo ""; \
	echo "All branches merged. Run 'make generate-types && make test-plan' next."

test-plan:
	@echo "Collecting TEST_PLAN.json from worktrees..."
	@echo "var TEST_PLANS = [" > tools/testing/test-plans.js
	@first=true; \
	for f in $$(find .. -maxdepth 2 -name "TEST_PLAN.json" -path "*/scanopy-*/TEST_PLAN.json" 2>/dev/null); do \
		if [ "$$first" = true ]; then first=false; else echo "," >> tools/testing/test-plans.js; fi; \
		cat "$$f" >> tools/testing/test-plans.js; \
		echo "  Found: $$f"; \
	done
	@echo "];" >> tools/testing/test-plans.js
	@echo "Opening test runner..."
	@open tools/testing/test-runner.html 2>/dev/null || xdg-open tools/testing/test-runner.html 2>/dev/null || echo "Open tools/testing/test-runner.html in your browser"

test-results:
	@if [ ! -f TEST_RESULTS.json ]; then \
		echo "TEST_RESULTS.json not found. Export from test runner first."; \
		exit 1; \
	fi
	@echo "Distributing results to worktrees..."
	@for wt in $$(git worktree list --porcelain | grep '^worktree ' | sed 's/^worktree //'); do \
		branch=$$(git -C "$$wt" branch --show-current 2>/dev/null); \
		if [ -z "$$branch" ]; then continue; fi; \
		if grep -q "\"$$branch\"" TEST_RESULTS.json 2>/dev/null; then \
			node -e " \
				const r = require('./TEST_RESULTS.json'); \
				const d = r['$$branch']; \
				if (d) { require('fs').writeFileSync('$$wt/TEST_RESULTS.json', JSON.stringify({'$$branch': d}, null, 2)); } \
			" && echo "  $$branch -> $$wt/TEST_RESULTS.json"; \
		fi; \
	done
	@echo "Done. Agents can read TEST_RESULTS.json in their worktree."

dev-server:
	make generate-fixtures
	@export DATABASE_URL="postgresql://postgres:password@localhost:5432/scanopy" && \
	cd backend && cargo run --bin server -- --log-level debug --public-url http://localhost:60072

dev-daemon:
	cd backend && cargo run --bin daemon -- --server-url http://127.0.0.1:60072 --log-level debug

dev-ui:
	cd ui && npm run dev

dev-container:
	docker compose -f docker-compose.test.yml up

dev-container-rebuild:
	docker compose -f docker-compose.test.yml up --build --force-recreate

dev-container-rebuild-clean:
	docker compose -f docker-compose.test.yml build --no-cache
	docker compose -f docker-compose.test.yml up

dev-down:
	docker compose -f docker-compose.test.yml down --volumes --rmi local

test-unit:
	cd ui && npx vite-node scripts/export-daemon-field-defs.ts --output=../backend/src/tests/daemon-config-frontend-fields.json 2>/dev/null
	@echo "Running frontend tests..."
	cd ui && npm test
	@echo "Running backend unit tests..."
	cd backend && cargo test --lib

test:
	cd ui && npx vite-node scripts/export-daemon-field-defs.ts --output=../backend/src/tests/daemon-config-frontend-fields.json 2>/dev/null
	@echo "Running frontend tests..."
	cd ui && npm test
	@echo "Running backend tests..."
	make dev-down
	rm -rf ./data/daemon_config/*
	@export DATABASE_URL="postgresql://postgres:password@localhost:5432/scanopy_test" && \
	cd backend && cargo test -- --nocapture --test-threads=1

format:
	@echo "Formatting Server..."
	cd backend && cargo fmt
	@echo "Formatting UI..."
	cd ui && npm run format
	@echo "All code formatted!"

lint:
	@echo "Linting Server..."
	cd backend && cargo fmt -- --check && cargo clippy --bin server -- -D warnings
	@echo "Linting Daemon..."
	cd backend && cargo clippy --bin daemon -- -D warnings
	@echo "Generating paraglide i18n..."
	cd ui && npx paraglide-js compile --outdir ./src/lib/paraglide --silent
	@echo "Linting UI..."
	cd ui && npm run lint && npm run format -- --check && npm run check

generate-types: generate-api-types generate-error-codes
	@echo "All types generated successfully"

generate-api-types:
	@echo "Exporting OpenAPI spec from backend..."
	cd backend && cargo test generate_openapi_spec -- --nocapture
	@echo "Generating TypeScript types from OpenAPI spec..."
	cd ui && npm run generate:api
	@echo "TypeScript types exported to ui/src/lib/api/schema.d.ts"

generate-error-codes:
	@echo "Generating error codes from Rust enum..."
	cd backend && cargo run --bin generate-error-codes
	@echo "Merging error messages into en.json..."
	cd ui && node scripts/merge-error-messages.js
	@echo "Error codes generated and merged"

generate-schema:
	@command -v tbls >/dev/null 2>&1 || { echo "Install tbls: brew install k1low/tap/tbls"; exit 1; }
	@rm -rf /tmp/tbls-schema && \
	tbls doc "postgres://postgres:password@localhost:5435/scanopy?sslmode=disable" /tmp/tbls-schema --er-format mermaid --exclude sqlx_migrations --force && \
	awk '/^```mermaid$$/,/^```$$/{if(!/^```/)print}' /tmp/tbls-schema/README.md > ui/static/schema.mermaid && \
	rm -rf /tmp/tbls-schema
	@echo "✅ Generated ui/static/schema.mermaid"

generate-messages:
	@echo "Generating i18n messages..."
	cd ui && npx paraglide-js compile --outdir ./src/lib/paraglide --silent
	@echo "Messages generated successfully"

generate-fixtures:
	@echo "Generating metadata fixtures from backend..."
	cd backend && cargo run --bin generate-fixtures
	@echo "✅ Generated all metadata fixtures in ui/src/lib/data/"

stripe-webhook:
	stripe listen --forward-to http://localhost:60072/api/billing/webhooks

clean:
	make clean-db
	docker compose down -v
	cd backend && cargo clean
	cd ui && rm -rf node_modules dist build .svelte-kit

install-dev-mac:
	@echo "Installing Rust toolchain..."
	rustup install stable
	rustup component add rustfmt clippy
	@echo "Installing Node.js dependencies..."
	cd ui && npm install
	@echo "Installing pre-commit hooks..."
	@command -v pre-commit >/dev/null 2>&1 || { \
		echo "Installing pre-commit via pip..."; \
		pip3 install pre-commit --break-system-packages || pip3 install pre-commit; \
	}
	pre-commit install
	pre-commit install --hook-type pre-push
	@echo "Development dependencies installed!"
	@echo "Note: Run 'source ~/.zshrc' to update your PATH, or restart your terminal"

install-dev-linux:
	@echo "Installing Rust toolchain..."
	rustup install stable
	rustup component add rustfmt clippy
	@echo "Installing Node.js dependencies..."
	cd ui && npm install
	@echo "Installing pre-commit hooks..."
	@command -v pre-commit >/dev/null 2>&1 || { \
		echo "Installing pre-commit via pip..."; \
		pip3 install pre-commit --break-system-packages || pip3 install pre-commit; \
	}
	pre-commit install
	pre-commit install --hook-type pre-push
	@echo ""
	@echo "Development dependencies installed!"

install-dev-windows:
	@echo "Installing native Windows development dependencies..."
	@echo "Installing Rust toolchain..."
	rustup install stable
	rustup component add rustfmt clippy
	@echo "Installing Node.js dependencies..."
	cd ui && npm install
	@echo ""
	@echo "Development dependencies installed!"
	@echo ""
	@echo "Tip: Install pre-commit for git hooks: pip install pre-commit"
	@echo "     Then run: pre-commit install && pre-commit install --hook-type pre-push"

# Plan management commands - set all organizations to a specific plan
set-plan-community:
	@echo "Setting all organizations to Community plan..."
	@docker exec -t scanopy-postgres psql -U postgres -d scanopy -c \
		"UPDATE organizations SET plan = '{\"type\": \"Community\", \"base_cents\": 0, \"rate\": \"Month\", \"trial_days\": 0, \"seat_cents\": null, \"network_cents\": null, \"included_seats\": null, \"included_networks\": null}'::jsonb"
	@echo "Done!"

set-plan-free:
	@echo "Setting all organizations to Free plan..."
	@docker exec -t scanopy-postgres psql -U postgres -d scanopy -c \
		"UPDATE organizations SET plan = '{\"type\": \"Free\", \"base_cents\": 999, \"rate\": \"Month\", \"trial_days\": 7, \"seat_cents\": null, \"network_cents\": null, \"included_seats\": 1, \"included_networks\": 1, \"included_hosts\": 25}'::jsonb"
	@echo "Done!"

set-plan-starter:
	@echo "Setting all organizations to Starter plan..."
	@docker exec -t scanopy-postgres psql -U postgres -d scanopy -c \
		"UPDATE organizations SET plan = '{\"type\": \"Starter\", \"base_cents\": 999, \"rate\": \"Month\", \"trial_days\": 7, \"seat_cents\": null, \"network_cents\": null, \"included_seats\": 1, \"included_networks\": 1}'::jsonb"
	@echo "Done!"

set-plan-pro:
	@echo "Setting all organizations to Pro plan..."
	@docker exec -t scanopy-postgres psql -U postgres -d scanopy -c \
		"UPDATE organizations SET plan = '{\"type\": \"Pro\", \"base_cents\": 1999, \"rate\": \"Month\", \"trial_days\": 7, \"seat_cents\": null, \"network_cents\": null, \"included_seats\": 1, \"included_networks\": 3}'::jsonb"
	@echo "Done!"

set-plan-team:
	@echo "Setting all organizations to Team plan..."
	@docker exec -t scanopy-postgres psql -U postgres -d scanopy -c \
		"UPDATE organizations SET plan = '{\"type\": \"Team\", \"base_cents\": 3999, \"rate\": \"Month\", \"trial_days\": 7, \"seat_cents\": 1000, \"network_cents\": 800, \"included_seats\": 5, \"included_networks\": 5}'::jsonb"
	@echo "Done!"

set-plan-business:
	@echo "Setting all organizations to Business plan..."
	@docker exec -t scanopy-postgres psql -U postgres -d scanopy -c \
		"UPDATE organizations SET plan = '{\"type\": \"Business\", \"base_cents\": 5999, \"rate\": \"Month\", \"trial_days\": 14, \"seat_cents\": 800, \"network_cents\": 500, \"included_seats\": 10, \"included_networks\": 25}'::jsonb"
	@echo "Done!"

set-plan-enterprise:
	@echo "Setting all organizations to Enterprise plan..."
	@docker exec -t scanopy-postgres psql -U postgres -d scanopy -c \
		"UPDATE organizations SET plan = '{\"type\": \"Enterprise\", \"base_cents\": 0, \"rate\": \"Month\", \"trial_days\": 0, \"seat_cents\": null, \"network_cents\": null, \"included_seats\": null, \"included_networks\": null}'::jsonb"
	@echo "Done!"

set-plan-demo:
	@echo "Setting all organizations to Demo plan..."
	@docker exec -t scanopy-postgres psql -U postgres -d scanopy -c \
		"UPDATE organizations SET plan = '{\"type\": \"Demo\", \"base_cents\": 0, \"rate\": \"Month\", \"trial_days\": 0, \"seat_cents\": null, \"network_cents\": null, \"included_seats\": null, \"included_networks\": null}'::jsonb"
	@echo "Done!"

# Test Environments

snmp-verify:
	tools/snmp/snmp-test-env.sh verify

snmp-status:
	tools/snmp/snmp-test-env.sh status

docker-proxy-up:
	tools/docker-proxy/docker-proxy-test-env.sh up

docker-proxy-up-tls:
	tools/docker-proxy/docker-proxy-test-env.sh up --tls

docker-proxy-down:
	tools/docker-proxy/docker-proxy-test-env.sh down

docker-proxy-status:
	tools/docker-proxy/docker-proxy-test-env.sh status
