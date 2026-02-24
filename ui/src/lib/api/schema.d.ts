export interface paths {
    "/api/auth/check-email": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["check_email"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/forgot-password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["forgot_password"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/login": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["login"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/logout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["logout"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/me": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["get_current_user"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/oidc/{slug}/unlink": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["unlink_oidc_account"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/onboarding-state": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get current onboarding state from session */
        get: operations["onboarding_state"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/onboarding-step": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Store onboarding step in session */
        post: operations["onboarding_step"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/register": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["register"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/request-email-change": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["request_email_change"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/resend-verification": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["resend_verification"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/reset-password": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["reset_password"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/setup": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Store pre-registration setup data (org name, networks, seed preference) in session */
        post: operations["setup"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/update": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["update_password_auth"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/auth/verify-email": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post: operations["verify_email"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/change-plan": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Change billing plan
         * @description Upgrades or downgrades the organization's billing plan.
         */
        post: operations["change_plan"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/change-plan/preview": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Preview plan change (shows overage counts) */
        get: operations["preview_plan_change"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/checkout": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Create a checkout session */
        post: operations["create_checkout_session"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/inquiry": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Submit enterprise plan inquiry
         * @description Updates Brevo contact/company with inquiry data, creates a deal, and
         *     tracks an event for automation triggers. Requires authentication to
         *     link the inquiry to an organization.
         */
        post: operations["submit_enterprise_inquiry"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/plans": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get available billing plans */
        get: operations["get_billing_plans"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/portal": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Create a billing portal session */
        post: operations["create_portal_session"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/setup-payment-method": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Setup payment method (collect card without charging) */
        post: operations["setup_payment_method"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/webhooks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Handle Stripe webhook
         * @description Internal endpoint for Stripe webhook callbacks.
         */
        post: operations["handle_webhook"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/config": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get public server configuration
         * @description Returns public configuration settings like OIDC providers, billing status, etc.
         */
        get: operations["get_public_config"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/daemons/register": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Register a new Daemon
         * @description Internal endpoint for daemon self-registration. Creates a host entry
         *     and sets up default discovery jobs for the daemon.
         */
        post: operations["register_daemon"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/daemons/{id}/heartbeat": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Receive daemon heartbeat (DEPRECATED - for backwards compatibility with pre-v0.14.0 daemons)
         * @description Internal endpoint for legacy daemons to send periodic heartbeats.
         *     New daemons (v0.14.0+) use the /request-work endpoint which includes heartbeat functionality.
         *     This endpoint is kept for backwards compatibility and will be removed in a future version.
         */
        post: operations["receive_heartbeat"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/daemons/{id}/request-work": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Request work from server
         * @description Internal endpoint for daemons to poll for pending discovery sessions.
         *     Also updates heartbeat and returns any pending cancellation requests.
         *     Returns tuple of (next_session, should_cancel).
         */
        post: operations["receive_work_request"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/daemons/{id}/startup": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Daemon startup handshake
         * @description Internal endpoint for daemons to report their version on startup.
         *     Updates the daemon's version and last_seen timestamp, returns server capabilities.
         */
        post: operations["daemon_startup"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/daemons/{id}/update-capabilities": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Update Daemon capabilities
         * @description Internal endpoint for daemons to report their current capabilities.
         */
        post: operations["update_capabilities"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/github-stars": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get GitHub star count
         * @description Returns the current star count for the Scanopy GitHub repository.
         */
        get: operations["get_stars"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/metadata": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get metadata registry
         * @description Returns metadata about all entity types, service definitions, and other system metadata.
         */
        get: operations["get_metadata_registry"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/daemon": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Daemon API Keys */
        get: operations["list_daemon_api_keys"];
        put?: never;
        /** Create Daemon API Key */
        post: operations["create_daemon_api_key"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/daemon/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk delete daemon_api_keys
         * @description Returns 409 Conflict if any key is currently assigned to a daemon.
         */
        post: operations["bulk_delete_daemon_api_keys"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/daemon/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Daemon API Keys to CSV
         * @description Export all Daemon API Keys matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_daemon_api_keys_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/daemon/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Daemon API Key by ID */
        get: operations["get_daemon_api_key_by_id"];
        /** Update a Daemon API Key */
        put: operations["update_daemon_api_key"];
        post?: never;
        /**
         * Delete daemon_api_key
         * @description Returns 409 Conflict if the key is currently assigned to a daemon.
         */
        delete: operations["delete_daemon_api_key"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/daemon/{id}/rotate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Rotate a Daemon API Key */
        post: operations["rotate_key_handler"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/keys": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get all user API keys for the current user */
        get: operations["get_all_user_api_keys"];
        put?: never;
        /** Create a new user API key */
        post: operations["create_user_api_key"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/keys/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete user API keys */
        post: operations["bulk_delete_user_api_keys"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/keys/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export User API Keys to CSV
         * @description Export all User API Keys matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_user_api_keys_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/keys/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get a user API key by ID */
        get: operations["get_user_api_key_by_id"];
        /** Update a user API key */
        put: operations["update_user_api_key"];
        post?: never;
        /** Delete a user API key */
        delete: operations["delete_user_api_key"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/auth/keys/{id}/rotate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Rotate a user API key */
        post: operations["rotate_user_api_key"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/bindings": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Bindings */
        get: operations["list_bindings"];
        put?: never;
        /**
         * Create a new Binding
         * @description Creates a binding that associates a service with a port or interface.
         *
         *     ### Binding Types
         *
         *     - **Interface binding**: Service is present at an interface (IP address) without a specific port.
         *       Used for non-port-bound services like gateways.
         *     - **Port binding (specific interface)**: Service listens on a specific port on a specific interface.
         *     - **Port binding (all interfaces)**: Service listens on a specific port on all interfaces
         *       (`interface_id: null`).
         *
         *     ### Validation and Deduplication Rules
         *
         *     - **Conflict detection**: Interface bindings conflict with port bindings on the same interface.
         *       A port binding on all interfaces conflicts with any interface binding for the same service.
         *     - **All-interfaces precedence**: When creating a port binding with `interface_id: null`,
         *       any existing specific-interface bindings for the same port are automatically removed,
         *       as they are superseded by the all-interfaces binding.
         */
        post: operations["create_binding"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/bindings/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Bindings */
        post: operations["bulk_delete_bindings"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/bindings/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Bindings to CSV
         * @description Export all Bindings matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_bindings_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/bindings/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Binding by ID */
        get: operations["get_binding_by_id"];
        /**
         * Update a Binding
         * @description Updates an existing binding. The same conflict detection rules from binding creation apply.
         *
         *     ## Validation Rules
         *
         *     - **Conflict detection**: The updated binding must not conflict with other bindings on the
         *       same service. Interface bindings conflict with port bindings on the same interface.
         */
        put: operations["update_binding"];
        post?: never;
        /** Delete Binding */
        delete: operations["delete_binding"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get all daemons
         * @description Returns all daemons accessible to the user.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["get_daemons"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Daemons */
        post: operations["bulk_delete_daemons"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Daemons to CSV
         * @description Export all Daemons matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_daemons_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/provision": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Pre-provision a ServerPoll mode daemon
         * @description Creates a daemon record on the server before the daemon is installed.
         *     This is only for ServerPoll mode where the server initiates connections to the daemon.
         *     For DaemonPoll mode, daemons self-register on startup.
         *
         *     Returns the daemon record and an API key that must be configured on the daemon.
         */
        post: operations["provision_daemon"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get daemon by ID
         * @description Returns a specific daemon with computed version status.
         */
        get: operations["get_daemon_by_id"];
        put?: never;
        post?: never;
        /** Delete Daemon */
        delete: operations["delete_daemon"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/{id}/retry-connection": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Retry connection to unreachable daemon
         * @description Resets the is_unreachable flag for a daemon that was marked unreachable
         *     due to repeated polling failures. The poller will attempt to contact
         *     the daemon again on the next cycle.
         */
        post: operations["retry_daemon_connection"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/dashboard/summary": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get dashboard summary
         * @description Returns aggregated dashboard data including network metrics, daemon health,
         *     recent discoveries, and plan usage.
         */
        get: operations["get_dashboard_summary"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Discoveries */
        get: operations["list_discoveries"];
        put?: never;
        /** Create new Discovery */
        post: operations["create_discovery"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/active-sessions": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get active Discovery Sessions */
        get: operations["get_active_sessions"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Discoveries */
        post: operations["bulk_delete_discoveries"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Discoveries to CSV
         * @description Export all Discoveries matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_discoveries_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/start-session": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Start a Discovery Session */
        post: operations["start_session"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Discovery by ID */
        get: operations["get_discovery_by_id"];
        /** Update Discovery */
        put: operations["update_discovery"];
        post?: never;
        /** Delete Discovery */
        delete: operations["delete_discovery"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/{session_id}/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Cancel a Discovery Session */
        post: operations["cancel_discovery"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/discovery/{session_id}/update": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Receive discovery progress update from daemon
         * @description Internal endpoint for daemons to report discovery progress.
         */
        post: operations["receive_discovery_update"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/groups": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all Groups
         * @description Returns all groups the authenticated user has access to.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["get_all_groups"];
        put?: never;
        /** Create a new Group */
        post: operations["create_group"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/groups/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Groups */
        post: operations["bulk_delete_groups"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/groups/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Groups to CSV
         * @description Export all Groups matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_groups_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/groups/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Group by ID */
        get: operations["get_group_by_id"];
        /** Update a Group */
        put: operations["update_group"];
        post?: never;
        /** Delete Group */
        delete: operations["delete_group"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all hosts
         * @description Returns all hosts the authenticated user has access to, with their
         *     interfaces, ports, and services included. Supports pagination via
         *     `limit` and `offset` query parameters, and ordering via `group_by`,
         *     `order_by`, and `order_direction`.
         */
        get: operations["get_all_hosts"];
        put?: never;
        /**
         * Create a new host
         * @description Creates a host with optional interfaces, ports, and services.
         *     The `source` field is automatically set to `Manual`.
         *
         *     ### Tag Validation
         *
         *     - Tags must exist and belong to your organization
         *     - Duplicate tag UUIDs are automatically deduplicated
         *     - Invalid or cross-organization tag UUIDs return a 400 error
         */
        post: operations["create_host"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk delete hosts
         * @description Deletes multiple hosts in a single request. The request body should be
         *     an array of host IDs to delete. Fails if any host has an associated daemon.
         */
        post: operations["bulk_delete_hosts"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/discovery": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Internal endpoint for daemon discovery
         * @description Used by daemons to report discovered hosts. Accepts full entities with
         *     pre-generated IDs. Uses upsert behavior to merge with existing hosts.
         *
         *     Tagged as "internal" - included in OpenAPI spec for client generation
         *     but hidden from public documentation.
         */
        post: operations["create_host_discovery"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Hosts to CSV
         * @description Export all Hosts matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_hosts_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/export/zip": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export hosts with children to ZIP
         * @description Exports all hosts matching the filter criteria along with their children
         *     (interfaces, ports, services, if_entries) as a ZIP archive containing
         *     separate CSV files for each entity type.
         */
        get: operations["export_hosts_zip"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/{destination_host}/consolidate/{other_host}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Consolidate hosts
         * @description Merges all interfaces, ports, and services from `other_host` into
         *     `destination_host`, then deletes `other_host`. Both hosts must be
         *     on the same network.
         *
         *     ### Merge Behavior
         *
         *     - **Interfaces**: Transferred to destination. If an interface with matching subnet+IP or MAC
         *       already exists on destination, bindings are remapped to use the existing interface.
         *     - **Ports**: Transferred to destination. If a port with the same number and protocol already
         *       exists, bindings are remapped to use the existing port.
         *     - **Services**: Transferred to destination with deduplication.
         *       See [upsert behavior](https://scanopy.net/docs/discovery/#upsert-behavior) for details.
         *
         *     ### Restrictions
         *
         *     - Cannot consolidate a host with itself.
         *     - Cannot consolidate a host that has a daemon - consolidate into it instead.
         */
        put: operations["consolidate_hosts"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/hosts/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get a host by ID
         * @description Returns a single host with its interfaces, ports, and services.
         */
        get: operations["get_host_by_id"];
        /**
         * Update a host
         * @description Updates host properties. Children (interfaces, ports, services)
         *     are managed via their own endpoints.
         *
         *     ### Tag Validation
         *
         *     - Tags must exist and belong to your organization
         *     - Duplicate tag UUIDs are automatically deduplicated
         *     - Invalid or cross-organization tag UUIDs return a 400 error
         */
        put: operations["update_host"];
        post?: never;
        /**
         * Delete a host
         * @description Prevents deletion if the host has a daemon associated with it
         */
        delete: operations["delete_host"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/if-entries": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all ifTable Entries */
        get: operations["list_iftable_entries"];
        put?: never;
        /**
         * Create a new IfEntry
         * @description Creates an SNMP ifTable entry for a host. These are typically created by
         *     SNMP discovery, but can also be created manually.
         */
        post: operations["create_if_entry"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/if-entries/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete ifTable Entries */
        post: operations["bulk_delete_iftable_entries"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/if-entries/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export ifTable Entries to CSV
         * @description Export all ifTable Entries matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_iftable_entries_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/if-entries/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get ifTable Entry by ID */
        get: operations["get_iftable_entry_by_id"];
        /** Update an IfEntry */
        put: operations["update_if_entry"];
        post?: never;
        /** Delete ifTable Entry */
        delete: operations["delete_iftable_entry"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/interfaces": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Interfaces */
        get: operations["list_interfaces"];
        put?: never;
        /**
         * Create a new interface
         *     Position is automatically assigned to the end of the host's interface list.
         */
        post: operations["create_interface"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/interfaces/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk delete interfaces
         *     Remaining interfaces for affected hosts are renumbered to maintain sequential positions.
         */
        post: operations["bulk_delete_interfaces"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/interfaces/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Interfaces to CSV
         * @description Export all Interfaces matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_interfaces_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/interfaces/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Interface by ID */
        get: operations["get_interface_by_id"];
        /**
         * Update an interface
         *     Position must be within valid range and not conflict with other interfaces.
         */
        put: operations["update_interface"];
        post?: never;
        /**
         * Delete an interface
         *     Remaining interfaces for the host are renumbered to maintain sequential positions.
         */
        delete: operations["delete_interface"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/invites": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all invites */
        get: operations["get_invites"];
        put?: never;
        /** Create invite */
        post: operations["create_invite"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/invites/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get an invite by ID */
        get: operations["get_invite"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/invites/{id}/revoke": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        post?: never;
        /** Revoke an invite */
        delete: operations["revoke_invite"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/networks": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Networks */
        get: operations["list_networks"];
        put?: never;
        /** Create a new network */
        post: operations["create_network"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/networks/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete networks */
        post: operations["bulk_delete_networks"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/networks/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Networks to CSV
         * @description Export all Networks matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_networks_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/networks/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Network by ID */
        get: operations["get_network_by_id"];
        /** Update a network */
        put: operations["update_network"];
        post?: never;
        /** Delete a network */
        delete: operations["delete_network"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get the current user's organization */
        get: operations["get_organization"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Update organization name */
        put: operations["update_org_name"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{id}/populate-demo": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Populate demo data (only available for demo organizations) */
        post: operations["populate_demo_data"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{id}/reset": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Reset all organization data (delete all entities except organization and owner user) */
        post: operations["reset"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ports": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Ports */
        get: operations["list_ports"];
        put?: never;
        /** Create a new port */
        post: operations["create_port"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ports/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Ports */
        post: operations["bulk_delete_ports"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ports/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Ports to CSV
         * @description Export all Ports matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_ports_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ports/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Port by ID */
        get: operations["get_port_by_id"];
        /** Update a port */
        put: operations["update_port"];
        post?: never;
        /** Delete Port */
        delete: operations["delete_port"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/services": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all services
         * @description Returns all services the authenticated user has access to.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["get_all_services"];
        put?: never;
        /**
         * Create a new service
         * @description Creates a service with optional bindings to interfaces or ports.
         *     The `id`, `created_at`, `updated_at`, and `source` fields are generated server-side.
         *     Bindings are specified without `service_id` or `network_id` - these are assigned automatically.
         *
         *     ### Binding Validation Rules
         *
         *     - **Cross-host validation**: All bindings must reference ports/interfaces that belong to the
         *       service's host. Bindings referencing entities from other hosts will be rejected.
         *     - **Deduplication**: Duplicate bindings in the same request are automatically deduplicated.
         *     - **All-interfaces precedence**: If a port binding with `interface_id: null` (all interfaces)
         *       is included, any specific-interface bindings for the same port are automatically removed.
         *     - **Conflict detection**: Interface bindings conflict with port bindings on the same interface.
         *       A port binding on all interfaces conflicts with any interface binding.
         */
        post: operations["create_service"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/services/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Services */
        post: operations["bulk_delete_services"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/services/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Services to CSV
         * @description Export all Services matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_services_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/services/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Service by ID */
        get: operations["get_service_by_id"];
        /**
         * Update a service
         * @description Updates an existing service. All binding validation rules from service creation apply here as well.
         *
         *     ## Binding Validation Rules
         *
         *     - **Cross-host validation**: All bindings must reference ports/interfaces that belong to the
         *       service's host. Bindings referencing entities from other hosts will be rejected.
         *     - **Deduplication**: Duplicate bindings are automatically deduplicated.
         *     - **All-interfaces precedence**: If a port binding with `interface_id: null` (all interfaces)
         *       is included, any specific-interface bindings for the same port are automatically removed.
         *     - **Conflict detection**: Interface bindings conflict with port bindings on the same interface.
         */
        put: operations["update_service"];
        post?: never;
        /** Delete Service */
        delete: operations["delete_service"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Shares */
        get: operations["list_shares"];
        put?: never;
        /** Create a new share */
        post: operations["create_share"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Shares */
        post: operations["bulk_delete_shares"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Shares to CSV
         * @description Export all Shares matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_shares_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/public/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Get share metadata
         * @description Does not include any topology data
         */
        get: operations["get_public_share_metadata"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/public/{id}/verify": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Verify password for a password-protected share (returns success/failure only) */
        post: operations["verify_share_password"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/shares/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Share by ID */
        get: operations["get_share_by_id"];
        /** Update a share */
        put: operations["update_share"];
        post?: never;
        /** Delete Share */
        delete: operations["delete_share"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/snmp-credentials": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all SNMP Credentials
         * @description Returns all SNMP Credentials in the authenticated user's organization.
         */
        get: operations["get_all_snmp_credentials"];
        put?: never;
        /**
         * Create a new SNMP Credential
         * @description Creates an SNMP credential scoped to your organization. Credential names must
         *     be unique within the organization.
         *
         *     ### Validation
         *
         *     - Name must be 1-100 characters
         *     - Name must be unique within your organization
         *     - Community string is required
         */
        post: operations["create_snmp_credential"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/snmp-credentials/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete SNMP Credential */
        post: operations["bulk_delete_snmp_credentials"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/snmp-credentials/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export SNMP Credentials to CSV
         * @description Export all SNMP Credentials matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_snmp_credentials_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/snmp-credentials/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get SNMP Credential by ID */
        get: operations["get_snmp_credential_by_id"];
        /** Update SNMP Credential */
        put: operations["update_snmp_credential"];
        post?: never;
        /** Delete SNMP credential */
        delete: operations["delete_snmp_credential"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/subnets": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all subnets
         * @description Returns all subnets accessible to the authenticated user or daemon.
         *     Daemons can only access subnets within their assigned network.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["list_subnets"];
        put?: never;
        /** Create a new subnet */
        post: operations["create_subnet"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/subnets/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Subnets */
        post: operations["bulk_delete_subnets"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/subnets/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Subnets to CSV
         * @description Export all Subnets matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_subnets_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/subnets/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Subnet by ID */
        get: operations["get_subnet_by_id"];
        /**
         * Update a subnet
         * @description Updates subnet properties. If the CIDR is being changed, validates that
         *     all existing interfaces on this subnet have IPs within the new CIDR range.
         */
        put: operations["update_subnet"];
        post?: never;
        /** Delete Subnet */
        delete: operations["delete_subnet"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all tags
         * @description Returns all tags in the authenticated user's organization.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["get_all_tags"];
        put?: never;
        /**
         * Create a new tag
         * @description Creates a tag scoped to your organization. Tag names must be unique within the organization.
         *
         *     ### Validation
         *
         *     - Name must be 1-100 characters (empty names are rejected)
         *     - Name must be unique within your organization
         */
        post: operations["create_tag"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/assign": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /**
         * Set all tags for an entity
         * @description Replaces all tags on an entity with the provided list.
         *
         *     ### Validation
         *
         *     - Entity type must be taggable (Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey)
         *     - All tags must exist and belong to your organization
         */
        put: operations["set_entity_tags"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/assign/bulk-add": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk add a tag to multiple entities
         * @description Adds a single tag to multiple entities of the same type. This is useful for batch tagging operations.
         *
         *     ### Validation
         *
         *     - Entity type must be taggable (Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey)
         *     - Tag must exist and belong to your organization
         *     - Entities that already have the tag are silently skipped
         */
        post: operations["bulk_add_tag"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/assign/bulk-remove": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk remove a tag from multiple entities
         * @description Removes a single tag from multiple entities of the same type.
         *
         *     ### Validation
         *
         *     - Entity type must be taggable (Host, Service, Subnet, Group, Network, Discovery, Daemon, DaemonApiKey, UserApiKey)
         *     - Entities that don't have the tag are silently skipped
         */
        post: operations["bulk_remove_tag"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Tags */
        post: operations["bulk_delete_tags"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Tags to CSV
         * @description Export all Tags matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_tags_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/tags/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Tag by ID */
        get: operations["get_tag_by_id"];
        /** Update Tag */
        put: operations["update_tag"];
        post?: never;
        /** Delete Tag */
        delete: operations["delete_tag"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get all topologies */
        get: operations["get_all_topologies"];
        put?: never;
        /** Create topology */
        post: operations["create_topology"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Topologies to CSV
         * @description Export all Topologies matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_topologies_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Topology by ID */
        get: operations["get_topology_by_id"];
        put: operations["update_topology"];
        post?: never;
        /** Delete Topology */
        delete: operations["delete_topology"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/edge-handles": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Update an edge's handles
         * @description Lightweight endpoint for edge reconnect operations. Instead of sending the entire
         *     topology, only sends the edge ID and new handle positions.
         *     Fixes HTTP 413 errors on edge reconnect operations for large topologies.
         */
        post: operations["update_edge_handles"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/lock": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Lock a topology */
        post: operations["lock"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/metadata": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Update topology metadata
         * @description Lightweight endpoint for editing topology name and parent. Instead of sending
         *     the entire topology (which includes all hosts, interfaces, services, etc.),
         *     only sends the metadata fields.
         *     Fixes HTTP 413 errors on metadata edit operations for large topologies.
         */
        post: operations["update_metadata"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/node-position": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Update a single node's position
         * @description Lightweight endpoint for drag operations. Instead of sending the entire topology
         *     (which can be several megabytes), only sends the node ID and new position.
         *     Fixes HTTP 413 errors on drag operations for large topologies.
         */
        post: operations["update_node_position"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/node-resize": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Update a node's size and position
         * @description Lightweight endpoint for subnet resize operations. Instead of sending the entire
         *     topology, only sends the node ID, new size, and new position.
         *     Fixes HTTP 413 errors on resize operations for large topologies.
         */
        post: operations["update_node_resize"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/rebuild": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Rebuild topology layout */
        post: operations["rebuild"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/refresh": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Refresh topology data */
        post: operations["refresh"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/unlock": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Unlock a topology */
        post: operations["unlock"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all users */
        get: operations["get_all_users"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete users */
        post: operations["bulk_delete_users"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Users to CSV
         * @description Export all Users matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_users_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get user by ID */
        get: operations["get_user_by_id"];
        /** Update your own user record */
        put: operations["update_user"];
        post?: never;
        /** Delete a user */
        delete: operations["delete_user"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/users/{id}/admin": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        /** Admin update user (for changing permissions) */
        put: operations["admin_update_user"];
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/version": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get API version information */
        get: operations["get_version"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
}
export type webhooks = Record<string, never>;
export interface components {
    schemas: {
        /** @description Error response type for API errors (no data field) */
        ApiErrorResponse: {
            /** @description Machine-readable error code for i18n translation */
            code?: string | null;
            error?: string | null;
            /** @description API metadata (version info) */
            meta: components["schemas"]["ApiMeta"];
            /** @description Parameters for interpolating into the translated error message */
            params?: {
                [key: string]: unknown;
            } | null;
            success: boolean;
        };
        /**
         * @description API metadata included in all responses
         * @example {
         *       "api_version": 1,
         *       "server_version": "0.14.7"
         *     }
         */
        ApiMeta: {
            /**
             * Format: int32
             * @description API version (integer, increments on breaking changes)
             */
            api_version: number;
            /**
             * @description Server version (semver)
             * @example 0.14.7
             */
            server_version: string;
        };
        ApiResponse: {
            data?: null;
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Binding: {
            /**
             * @description Association between a service and a port / interface that the service is listening on
             * @example {
             *       "created_at": "2026-02-23T22:33:25.667361Z",
             *       "id": "70e7b283-7b56-464a-9265-33b41efb809d",
             *       "interface_id": "550e8400-e29b-41d4-a716-446655440005",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "port_id": "550e8400-e29b-41d4-a716-446655440006",
             *       "service_id": "550e8400-e29b-41d4-a716-446655440007",
             *       "type": "Port",
             *       "updated_at": "2026-02-23T22:33:25.667361Z"
             *     }
             */
            data?: components["schemas"]["BindingBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_BulkDeleteResponse: {
            data?: {
                deleted_count: number;
                requested_count: number;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_BulkTagResponse: {
            /** @description Response for bulk tag operations */
            data?: {
                /** @description Number of entities affected */
                affected_count: number;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_ChangePlanPreview: {
            data?: {
                /** Format: int64 */
                excess_hosts: number;
                /** Format: int64 */
                excess_networks: number;
                /** Format: int64 */
                excess_seats: number;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DaemonApiKey: {
            data?: components["schemas"]["DaemonApiKeyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DaemonApiKeyResponse: {
            data?: {
                api_key: components["schemas"]["DaemonApiKey"];
                key: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DaemonRegistrationResponse: {
            /** @description Daemon registration response from server to daemon */
            data?: {
                daemon: components["schemas"]["Daemon"];
                /** Format: uuid */
                host_id: string;
                server_capabilities?: null | components["schemas"]["ServerCapabilities"];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DaemonResponse: {
            /** @description Daemon response for UI including computed version status */
            data?: components["schemas"]["DaemonBase"] & {
                /** Format: date-time */
                created_at: string;
                /** Format: uuid */
                id: string;
                /** Format: date-time */
                updated_at: string;
                /** @description Computed version status including health and warnings */
                version_status: components["schemas"]["DaemonVersionStatus"];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DashboardSummary: {
            /** @description Dashboard summary response */
            data?: {
                daemons: components["schemas"]["DaemonSummary"][];
                networks: components["schemas"]["NetworkSummary"][];
                plan_usage: components["schemas"]["PlanUsage"];
                recent_discoveries: components["schemas"]["Discovery"][];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Discovery: {
            data?: components["schemas"]["DiscoveryBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_DiscoveryUpdatePayload: {
            /** @description Progress update from daemon to server during discovery */
            data?: {
                /** Format: uuid */
                daemon_id: string;
                discovery_type: components["schemas"]["DiscoveryType"];
                error?: string | null;
                /** Format: date-time */
                finished_at?: string | null;
                /** Format: uuid */
                network_id: string;
                phase: components["schemas"]["DiscoveryPhase"];
                /** Format: int32 */
                progress: number;
                /** Format: uuid */
                session_id: string;
                /** Format: date-time */
                started_at?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Group: {
            /**
             * @example {
             *       "binding_ids": [],
             *       "color": "Blue",
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "description": "HTTP/HTTPS services group",
             *       "edge_style": "Bezier",
             *       "group_type": "RequestPath",
             *       "id": "550e8400-e29b-41d4-a716-446655440008",
             *       "name": "Web Services",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z"
             *     }
             */
            data?: components["schemas"]["GroupBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_HostResponse: {
            /**
             * @description Response type for host endpoints.
             *     Includes children (interfaces, ports, services, if_entries).
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "description": "Primary web server",
             *       "hidden": false,
             *       "hostname": "web-server-01.local",
             *       "id": "550e8400-e29b-41d4-a716-446655440003",
             *       "if_entries": [
             *         {
             *           "admin_status": "Up",
             *           "cdp_address": null,
             *           "cdp_device_id": null,
             *           "cdp_platform": null,
             *           "cdp_port_id": null,
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-44665544000f",
             *           "if_alias": "Uplink to Core Switch",
             *           "if_descr": "GigabitEthernet0/1",
             *           "if_index": 1,
             *           "if_type": 6,
             *           "interface_id": "550e8400-e29b-41d4-a716-446655440005",
             *           "lldp_chassis_id": null,
             *           "lldp_mgmt_addr": null,
             *           "lldp_port_desc": null,
             *           "lldp_port_id": null,
             *           "lldp_sys_desc": null,
             *           "lldp_sys_name": null,
             *           "mac_address": "DE:AD:BE:EF:CA:FE",
             *           "neighbor": null,
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "oper_status": "Up",
             *           "speed_bps": 1000000000,
             *           "updated_at": "2026-01-15T10:30:00Z"
             *         }
             *       ],
             *       "interfaces": [
             *         {
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-446655440005",
             *           "ip_address": "192.168.1.100",
             *           "mac_address": "DE:AD:BE:EF:CA:FE",
             *           "name": "eth0",
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "position": 0,
             *           "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
             *           "updated_at": "2026-01-15T10:30:00Z"
             *         }
             *       ],
             *       "name": "web-server-01",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "ports": [
             *         {
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-446655440006",
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "number": 80,
             *           "protocol": "Tcp",
             *           "type": "Http",
             *           "updated_at": "2026-01-15T10:30:00Z"
             *         }
             *       ],
             *       "services": [
             *         {
             *           "bindings": [
             *             {
             *               "created_at": "2026-02-23T22:33:25.648129Z",
             *               "id": "0ad7f1a0-fc1d-48a3-bb93-c12933e3c923",
             *               "interface_id": "550e8400-e29b-41d4-a716-446655440005",
             *               "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *               "port_id": "550e8400-e29b-41d4-a716-446655440006",
             *               "service_id": "550e8400-e29b-41d4-a716-446655440007",
             *               "type": "Port",
             *               "updated_at": "2026-02-23T22:33:25.648129Z"
             *             }
             *           ],
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-446655440007",
             *           "name": "nginx",
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "position": 0,
             *           "service_definition": "Google Home",
             *           "source": {
             *             "type": "Manual"
             *           },
             *           "tags": [],
             *           "updated_at": "2026-01-15T10:30:00Z",
             *           "virtualization": null
             *         }
             *       ],
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "virtualization": null
             *     }
             */
            data?: {
                chassis_id?: string | null;
                /** Format: date-time */
                created_at: string;
                description?: string | null;
                hidden: boolean;
                hostname?: string | null;
                /** Format: uuid */
                id: string;
                /** @description SNMP ifTable entries */
                if_entries: components["schemas"]["IfEntry"][];
                interfaces: components["schemas"]["Interface"][];
                management_url?: string | null;
                name: string;
                /** Format: uuid */
                network_id: string;
                ports: components["schemas"]["Port"][];
                services: components["schemas"]["Service"][];
                /** Format: uuid */
                snmp_credential_id?: string | null;
                source: components["schemas"]["EntitySource"];
                sys_contact?: string | null;
                sys_descr?: string | null;
                sys_location?: string | null;
                sys_object_id?: string | null;
                tags: string[];
                /** Format: date-time */
                updated_at: string;
                virtualization?: null | components["schemas"]["HostVirtualization"];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_IfEntry: {
            data?: components["schemas"]["IfEntryBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Interface: {
            /**
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *       "id": "550e8400-e29b-41d4-a716-446655440005",
             *       "ip_address": "192.168.1.100",
             *       "mac_address": "DE:AD:BE:EF:CA:FE",
             *       "name": "eth0",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "position": 0,
             *       "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
             *       "updated_at": "2026-01-15T10:30:00Z"
             *     }
             */
            data?: components["schemas"]["InterfaceBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Invite: {
            data?: components["schemas"]["InviteBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_MetadataRegistry: {
            data?: {
                billing_plans: components["schemas"]["TypeMetadata"][];
                concepts: components["schemas"]["EntityMetadata"][];
                discovery_types: components["schemas"]["TypeMetadata"][];
                edge_types: components["schemas"]["TypeMetadata"][];
                entities: components["schemas"]["EntityMetadata"][];
                features: components["schemas"]["TypeMetadata"][];
                group_types: components["schemas"]["TypeMetadata"][];
                permissions: components["schemas"]["TypeMetadata"][];
                ports: components["schemas"]["TypeMetadata"][];
                service_definitions: components["schemas"]["TypeMetadata"][];
                subnet_types: components["schemas"]["TypeMetadata"][];
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Network: {
            /**
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "id": "550e8400-e29b-41d4-a716-446655440002",
             *       "name": "Home Network",
             *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z"
             *     }
             */
            data?: components["schemas"]["NetworkBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_OnboardingStateResponse: {
            /** @description Response from onboarding state endpoint */
            data?: {
                network?: null | components["schemas"]["OnboardingNetworkState"];
                /**
                 * Format: uuid
                 * @description Network ID from pending setup (if any)
                 */
                network_id?: string | null;
                /** @description Organization name from pending setup */
                org_name?: string | null;
                /** @description Current onboarding step (if any) */
                step?: string | null;
                /** @description Use case selection (homelab, company, msp) */
                use_case?: string | null;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Organization: {
            data?: components["schemas"]["OrganizationBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Port: {
            /**
             * @description Port entity with custom serialization that flattens PortType fields.
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *       "id": "550e8400-e29b-41d4-a716-446655440006",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "number": 80,
             *       "protocol": "Tcp",
             *       "type": "Http",
             *       "updated_at": "2026-01-15T10:30:00Z"
             *     }
             */
            data?: components["schemas"]["PortBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_ProvisionDaemonResponse: {
            /**
             * @description Response from provisioning a daemon.
             *     Contains the daemon record and the API key (shown only once).
             */
            data?: {
                /** @description The created daemon record (with version status). */
                daemon: components["schemas"]["DaemonResponse"];
                /**
                 * @description The API key (plaintext) for daemon authentication.
                 *     This is shown only once - store it securely.
                 */
                daemon_api_key: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_PublicConfigResponse: {
            data?: {
                billing_enabled: boolean;
                deployment_type: components["schemas"]["DeploymentType"];
                disable_registration: boolean;
                has_email_opt_in: boolean;
                has_email_service: boolean;
                has_integrated_daemon: boolean;
                needs_cookie_consent: boolean;
                oidc_providers: components["schemas"]["OidcProviderMetadata"][];
                posthog_key?: string | null;
                public_url: string;
                /** Format: int32 */
                server_port: number;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_PublicShareMetadata: {
            /** @description Public share metadata (returned without authentication) */
            data?: {
                /** Format: uuid */
                id: string;
                name: string;
                options: components["schemas"]["ShareOptions"];
                requires_password: boolean;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_ServerCapabilities: {
            /** @description Server capabilities returned on startup/registration */
            data?: {
                /** @description Deprecation warnings for the daemon */
                deprecation_warnings?: components["schemas"]["DeprecationWarning"][];
                /** @description Minimum daemon version supported by this server */
                minimum_daemon_version: string;
                /** @description Server software version */
                server_version: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Service: {
            /**
             * @example {
             *       "bindings": [
             *         {
             *           "created_at": "2026-02-23T22:33:25.662026Z",
             *           "id": "3fe34ab2-7e67-4b32-8588-c83ea54dfc06",
             *           "interface_id": "550e8400-e29b-41d4-a716-446655440005",
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "port_id": "550e8400-e29b-41d4-a716-446655440006",
             *           "service_id": "550e8400-e29b-41d4-a716-446655440007",
             *           "type": "Port",
             *           "updated_at": "2026-02-23T22:33:25.662026Z"
             *         }
             *       ],
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *       "id": "550e8400-e29b-41d4-a716-446655440007",
             *       "name": "nginx",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "position": 0,
             *       "service_definition": "Google Home",
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "virtualization": null
             *     }
             */
            data?: components["schemas"]["ServiceBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_SetupResponse: {
            /** @description Response from setup endpoint */
            data?: {
                /** Format: uuid */
                network_id: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Share: {
            data?: components["schemas"]["ShareBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_SnmpCredential: {
            data?: components["schemas"]["SnmpCredentialBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_String: {
            data?: string;
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Subnet: {
            /**
             * @example {
             *       "cidr": "192.168.1.0/24",
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "description": "Local area network",
             *       "id": "550e8400-e29b-41d4-a716-446655440004",
             *       "name": "LAN",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "subnet_type": "Lan",
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z"
             *     }
             */
            data?: components["schemas"]["SubnetBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Tag: {
            /**
             * @example {
             *       "color": "Green",
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "description": "Production environment resources",
             *       "id": "550e8400-e29b-41d4-a716-44665544000a",
             *       "name": "production",
             *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
             *       "updated_at": "2026-01-15T10:30:00Z"
             *     }
             */
            data?: components["schemas"]["TagBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Topology: {
            data?: components["schemas"]["TopologyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_User: {
            data?: components["schemas"]["UserBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_UserApiKey: {
            data?: components["schemas"]["UserApiKeyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_UserApiKeyResponse: {
            /**
             * @description Response for user API key creation/rotation
             *     Contains the full API key record plus the plaintext key (shown only once)
             */
            data?: {
                api_key: components["schemas"]["UserApiKey"];
                /** @description The plaintext API key - only returned once during creation or rotation */
                key: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Vec_BillingPlan: {
            data?: ((components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Community";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Free";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Starter";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Pro";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Team";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Business";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Enterprise";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "Demo";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "CommercialSelfHosted";
            }))[];
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Vec_DiscoveryUpdatePayload: {
            data?: {
                /** Format: uuid */
                daemon_id: string;
                discovery_type: components["schemas"]["DiscoveryType"];
                error?: string | null;
                /** Format: date-time */
                finished_at?: string | null;
                /** Format: uuid */
                network_id: string;
                phase: components["schemas"]["DiscoveryPhase"];
                /** Format: int32 */
                progress: number;
                /** Format: uuid */
                session_id: string;
                /** Format: date-time */
                started_at?: string | null;
            }[];
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_Vec_Invite: {
            data?: (components["schemas"]["InviteBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_VersionInfo: {
            /** @description Version information for API compatibility checking */
            data?: {
                /**
                 * Format: int32
                 * @description Current API version (integer, increments on breaking changes)
                 */
                api_version: number;
                /** @description Minimum client version that can use this API (optional, for future use) */
                min_compatible_client?: string | null;
                /**
                 * @description Server version (semver)
                 * @example 0.12.10
                 */
                server_version: string;
            };
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_bool: {
            data?: boolean;
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        ApiResponse_u32: {
            /** Format: int32 */
            data?: number;
            error?: string | null;
            meta: components["schemas"]["ApiMeta"];
            success: boolean;
        };
        BillingPlan: (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Community";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Free";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Starter";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Pro";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Team";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Business";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Enterprise";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "Demo";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "CommercialSelfHosted";
        });
        /** @enum {string} */
        BillingRate: "Month" | "Year";
        /**
         * @description Association between a service and a port / interface that the service is listening on
         * @example {
         *       "created_at": "2026-02-23T22:33:25.648333Z",
         *       "id": "89f21030-cc1c-4442-966a-0453de0a20c1",
         *       "interface_id": "550e8400-e29b-41d4-a716-446655440005",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *       "service_id": "550e8400-e29b-41d4-a716-446655440007",
         *       "type": "Port",
         *       "updated_at": "2026-02-23T22:33:25.648333Z"
         *     }
         */
        Binding: components["schemas"]["BindingBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        /** @description The base data for a Binding entity (everything except id, created_at, updated_at) */
        BindingBase: components["schemas"]["BindingType"] & {
            /** Format: uuid */
            network_id: string;
            /** Format: uuid */
            service_id: string;
        };
        /**
         * @description Input for creating or updating a binding within a service.
         *     Used in both CreateHostRequest and UpdateHostRequest.
         *     Client must provide a UUID for the binding.
         */
        BindingInput: {
            /**
             * Format: uuid
             * @description Client-provided UUID for this binding
             */
            id: string;
            /** Format: uuid */
            interface_id: string;
            /** @enum {string} */
            type: "Interface";
        } | {
            /**
             * Format: uuid
             * @description Client-provided UUID for this binding
             */
            id: string;
            /**
             * Format: uuid
             * @description null = bind to all interfaces
             */
            interface_id?: string | null;
            /** Format: uuid */
            port_id: string;
            /** @enum {string} */
            type: "Port";
        };
        /**
         * @description The type of binding - either to an interface or to a port.
         *
         *     Bindings associate a service with network resources (interfaces/ports) on a host.
         *
         *     ## Validation Rules
         *
         *     - All bindings must reference ports/interfaces that belong to the same host as the service.
         *     - Interface bindings conflict with port bindings on the same interface.
         *     - A port binding on all interfaces (`interface_id: null`) conflicts with any interface binding.
         *     - When a port binding with `interface_id: null` is created, it supersedes (removes) any
         *       existing specific-interface bindings for the same port.
         */
        BindingType: {
            /** Format: uuid */
            interface_id: string;
            /** @enum {string} */
            type: "Interface";
        } | {
            /**
             * Format: uuid
             * @description The interface this port binding applies to. If `null`, the binding applies to all
             *     interfaces on the host (and supersedes specific-interface bindings for this port).
             */
            interface_id: string | null;
            /** Format: uuid */
            port_id: string;
            /** @enum {string} */
            type: "Port";
        };
        BulkDeleteResponse: {
            deleted_count: number;
            requested_count: number;
        };
        /** @description Request body for bulk tag operations */
        BulkTagRequest: {
            /** @description The IDs of entities to modify */
            entity_ids: string[];
            /** @description The entity type (e.g., Host, Service, Subnet) */
            entity_type: components["schemas"]["EntityDiscriminants"];
            /**
             * Format: uuid
             * @description The tag ID to add or remove
             */
            tag_id: string;
        };
        /** @description Response for bulk tag operations */
        BulkTagResponse: {
            /** @description Number of entities affected */
            affected_count: number;
        };
        ChangePlanPreview: {
            /** Format: int64 */
            excess_hosts: number;
            /** Format: int64 */
            excess_networks: number;
            /** Format: int64 */
            excess_seats: number;
        };
        ChangePlanRequest: {
            plan: components["schemas"]["BillingPlan"];
            rate: components["schemas"]["BillingRate"];
        };
        /** @description Check email availability request */
        CheckEmailRequest: {
            /** Format: email */
            email: string;
        };
        /** @enum {string} */
        Color: "Pink" | "Rose" | "Red" | "Orange" | "Green" | "Emerald" | "Teal" | "Cyan" | "Blue" | "Indigo" | "Purple" | "Gray" | "Yellow";
        /**
         * @description Input for creating a binding with a service.
         *     `service_id` and `network_id` are assigned by the server after the service is created.
         */
        CreateBindingInput: {
            /** Format: uuid */
            interface_id: string;
            /** @enum {string} */
            type: "Interface";
        } | {
            /** Format: uuid */
            interface_id?: string | null;
            /** Format: uuid */
            port_id: string;
            /** @enum {string} */
            type: "Port";
        };
        CreateCheckoutRequest: {
            plan: components["schemas"]["BillingPlan"];
            url: string;
        };
        /**
         * @description Request type for creating a host with its associated interfaces, ports, and services.
         *     Server assigns `host_id`, `network_id`, and `source` to all children.
         *     Client must provide UUIDs for all entities, enabling services to reference
         *     interfaces/ports by ID in the same request.
         * @example {
         *       "description": "Primary web server",
         *       "hidden": false,
         *       "hostname": "web-server-01.local",
         *       "if_entries": [],
         *       "interfaces": [
         *         {
         *           "id": "550e8400-e29b-41d4-a716-446655440005",
         *           "ip_address": "192.168.1.100",
         *           "mac_address": "DE:AD:BE:EF:12:34",
         *           "name": "eth0",
         *           "position": 0,
         *           "subnet_id": "550e8400-e29b-41d4-a716-446655440004"
         *         }
         *       ],
         *       "name": "web-server-01",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "ports": [
         *         {
         *           "id": "550e8400-e29b-41d4-a716-446655440006",
         *           "number": 80,
         *           "protocol": "Tcp"
         *         }
         *       ],
         *       "services": [
         *         {
         *           "bindings": [
         *             {
         *               "id": "550e8400-e29b-41d4-a716-446655440009",
         *               "interface_id": "550e8400-e29b-41d4-a716-446655440005",
         *               "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *               "type": "Port"
         *             }
         *           ],
         *           "id": "550e8400-e29b-41d4-a716-446655440007",
         *           "name": "nginx",
         *           "position": 0,
         *           "service_definition": "Google Home",
         *           "tags": [],
         *           "virtualization": null
         *         }
         *       ],
         *       "tags": [],
         *       "virtualization": null
         *     }
         */
        CreateHostRequest: {
            chassis_id?: string | null;
            description?: string | null;
            hidden?: boolean;
            hostname?: string | null;
            /** @description SNMP interface entries (ifTable data) - server assigns UUIDs */
            if_entries?: components["schemas"]["IfEntryInput"][];
            /** @description Interfaces to create with this host (client provides UUIDs) */
            interfaces?: components["schemas"]["InterfaceInput"][];
            management_url?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            /** @description Ports to create with this host (client provides UUIDs) */
            ports?: components["schemas"]["PortInput"][];
            /** @description Services to create with this host (can reference interfaces/ports by their UUIDs) */
            services?: components["schemas"]["ServiceInput"][];
            /** Format: uuid */
            snmp_credential_id?: string | null;
            sys_contact?: string | null;
            sys_descr?: string | null;
            sys_location?: string | null;
            sys_object_id?: string | null;
            tags: string[];
            virtualization?: null | components["schemas"]["HostVirtualization"];
        };
        CreateInviteRequest: {
            /** Format: int64 */
            expiration_hours?: number | null;
            network_ids: string[];
            permissions: components["schemas"]["UserOrgPermissions"];
            send_to?: string | null;
        };
        /**
         * @description Request type for creating a service.
         *     Server assigns `id`, `created_at`, `updated_at`, and `source`.
         *     Server also assigns `service_id` and `network_id` to all bindings.
         */
        CreateServiceRequest: {
            /**
             * @description Bindings to create with the service.
             *     `service_id` and `network_id` are assigned by the server.
             */
            bindings?: components["schemas"]["CreateBindingInput"][];
            /** Format: uuid */
            host_id: string;
            name: string;
            /** Format: uuid */
            network_id: string;
            service_definition: string;
            tags: string[];
            virtualization?: null | components["schemas"]["ServiceVirtualization"];
        };
        CreateUpdateShareRequest: {
            password?: string | null;
            share: components["schemas"]["Share"];
        };
        Daemon: components["schemas"]["DaemonBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        DaemonApiKey: components["schemas"]["DaemonApiKeyBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        DaemonApiKeyBase: {
            /** Format: date-time */
            expires_at?: string | null;
            is_enabled?: boolean;
            readonly key: string;
            /** Format: date-time */
            readonly last_used: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            tags: string[];
        };
        DaemonApiKeyResponse: {
            api_key: components["schemas"]["DaemonApiKey"];
            key: string;
        };
        DaemonBase: {
            /**
             * Format: uuid
             * @description Foreign key to API key used for ServerPoll authentication.
             *     NULL for DaemonPoll daemons or those not yet linked to a key.
             */
            api_key_id?: string | null;
            capabilities: components["schemas"]["DaemonCapabilities"];
            /** Format: uuid */
            host_id: string;
            /**
             * @description Whether the daemon is unreachable (for ServerPoll circuit breaker).
             *     Set to true after repeated polling failures, reset via retry-connection endpoint.
             */
            is_unreachable?: boolean;
            /**
             * Format: date-time
             * @description Timestamp of last successful contact with daemon.
             *     NULL for provisioned ServerPoll daemons that haven't been contacted yet.
             */
            readonly last_seen?: string | null;
            mode: components["schemas"]["DaemonMode"];
            name: string;
            /** Format: uuid */
            network_id: string;
            /** @description Whether the daemon is on standby due to plan restrictions (DaemonPoll on Free plan). */
            readonly standby?: boolean;
            tags: string[];
            readonly url: string;
            /**
             * Format: uuid
             * @description User responsible for maintaining this daemon
             */
            user_id: string;
            /** @description Daemon software version (semver format) */
            version?: string | null;
        };
        /** @description Daemon capabilities */
        DaemonCapabilities: {
            has_docker_socket?: boolean;
            interfaced_subnet_ids: string[];
        };
        /**
         * @description Legacy heartbeat payload for backwards compatibility with pre-v0.14.0 daemons.
         *     Old daemons call POST /api/daemons/{id}/heartbeat with this payload.
         */
        DaemonHeartbeatPayload: {
            mode: components["schemas"]["DaemonMode"];
            name: string;
            url: string;
        };
        /**
         * @description Daemon operating mode that determines the communication pattern.
         *
         *     - **DaemonPoll** (formerly "Pull"): Daemon makes outbound connections to the server.
         *       The daemon registers itself and polls for work. Best for daemons behind NAT/firewall.
         *
         *     - **ServerPoll** (formerly "Push"): Server makes connections to the daemon.
         *       Server polls daemon for status and discovery results. Best for DMZ deployments
         *       where daemon cannot make outbound connections.
         * @enum {string}
         */
        DaemonMode: "server_poll" | "daemon_poll";
        /**
         * @description Fields that daemons can be ordered/grouped by.
         * @enum {string}
         */
        DaemonOrderField: "created_at" | "name" | "last_seen" | "updated_at" | "network_id";
        /** @description Daemon registration request from daemon to server */
        DaemonRegistrationRequest: {
            capabilities: components["schemas"]["DaemonCapabilities"];
            /** Format: uuid */
            daemon_id: string;
            mode: components["schemas"]["DaemonMode"];
            name: string;
            /** Format: uuid */
            network_id: string;
            /**
             * @description URL is ignored by server - kept for backwards compat with old daemons.
             *     URL is only set via admin provisioning for ServerPoll daemons.
             */
            url?: string | null;
            /**
             * Format: uuid
             * @description User responsible for maintaining this daemon (from frontend install command)
             *     Optional for backwards compat with old daemons - defaults to nil UUID
             */
            user_id?: string;
            /** @description Daemon software version (optional for backwards compat with old daemons) */
            version?: string | null;
        };
        /** @description Daemon registration response from server to daemon */
        DaemonRegistrationResponse: {
            daemon: components["schemas"]["Daemon"];
            /** Format: uuid */
            host_id: string;
            server_capabilities?: null | components["schemas"]["ServerCapabilities"];
        };
        /** @description Daemon response for UI including computed version status */
        DaemonResponse: components["schemas"]["DaemonBase"] & {
            /** Format: date-time */
            created_at: string;
            /** Format: uuid */
            id: string;
            /** Format: date-time */
            updated_at: string;
            /** @description Computed version status including health and warnings */
            version_status: components["schemas"]["DaemonVersionStatus"];
        };
        /** @description Sent by daemon on startup to report version */
        DaemonStartupRequest: {
            /** @description Daemon software version (semver format) */
            daemon_version: string;
        };
        /**
         * @description Daemon status payload sent when polling for work or in heartbeats.
         *     Used by DaemonPoll mode to send status alongside work requests,
         *     and by ServerPoll mode when processing daemon status.
         */
        DaemonStatusPayload: {
            mode: components["schemas"]["DaemonMode"];
            name: string;
            /** @description URL is ignored by server - kept for backwards compat with old daemons. */
            url?: string | null;
            /** @description Daemon software version (optional for backwards compat) */
            version?: string | null;
        };
        /** @description Daemon health summary */
        DaemonSummary: {
            /** Format: uuid */
            id: string;
            is_unreachable: boolean;
            /** Format: date-time */
            last_seen?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            version_status: components["schemas"]["DaemonVersionStatus"];
        };
        /** @description Daemon version status including health and any warnings */
        DaemonVersionStatus: {
            status: components["schemas"]["VersionHealthStatus"];
            version?: string | null;
            warnings?: components["schemas"]["DeprecationWarning"][];
        };
        /** @description Dashboard summary response */
        DashboardSummary: {
            daemons: components["schemas"]["DaemonSummary"][];
            networks: components["schemas"]["NetworkSummary"][];
            plan_usage: components["schemas"]["PlanUsage"];
            recent_discoveries: components["schemas"]["Discovery"][];
        };
        /** @enum {string} */
        DeploymentType: "cloud" | "commercial" | "community";
        /**
         * @description Severity level for deprecation warnings
         * @enum {string}
         */
        DeprecationSeverity: "Info" | "Warning" | "Critical";
        /** @description Deprecation warning for daemon version */
        DeprecationWarning: {
            message: string;
            severity: components["schemas"]["DeprecationSeverity"];
            sunset_date?: string | null;
        };
        Discovery: components["schemas"]["DiscoveryBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        DiscoveryBase: {
            /** Format: uuid */
            daemon_id: string;
            discovery_type: components["schemas"]["DiscoveryType"];
            name: string;
            /** Format: uuid */
            network_id: string;
            run_type: components["schemas"]["RunType"];
            tags: string[];
        };
        /**
         * @description Request type for daemon discovery - accepts full entities with IDs.
         *     Used internally by daemons for host creation/upsert, NOT the external API.
         *     This supports the discovery workflow where daemons manage entity IDs.
         */
        DiscoveryHostRequest: {
            host: components["schemas"]["Host"];
            /** @description SNMP interface entries (ifTable data) - optional, populated when SNMP is enabled */
            if_entries?: components["schemas"]["IfEntry"][];
            interfaces: components["schemas"]["Interface"][];
            ports: components["schemas"]["Port"][];
            services: components["schemas"]["Service"][];
        };
        DiscoveryMetadata: components["schemas"]["DiscoveryType"] & {
            /** Format: uuid */
            daemon_id: string;
            /** Format: date-time */
            date: string;
        };
        /** @enum {string} */
        DiscoveryPhase: "Pending" | "Starting" | "Started" | "Scanning" | "Complete" | "Failed" | "Cancelled";
        /**
         * @description Protocol that discovered the physical link between network devices
         * @enum {string}
         */
        DiscoveryProtocol: "LLDP" | "CDP";
        DiscoveryType: {
            /** Format: uuid */
            host_id: string;
            /** @enum {string} */
            type: "SelfReport";
        } | {
            host_naming_fallback: components["schemas"]["HostNamingFallback"];
            /**
             * @description Whether to probe raw-socket ports (9100-9107) during endpoint scanning.
             *     Disabled by default to prevent ghost printing on JetDirect printers.
             */
            probe_raw_socket_ports?: boolean;
            /**
             * @description SNMP credentials for querying devices during discovery
             *     Server builds this mapping before initiating discovery
             */
            snmp_credentials?: components["schemas"]["SnmpCredentialMapping"];
            subnet_ids: string[] | null;
            /** @enum {string} */
            type: "Network";
        } | {
            /** Format: uuid */
            host_id: string;
            host_naming_fallback: components["schemas"]["HostNamingFallback"];
            /** @enum {string} */
            type: "Docker";
        };
        /** @description Progress update from daemon to server during discovery */
        DiscoveryUpdatePayload: {
            /** Format: uuid */
            daemon_id: string;
            discovery_type: components["schemas"]["DiscoveryType"];
            error?: string | null;
            /** Format: date-time */
            finished_at?: string | null;
            /** Format: uuid */
            network_id: string;
            phase: components["schemas"]["DiscoveryPhase"];
            /** Format: int32 */
            progress: number;
            /** Format: uuid */
            session_id: string;
            /** Format: date-time */
            started_at?: string | null;
        };
        DockerVirtualization: {
            container_id?: string | null;
            container_name?: string | null;
            /** Format: uuid */
            service_id: string;
        };
        Edge: components["schemas"]["EdgeType"] & {
            /** Format: uuid */
            id: string;
            is_multi_hop: boolean;
            label: string | null;
            /** Format: uuid */
            source: string;
            source_handle: components["schemas"]["EdgeHandle"];
            /** Format: uuid */
            target: string;
            target_handle: components["schemas"]["EdgeHandle"];
        };
        /** @enum {string} */
        EdgeHandle: "Top" | "Bottom" | "Left" | "Right";
        /** @enum {string} */
        EdgeStyle: "Straight" | "SmoothStep" | "Step" | "Bezier" | "SimpleBezier";
        EdgeType: {
            /** @enum {string} */
            edge_type: "Interface";
            /** Format: uuid */
            host_id: string;
        } | {
            /** @enum {string} */
            edge_type: "HostVirtualization";
            /** Format: uuid */
            vm_service_id: string;
        } | {
            /** Format: uuid */
            containerizing_service_id: string;
            /** @enum {string} */
            edge_type: "ServiceVirtualization";
            /** Format: uuid */
            host_id: string;
        } | {
            /** @enum {string} */
            edge_type: "RequestPath";
            /** Format: uuid */
            group_id: string;
            /** Format: uuid */
            source_binding_id: string;
            /** Format: uuid */
            target_binding_id: string;
        } | {
            /** @enum {string} */
            edge_type: "HubAndSpoke";
            /** Format: uuid */
            group_id: string;
            /** Format: uuid */
            source_binding_id: string;
            /** Format: uuid */
            target_binding_id: string;
        } | {
            /** @enum {string} */
            edge_type: "PhysicalLink";
            protocol: components["schemas"]["DiscoveryProtocol"];
            /** Format: uuid */
            source_if_entry_id: string;
            /** Format: uuid */
            target_if_entry_id: string;
        };
        /** @enum {string} */
        EdgeTypeDiscriminants: "Interface" | "HostVirtualization" | "ServiceVirtualization" | "RequestPath" | "HubAndSpoke" | "PhysicalLink";
        /** @description Enterprise plan inquiry request */
        EnterpriseInquiryRequest: {
            /** @description Company name */
            company: string;
            /** @description Contact email */
            email: string;
            /** @description Message/use case description */
            message: string;
            /** @description Contact name */
            name: string;
            /**
             * Format: int64
             * @description Number of networks/sites
             */
            network_count?: number | null;
            /** @description Plan type being inquired about */
            plan_type?: string | null;
            /** @description Team/company size: 1-10, 11-25, 26-50, 51-100, 101-250, 251-500, 501-1000, 1001+ */
            team_size: string;
            /** @description Urgency: immediately, 1-3 months, 3-6 months, exploring */
            urgency?: string | null;
        };
        /** @enum {string} */
        EntityDiscriminants: "Organization" | "Invite" | "Share" | "Network" | "DaemonApiKey" | "UserApiKey" | "User" | "Tag" | "Discovery" | "Daemon" | "Host" | "Service" | "Port" | "Binding" | "Interface" | "IfEntry" | "SnmpCredential" | "Subnet" | "Group" | "Topology" | "Unknown";
        EntityMetadata: {
            color: components["schemas"]["Color"];
            icon: string;
            id: string;
        };
        EntitySource: {
            /** @enum {string} */
            type: "Manual";
        } | {
            /** @enum {string} */
            type: "System";
        } | {
            metadata: components["schemas"]["DiscoveryMetadata"][];
            /** @enum {string} */
            type: "Discovery";
        } | {
            details: components["schemas"]["MatchDetails"];
            metadata: components["schemas"]["DiscoveryMetadata"][];
            /** @enum {string} */
            type: "DiscoveryWithMatch";
        } | {
            /** @enum {string} */
            type: "Unknown";
        };
        ForgotPasswordRequest: {
            /** Format: email */
            email: string;
        };
        /**
         * @example {
         *       "binding_ids": [],
         *       "color": "Blue",
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "description": "HTTP/HTTPS services group",
         *       "edge_style": "Bezier",
         *       "group_type": "RequestPath",
         *       "id": "550e8400-e29b-41d4-a716-446655440008",
         *       "name": "Web Services",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z"
         *     }
         */
        Group: components["schemas"]["GroupBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        GroupBase: {
            /** @description Ordered list of binding IDs for this group. */
            binding_ids: string[];
            color: components["schemas"]["Color"];
            description?: string | null;
            edge_style: components["schemas"]["EdgeStyle"];
            group_type: components["schemas"]["GroupType"];
            name: string;
            /** Format: uuid */
            network_id: string;
            /** @description Will be automatically set to Manual for creation through API */
            source?: components["schemas"]["EntitySource"];
            tags: string[];
        };
        /**
         * @description Fields that groups can be ordered/grouped by.
         * @enum {string}
         */
        GroupOrderField: "created_at" | "name" | "group_type" | "updated_at" | "network_id";
        /** @enum {string} */
        GroupType: "RequestPath" | "HubAndSpoke";
        /**
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "description": "Primary web server",
         *       "hidden": false,
         *       "hostname": "web-server-01.local",
         *       "id": "550e8400-e29b-41d4-a716-446655440003",
         *       "name": "web-server-01",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "virtualization": null
         *     }
         */
        Host: components["schemas"]["HostBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        /**
         * @description Base data for a Host entity (stored in database).
         *     Child entities (interfaces, ports, services) are stored in their own tables
         *     and queried by `host_id`. They are NOT stored on the host.
         */
        HostBase: {
            /** @description LLDP lldpLocChassisId - globally unique device identifier for deduplication */
            chassis_id?: string | null;
            description: string | null;
            hidden: boolean;
            hostname: string | null;
            /** @description URL for device management interface (manual or discovered) */
            management_url?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            /**
             * Format: uuid
             * @description Per-host SNMP credential override (null = use network default)
             */
            snmp_credential_id?: string | null;
            source: components["schemas"]["EntitySource"];
            /** @description SNMP sysContact.0 - admin contact info */
            sys_contact?: string | null;
            /** @description SNMP sysDescr.0 - full system description */
            sys_descr?: string | null;
            /** @description SNMP sysLocation.0 - physical location */
            sys_location?: string | null;
            /** @description SNMP sysObjectID.0 - vendor OID for device identification */
            sys_object_id?: string | null;
            tags: string[];
            virtualization: null | components["schemas"]["HostVirtualization"];
        };
        /** @enum {string} */
        HostNamingFallback: "Ip" | "BestService";
        /**
         * @description Fields that hosts can be ordered/grouped by.
         * @enum {string}
         */
        HostOrderField: "created_at" | "name" | "hostname" | "updated_at" | "virtualized_by" | "network_id";
        /**
         * @description Response type for host endpoints.
         *     Includes children (interfaces, ports, services, if_entries).
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "description": "Primary web server",
         *       "hidden": false,
         *       "hostname": "web-server-01.local",
         *       "id": "550e8400-e29b-41d4-a716-446655440003",
         *       "if_entries": [
         *         {
         *           "admin_status": "Up",
         *           "cdp_address": null,
         *           "cdp_device_id": null,
         *           "cdp_platform": null,
         *           "cdp_port_id": null,
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-44665544000f",
         *           "if_alias": "Uplink to Core Switch",
         *           "if_descr": "GigabitEthernet0/1",
         *           "if_index": 1,
         *           "if_type": 6,
         *           "interface_id": "550e8400-e29b-41d4-a716-446655440005",
         *           "lldp_chassis_id": null,
         *           "lldp_mgmt_addr": null,
         *           "lldp_port_desc": null,
         *           "lldp_port_id": null,
         *           "lldp_sys_desc": null,
         *           "lldp_sys_name": null,
         *           "mac_address": "DE:AD:BE:EF:CA:FE",
         *           "neighbor": null,
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "oper_status": "Up",
         *           "speed_bps": 1000000000,
         *           "updated_at": "2026-01-15T10:30:00Z"
         *         }
         *       ],
         *       "interfaces": [
         *         {
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-446655440005",
         *           "ip_address": "192.168.1.100",
         *           "mac_address": "DE:AD:BE:EF:CA:FE",
         *           "name": "eth0",
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "position": 0,
         *           "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
         *           "updated_at": "2026-01-15T10:30:00Z"
         *         }
         *       ],
         *       "name": "web-server-01",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "ports": [
         *         {
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-446655440006",
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "number": 80,
         *           "protocol": "Tcp",
         *           "type": "Http",
         *           "updated_at": "2026-01-15T10:30:00Z"
         *         }
         *       ],
         *       "services": [
         *         {
         *           "bindings": [
         *             {
         *               "created_at": "2026-02-23T22:33:25.647855Z",
         *               "id": "fb917bdf-3bd8-498f-b0d8-5427a5f1d388",
         *               "interface_id": "550e8400-e29b-41d4-a716-446655440005",
         *               "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *               "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *               "service_id": "550e8400-e29b-41d4-a716-446655440007",
         *               "type": "Port",
         *               "updated_at": "2026-02-23T22:33:25.647855Z"
         *             }
         *           ],
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-446655440007",
         *           "name": "nginx",
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "position": 0,
         *           "service_definition": "Google Home",
         *           "source": {
         *             "type": "Manual"
         *           },
         *           "tags": [],
         *           "updated_at": "2026-01-15T10:30:00Z",
         *           "virtualization": null
         *         }
         *       ],
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "virtualization": null
         *     }
         */
        HostResponse: {
            chassis_id?: string | null;
            /** Format: date-time */
            created_at: string;
            description?: string | null;
            hidden: boolean;
            hostname?: string | null;
            /** Format: uuid */
            id: string;
            /** @description SNMP ifTable entries */
            if_entries: components["schemas"]["IfEntry"][];
            interfaces: components["schemas"]["Interface"][];
            management_url?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            ports: components["schemas"]["Port"][];
            services: components["schemas"]["Service"][];
            /** Format: uuid */
            snmp_credential_id?: string | null;
            source: components["schemas"]["EntitySource"];
            sys_contact?: string | null;
            sys_descr?: string | null;
            sys_location?: string | null;
            sys_object_id?: string | null;
            tags: string[];
            /** Format: date-time */
            updated_at: string;
            virtualization?: null | components["schemas"]["HostVirtualization"];
        };
        /** HostVirtualization */
        HostVirtualization: {
            details: components["schemas"]["ProxmoxVirtualization"];
            /** @enum {string} */
            type: "Proxmox";
        };
        /**
         * @description SNMP ifAdminStatus values per IF-MIB RFC 2863
         * @enum {string}
         */
        IfAdminStatus: "Up" | "Down" | "Testing";
        IfEntry: components["schemas"]["IfEntryBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        IfEntryBase: {
            /** @description SNMP ifAdminStatus: 1=up, 2=down, 3=testing */
            admin_status: components["schemas"]["IfAdminStatus"];
            /** @description Remote management IP from CDP (cdpCacheAddress) */
            cdp_address?: string | null;
            /** @description Remote device ID from CDP (typically hostname, locally unique) */
            cdp_device_id?: string | null;
            /** @description Remote platform from CDP (e.g., "Cisco IOS") */
            cdp_platform?: string | null;
            /** @description Remote port ID from CDP */
            cdp_port_id?: string | null;
            /** Format: uuid */
            host_id: string;
            /** @description SNMP ifAlias - user-configured description */
            if_alias?: string | null;
            /** @description SNMP ifDescr - interface description (e.g., GigabitEthernet0/1) */
            if_descr: string;
            /**
             * Format: int32
             * @description SNMP ifIndex - stable identifier within device
             */
            if_index: number;
            /**
             * Format: int32
             * @description SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)
             */
            if_type: number;
            /**
             * Format: uuid
             * @description FK to Interface entity - this port's IP assignment (must be on same host)
             */
            interface_id?: string | null;
            lldp_chassis_id?: null | components["schemas"]["LldpChassisId"];
            /** @description Remote management IP from LLDP neighbor (lldpRemManAddr) */
            lldp_mgmt_addr?: string | null;
            /** @description Remote port description from LLDP neighbor (lldpRemPortDesc) */
            lldp_port_desc?: string | null;
            lldp_port_id?: null | components["schemas"]["LldpPortId"];
            /** @description Remote system description from LLDP neighbor (lldpRemSysDesc) - platform info */
            lldp_sys_desc?: string | null;
            /** @description Remote system name from LLDP neighbor (lldpRemSysName) */
            lldp_sys_name?: string | null;
            /** @description MAC address from SNMP ifPhysAddress - immutable once set */
            mac_address?: string | null;
            neighbor?: null | components["schemas"]["Neighbor"];
            /** Format: uuid */
            network_id: string;
            /** @description SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown */
            oper_status: components["schemas"]["IfOperStatus"];
            /**
             * Format: int64
             * @description Interface speed from ifSpeed/ifHighSpeed in bits per second
             */
            speed_bps?: number | null;
        };
        /**
         * @description Input for creating an SNMP interface entry (ifTable data).
         *     Used in CreateHostRequest. Server assigns UUIDs since nothing references
         *     IfEntry IDs at creation time (neighbor resolution is done server-side).
         */
        IfEntryInput: {
            admin_status?: null | components["schemas"]["IfAdminStatus"];
            /** @description SNMP ifAlias - user-configured description */
            if_alias?: string | null;
            /** @description SNMP ifDescr - interface description (e.g., GigabitEthernet0/1) */
            if_descr: string;
            /**
             * Format: int32
             * @description SNMP ifIndex - stable identifier within device
             */
            if_index: number;
            /**
             * Format: int32
             * @description SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)
             */
            if_type?: number | null;
            /**
             * Format: uuid
             * @description Optional FK to Interface - links this SNMP port to its IP assignment
             */
            interface_id?: string | null;
            /** @description MAC address from SNMP ifPhysAddress */
            mac_address?: string | null;
            oper_status?: null | components["schemas"]["IfOperStatus"];
            /**
             * Format: int64
             * @description Interface speed in bits per second
             */
            speed_bps?: number | null;
        };
        /**
         * @description SNMP ifOperStatus values per IF-MIB RFC 2863
         * @enum {string}
         */
        IfOperStatus: "Up" | "Down" | "Testing" | "Unknown" | "Dormant" | "NotPresent" | "LowerLayerDown";
        /**
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *       "id": "550e8400-e29b-41d4-a716-446655440005",
         *       "ip_address": "192.168.1.100",
         *       "mac_address": "DE:AD:BE:EF:CA:FE",
         *       "name": "eth0",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "position": 0,
         *       "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
         *       "updated_at": "2026-01-15T10:30:00Z"
         *     }
         */
        Interface: components["schemas"]["InterfaceBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        InterfaceBase: {
            /** Format: uuid */
            host_id: string;
            ip_address: string;
            /** @description MAC address discovered from ARP, SNMP, or Docker - immutable once set */
            mac_address?: string | null;
            name: string | null;
            /** Format: uuid */
            network_id: string;
            /**
             * Format: int32
             * @description Position of this interface in the host's interface list (for ordering)
             */
            position?: number;
            /** Format: uuid */
            subnet_id: string;
        };
        /**
         * @description Input for creating or updating an interface.
         *     Used in both CreateHostRequest and UpdateHostRequest.
         *     Client must provide a UUID for the interface.
         */
        InterfaceInput: {
            /**
             * Format: uuid
             * @description Client-provided UUID for this interface
             */
            id: string;
            ip_address: string;
            mac_address?: string | null;
            name?: string | null;
            /**
             * Format: int32
             * @description Position in the host's interface list (for ordering).
             *     If omitted on create: appends to end of list.
             *     If omitted on update: existing interfaces keep their positions; new interfaces append.
             *     Must be all specified or all omitted across all interfaces in the request.
             */
            position?: number | null;
            /** Format: uuid */
            subnet_id: string;
        };
        Invite: components["schemas"]["InviteBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        InviteBase: {
            /** Format: uuid */
            created_by: string;
            /** Format: date-time */
            expires_at: string;
            network_ids: string[];
            /** Format: uuid */
            organization_id: string;
            permissions: components["schemas"]["UserOrgPermissions"];
            /** @description Optional email address to send the invite to */
            send_to: string | null;
            url: string;
        };
        Ixy: {
            x: number;
            y: number;
        };
        /**
         * @description LLDP Chassis ID subtypes per IEEE 802.1AB.
         *
         *     The chassis ID identifies the remote device. Different network equipment
         *     may use different subtypes depending on configuration and capabilities.
         */
        LldpChassisId: {
            /** @enum {string} */
            subtype: "ChassisComponent";
            /** @description Subtype 1: Chassis component (e.g., backplane serial number) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "InterfaceAlias";
            /** @description Subtype 2: Interface alias (ifAlias from IF-MIB) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "PortComponent";
            /** @description Subtype 3: Port component (e.g., backplane port number) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "MacAddress";
            /** @description Subtype 4: MAC address (most common) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "NetworkAddress";
            /** @description Subtype 5: Network address (IP address stored as string) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "InterfaceName";
            /** @description Subtype 6: Interface name (ifName from IF-MIB) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "LocallyAssigned";
            /** @description Subtype 7: Locally assigned (device-specific identifier) */
            value: string;
        };
        /**
         * @description LLDP Port ID subtypes per IEEE 802.1AB.
         *
         *     The port ID identifies the specific port on the remote device.
         */
        LldpPortId: {
            /** @enum {string} */
            subtype: "InterfaceAlias";
            /** @description Subtype 1: Interface alias (ifAlias from IF-MIB) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "PortComponent";
            /** @description Subtype 2: Port component (e.g., backplane port number) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "MacAddress";
            /** @description Subtype 3: MAC address */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "NetworkAddress";
            /** @description Subtype 4: Network address (IP address stored as string) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "InterfaceName";
            /** @description Subtype 5: Interface name (ifName from IF-MIB) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "AgentCircuitId";
            /** @description Subtype 6: Agent circuit ID (used by some providers) */
            value: string;
        } | {
            /** @enum {string} */
            subtype: "LocallyAssigned";
            /** @description Subtype 7: Locally assigned (device-specific identifier) */
            value: string;
        };
        /** @description Login request from client */
        LoginRequest: {
            /** Format: email */
            email: string;
            password: string;
        };
        /** @enum {string} */
        MatchConfidence: "NotApplicable" | "Low" | "Medium" | "High" | "Certain";
        MatchDetails: {
            confidence: components["schemas"]["MatchConfidence"];
            reason: components["schemas"]["MatchReason"];
        };
        /** @description Match reason - either a simple reason string or a container with nested reasons */
        MatchReason: {
            data: string;
            /** @enum {string} */
            type: "reason";
        } | {
            /** @description Tuple of [name: string, children: MatchReason[]] */
            data: unknown[];
            /** @enum {string} */
            type: "container";
        };
        MetadataRegistry: {
            billing_plans: components["schemas"]["TypeMetadata"][];
            concepts: components["schemas"]["EntityMetadata"][];
            discovery_types: components["schemas"]["TypeMetadata"][];
            edge_types: components["schemas"]["TypeMetadata"][];
            entities: components["schemas"]["EntityMetadata"][];
            features: components["schemas"]["TypeMetadata"][];
            group_types: components["schemas"]["TypeMetadata"][];
            permissions: components["schemas"]["TypeMetadata"][];
            ports: components["schemas"]["TypeMetadata"][];
            service_definitions: components["schemas"]["TypeMetadata"][];
            subnet_types: components["schemas"]["TypeMetadata"][];
        };
        /**
         * @description Resolved LLDP/CDP neighbor connection.
         *
         *     Represents the remote endpoint this port connects to, discovered via LLDP or CDP.
         *     The two variants are mutually exclusive and represent different resolution states.
         */
        Neighbor: {
            /**
             * Format: uuid
             * @description Full resolution - the specific remote port was identified
             */
            id: string;
            /** @enum {string} */
            type: "IfEntry";
        } | {
            /**
             * Format: uuid
             * @description Partial resolution - the remote device was identified but not the specific port
             */
            id: string;
            /** @enum {string} */
            type: "Host";
        };
        /**
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "id": "550e8400-e29b-41d4-a716-446655440002",
         *       "name": "Home Network",
         *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z"
         *     }
         */
        Network: components["schemas"]["NetworkBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        NetworkBase: {
            name: string;
            /** Format: uuid */
            organization_id: string;
            /**
             * Format: uuid
             * @description Default SNMP credential for this network (hosts can override).
             *     When set, SNMP discovery is enabled for this network.
             */
            snmp_credential_id?: string | null;
            tags: string[];
        };
        /** @description Network configuration for setup */
        NetworkSetup: {
            name: string;
            /** @description SNMP community string (for V2c) */
            snmp_community?: string | null;
            /** @description Whether SNMP is enabled for this network */
            snmp_enabled?: boolean;
            /** @description SNMP version ("V2c" or "V3") */
            snmp_version?: string | null;
        };
        /** @description Per-network summary of entity counts */
        NetworkSummary: {
            /** Format: int64 */
            daemon_count: number;
            /** Format: int64 */
            host_count: number;
            /** Format: uuid */
            id: string;
            name: string;
            /** Format: int64 */
            service_count: number;
            /** Format: int64 */
            subnet_count: number;
        };
        Node: components["schemas"]["NodeType"] & {
            header?: string | null;
            /** Format: uuid */
            id: string;
            position: components["schemas"]["Ixy"];
            size: components["schemas"]["Uxy"];
        };
        NodeType: {
            infra_width: number;
            /** @enum {string} */
            node_type: "SubnetNode";
        } | {
            /** Format: uuid */
            host_id: string;
            /** Format: uuid */
            interface_id?: string | null;
            is_infra: boolean;
            /** @enum {string} */
            node_type: "InterfaceNode";
            /** Format: uuid */
            subnet_id: string;
        };
        OidcProviderMetadata: {
            logo?: string | null;
            name: string;
            slug: string;
        };
        /** @description Network data in onboarding state response */
        OnboardingNetworkState: {
            /**
             * Format: uuid
             * @description Network ID (if created)
             */
            id?: string | null;
            /** @description Network name */
            name: string;
            /** @description SNMP community string */
            snmp_community?: string | null;
            /** @description Whether SNMP is enabled */
            snmp_enabled?: boolean;
            /** @description SNMP version */
            snmp_version?: string | null;
        };
        /** @description Response from onboarding state endpoint */
        OnboardingStateResponse: {
            network?: null | components["schemas"]["OnboardingNetworkState"];
            /**
             * Format: uuid
             * @description Network ID from pending setup (if any)
             */
            network_id?: string | null;
            /** @description Organization name from pending setup */
            org_name?: string | null;
            /** @description Current onboarding step (if any) */
            step?: string | null;
            /** @description Use case selection (homelab, company, msp) */
            use_case?: string | null;
        };
        /** @description Request to save onboarding step */
        OnboardingStepRequest: {
            /** @description Company size */
            company_size?: string | null;
            /** @description Job title/role */
            job_title?: string | null;
            /** @description Referral source (how they heard about Scanopy) */
            referral_source?: string | null;
            /** @description Free-text referral source (when "other" is selected) */
            referral_source_other?: string | null;
            step: string;
            /** @description Use case selection (homelab, company, msp) */
            use_case?: string | null;
        };
        /**
         * @description Direction for ORDER BY clauses.
         * @enum {string}
         */
        OrderDirection: "asc" | "desc";
        Organization: components["schemas"]["OrganizationBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        OrganizationBase: {
            readonly has_payment_method?: boolean;
            name: string;
            onboarding: components["schemas"]["TelemetryOperation"][];
            plan: null | components["schemas"]["BillingPlan"];
            readonly plan_status: string | null;
            /** Format: date-time */
            readonly trial_end_date?: string | null;
        };
        /**
         * @description API metadata for paginated list responses (pagination is always present)
         * @example {
         *       "api_version": 1,
         *       "pagination": {
         *         "has_more": true,
         *         "limit": 50,
         *         "offset": 0,
         *         "total_count": 142
         *       },
         *       "server_version": "0.14.7"
         *     }
         */
        PaginatedApiMeta: {
            /**
             * Format: int32
             * @description API version (integer, increments on breaking changes)
             */
            api_version: number;
            /** @description Pagination info */
            pagination: components["schemas"]["PaginationMeta"];
            /**
             * @description Server version (semver)
             * @example 0.14.7
             */
            server_version: string;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_DaemonResponse: {
            data: (components["schemas"]["DaemonBase"] & {
                /** Format: date-time */
                created_at: string;
                /** Format: uuid */
                id: string;
                /** Format: date-time */
                updated_at: string;
                /** @description Computed version status including health and warnings */
                version_status: components["schemas"]["DaemonVersionStatus"];
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Group: {
            data: (components["schemas"]["GroupBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_HostResponse: {
            data: {
                chassis_id?: string | null;
                /** Format: date-time */
                created_at: string;
                description?: string | null;
                hidden: boolean;
                hostname?: string | null;
                /** Format: uuid */
                id: string;
                /** @description SNMP ifTable entries */
                if_entries: components["schemas"]["IfEntry"][];
                interfaces: components["schemas"]["Interface"][];
                management_url?: string | null;
                name: string;
                /** Format: uuid */
                network_id: string;
                ports: components["schemas"]["Port"][];
                services: components["schemas"]["Service"][];
                /** Format: uuid */
                snmp_credential_id?: string | null;
                source: components["schemas"]["EntitySource"];
                sys_contact?: string | null;
                sys_descr?: string | null;
                sys_location?: string | null;
                sys_object_id?: string | null;
                tags: string[];
                /** Format: date-time */
                updated_at: string;
                virtualization?: null | components["schemas"]["HostVirtualization"];
            }[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Service: {
            data: (components["schemas"]["ServiceBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_SnmpCredential: {
            data: (components["schemas"]["SnmpCredentialBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Subnet: {
            data: (components["schemas"]["SubnetBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Tag: {
            data: (components["schemas"]["TagBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Topology: {
            data: (components["schemas"]["TopologyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_User: {
            data: (components["schemas"]["UserBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_UserApiKey: {
            data: (components["schemas"]["UserApiKeyBase"] & {
                /** Format: date-time */
                readonly created_at: string;
                /** Format: uuid */
                readonly id: string;
                /** Format: date-time */
                readonly updated_at: string;
            })[];
            error?: string | null;
            meta: components["schemas"]["PaginatedApiMeta"];
            success: boolean;
        };
        /**
         * @description Pagination metadata returned with paginated responses.
         * @example {
         *       "has_more": true,
         *       "limit": 50,
         *       "offset": 0,
         *       "total_count": 142
         *     }
         */
        PaginationMeta: {
            /** @description Whether there are more items after this page */
            has_more: boolean;
            /**
             * Format: int32
             * @description Maximum items per page (as requested)
             */
            limit: number;
            /**
             * Format: int32
             * @description Number of items skipped
             */
            offset: number;
            /**
             * Format: int64
             * @description Total number of items matching the filter (ignoring pagination)
             */
            total_count: number;
        };
        /**
         * @description Pagination parameters that can be composed into filter queries.
         *
         *     Default behavior:
         *     - `limit`: 50 (returns up to 50 results)
         *     - `offset`: 0 (starts from the beginning)
         *     - `limit=0`: No limit (returns all results)
         *     - `limit` values above 1000 are capped to 1000
         */
        PaginationParams: {
            /**
             * Format: int32
             * @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit.
             */
            limit?: number | null;
            /**
             * Format: int32
             * @description Number of results to skip. Default: 0.
             */
            offset?: number | null;
        };
        PlanConfig: {
            /** Format: int64 */
            base_cents: number;
            /** Format: int64 */
            host_cents?: number | null;
            /** Format: int64 */
            included_hosts?: number | null;
            /** Format: int64 */
            included_networks?: number | null;
            /** Format: int64 */
            included_seats?: number | null;
            /** Format: int64 */
            network_cents?: number | null;
            rate: components["schemas"]["BillingRate"];
            /** Format: int64 */
            seat_cents?: number | null;
            /** Format: int32 */
            trial_days: number;
        };
        /** @description Plan usage limits and current counts */
        PlanUsage: {
            /** Format: int64 */
            host_count: number;
            /** Format: int64 */
            host_limit?: number | null;
            /** Format: int64 */
            network_count: number;
            /** Format: int64 */
            network_limit?: number | null;
            /** Format: int64 */
            seat_count: number;
            /** Format: int64 */
            seat_limit?: number | null;
        };
        /**
         * @description Port entity with custom serialization that flattens PortType fields.
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *       "id": "550e8400-e29b-41d4-a716-446655440006",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "number": 80,
         *       "protocol": "Tcp",
         *       "type": "Http",
         *       "updated_at": "2026-01-15T10:30:00Z"
         *     }
         */
        Port: components["schemas"]["PortBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        /** @description The base data for a Port entity (everything except id, created_at, updated_at) */
        PortBase: components["schemas"]["PortType"] & {
            /** Format: uuid */
            host_id: string;
            /** Format: uuid */
            network_id: string;
        };
        /**
         * @description Input for creating or updating a port.
         *     Used in both CreateHostRequest and UpdateHostRequest.
         *     Client must provide a UUID for the port.
         */
        PortInput: {
            /**
             * Format: uuid
             * @description Client-provided UUID for this port
             */
            id: string;
            /**
             * Format: int32
             * @description Port number (1-65535)
             */
            number: number;
            /** @description Transport protocol (Tcp or Udp) */
            protocol: components["schemas"]["TransportProtocol"];
        };
        /** @description Port type with number, protocol, and optional type identifier */
        PortType: {
            number: number;
            /** @enum {string} */
            protocol: "Udp" | "Tcp";
            /** @description Auto-derived from number+protocol; optional on create */
            type?: string;
        };
        /**
         * @description Request to pre-provision a ServerPoll mode daemon.
         *     This creates the daemon record on the server before the daemon is installed.
         */
        ProvisionDaemonRequest: {
            /** @description Human-readable name for the daemon. */
            name: string;
            /**
             * Format: uuid
             * @description Network this daemon will be associated with.
             */
            network_id: string;
            /** @description URL where the server can reach the daemon (required for ServerPoll mode). */
            url: string;
        };
        /**
         * @description Response from provisioning a daemon.
         *     Contains the daemon record and the API key (shown only once).
         */
        ProvisionDaemonResponse: {
            /** @description The created daemon record (with version status). */
            daemon: components["schemas"]["DaemonResponse"];
            /**
             * @description The API key (plaintext) for daemon authentication.
             *     This is shown only once - store it securely.
             */
            daemon_api_key: string;
        };
        ProxmoxVirtualization: {
            /** Format: uuid */
            service_id: string;
            vm_id?: string | null;
            vm_name?: string | null;
        };
        PublicConfigResponse: {
            billing_enabled: boolean;
            deployment_type: components["schemas"]["DeploymentType"];
            disable_registration: boolean;
            has_email_opt_in: boolean;
            has_email_service: boolean;
            has_integrated_daemon: boolean;
            needs_cookie_consent: boolean;
            oidc_providers: components["schemas"]["OidcProviderMetadata"][];
            posthog_key?: string | null;
            public_url: string;
            /** Format: int32 */
            server_port: number;
        };
        /** @description Public share metadata (returned without authentication) */
        PublicShareMetadata: {
            /** Format: uuid */
            id: string;
            name: string;
            options: components["schemas"]["ShareOptions"];
            requires_password: boolean;
        };
        /** @description Registration request from client */
        RegisterRequest: {
            /** Format: email */
            email: string;
            marketing_opt_in?: boolean;
            password: string;
            terms_accepted: boolean;
        };
        RequestEmailChangeRequest: {
            /**
             * @description Current password — required if the user already has a password set.
             *     Not required for OIDC-only users.
             */
            current_password?: string | null;
            /** Format: email */
            new_email: string;
        };
        /** @description Request to resend verification email */
        ResendVerificationRequest: {
            /** Format: email */
            email: string;
        };
        ResetPasswordRequest: {
            password: string;
            token: string;
        };
        RunType: {
            cron_schedule: string;
            enabled: boolean;
            /** Format: date-time */
            readonly last_run?: string | null;
            /** @enum {string} */
            type: "Scheduled";
        } | {
            results: components["schemas"]["DiscoveryUpdatePayload"];
            /** @enum {string} */
            type: "Historical";
        } | {
            /** Format: date-time */
            readonly last_run?: string | null;
            /** @enum {string} */
            type: "AdHoc";
        };
        /** @description Server capabilities returned on startup/registration */
        ServerCapabilities: {
            /** @description Deprecation warnings for the daemon */
            deprecation_warnings?: components["schemas"]["DeprecationWarning"][];
            /** @description Minimum daemon version supported by this server */
            minimum_daemon_version: string;
            /** @description Server software version */
            server_version: string;
        };
        /**
         * @example {
         *       "bindings": [
         *         {
         *           "created_at": "2026-02-23T22:33:25.648265Z",
         *           "id": "dfd72817-8cea-4693-bdae-df7be6bf4922",
         *           "interface_id": "550e8400-e29b-41d4-a716-446655440005",
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *           "service_id": "550e8400-e29b-41d4-a716-446655440007",
         *           "type": "Port",
         *           "updated_at": "2026-02-23T22:33:25.648265Z"
         *         }
         *       ],
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *       "id": "550e8400-e29b-41d4-a716-446655440007",
         *       "name": "nginx",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "position": 0,
         *       "service_definition": "Google Home",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "virtualization": null
         *     }
         */
        Service: components["schemas"]["ServiceBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        ServiceBase: {
            bindings: components["schemas"]["Binding"][];
            /** Format: uuid */
            host_id: string;
            name: string;
            /** Format: uuid */
            network_id: string;
            /**
             * Format: int32
             * @description Position of this service in the host's service list (for ordering)
             */
            position: number;
            service_definition: string;
            /** @description Will be automatically set to Manual for creation through API */
            source: components["schemas"]["EntitySource"];
            tags: string[];
            virtualization?: null | components["schemas"]["ServiceVirtualization"];
        };
        /** @enum {string} */
        ServiceCategory: "NetworkCore" | "NetworkAccess" | "NetworkSecurity" | "Storage" | "Backup" | "Media" | "HomeAutomation" | "Virtualization" | "DNS" | "VPN" | "SNMP" | "Monitoring" | "AdBlock" | "ReverseProxy" | "Workstation" | "Mobile" | "IoT" | "Printer" | "Database" | "Development" | "Dashboard" | "MessageQueue" | "IdentityAndAccess" | "Office" | "ProjectManagement" | "Messaging" | "Conferencing" | "Telephony" | "Email" | "Publishing" | "Unknown" | "Custom" | "Scanopy" | "OpenPorts";
        /**
         * @description Input for creating or updating a service.
         *     Used in both CreateHostRequest and UpdateHostRequest.
         *     Client must provide a UUID for the service.
         */
        ServiceInput: {
            /** @description Bindings that associate this service with ports/interfaces */
            bindings?: components["schemas"]["BindingInput"][];
            /**
             * Format: uuid
             * @description Client-provided UUID for this service
             */
            id: string;
            /** @description Display name for this service */
            name: string;
            /**
             * Format: int32
             * @description Position in the host's service list (for ordering).
             *     If omitted on create: appends to end of list.
             *     If omitted on update: existing services keep their positions; new services append.
             *     Must be all specified or all omitted across all services in the request.
             */
            position?: number | null;
            /** @description Service definition ID (e.g., "Nginx", "PostgreSQL") */
            service_definition: string;
            /** @description Tags for categorization */
            tags?: string[];
            virtualization?: null | components["schemas"]["ServiceVirtualization"];
        };
        /**
         * @description Fields that services can be ordered/grouped by.
         * @enum {string}
         */
        ServiceOrderField: "created_at" | "name" | "updated_at" | "host" | "network_id" | "position";
        /** ServiceVirtualization */
        ServiceVirtualization: {
            details: components["schemas"]["DockerVirtualization"];
            /** @enum {string} */
            type: "Docker";
        };
        /** @description Request body for setting all tags on an entity */
        SetTagsRequest: {
            /**
             * Format: uuid
             * @description The entity ID
             */
            entity_id: string;
            /** @description The entity type (e.g., Host, Service, Subnet) */
            entity_type: components["schemas"]["EntityDiscriminants"];
            /** @description The new list of tag IDs */
            tag_ids: string[];
        };
        SetupPaymentMethodRequest: {
            url: string;
        };
        /** @description Setup request for pre-registration org/network configuration */
        SetupRequest: {
            network: components["schemas"]["NetworkSetup"];
            organization_name: string;
        };
        /** @description Response from setup endpoint */
        SetupResponse: {
            /** Format: uuid */
            network_id: string;
        };
        Share: components["schemas"]["ShareBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        ShareBase: {
            allowed_domains: string[] | null;
            /** Format: uuid */
            created_by: string;
            /** Format: date-time */
            expires_at: string | null;
            is_enabled: boolean;
            name: string;
            /** Format: uuid */
            network_id: string;
            options: components["schemas"]["ShareOptions"];
            /** Format: uuid */
            topology_id: string;
        };
        /** @description Share display options */
        ShareOptions: {
            show_export_button: boolean;
            show_inspect_panel: boolean;
            show_zoom_controls: boolean;
        };
        SnmpCredential: components["schemas"]["SnmpCredentialBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        SnmpCredentialBase: {
            /**
             * @description SNMPv2c community string (stored encrypted)
             *     For V3, this would be extended with auth/priv credentials
             *     Redacted in API responses for security
             */
            community: string;
            name: string;
            /** Format: uuid */
            organization_id: string;
            tags: string[];
            /** @description SNMP version (V2c or V3) */
            version?: components["schemas"]["SnmpVersion"];
        };
        /**
         * @description SNMP credential mapping for network discovery
         *     Server builds this before initiating discovery; daemon uses it during scan
         */
        SnmpCredentialMapping: {
            default_credential?: null | components["schemas"]["SnmpQueryCredential"];
            /** @description Per-IP overrides (from host.snmp_credential_id where host has known IPs) */
            ip_overrides?: components["schemas"]["SnmpIpOverride"][];
        };
        /** @enum {string} */
        SnmpCredentialOrderField: "created_at" | "name" | "version" | "updated_at";
        /** @description IP-specific SNMP credential override */
        SnmpIpOverride: {
            /** @description Credential to use for this IP */
            credential: components["schemas"]["SnmpQueryCredential"];
            /** @description IP address for this override */
            ip: string;
        };
        /**
         * @description Minimal SNMP credential for daemon queries (version + community only)
         *     Does not include organization_id, name, timestamps - just what's needed for SNMP queries
         */
        SnmpQueryCredential: {
            /** @description SNMPv2c community string */
            community: string;
            /** @description SNMP version (V2c or V3) */
            version?: components["schemas"]["SnmpVersion"];
        };
        /**
         * @description SNMP protocol version
         * @enum {string}
         */
        SnmpVersion: "V2c" | "V3";
        /**
         * @example {
         *       "cidr": "192.168.1.0/24",
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "description": "Local area network",
         *       "id": "550e8400-e29b-41d4-a716-446655440004",
         *       "name": "LAN",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "subnet_type": "Lan",
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z"
         *     }
         */
        Subnet: components["schemas"]["SubnetBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        SubnetBase: {
            cidr: string;
            description?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            /** @description Will be automatically set to Manual for creation through API */
            source: components["schemas"]["EntitySource"];
            subnet_type: components["schemas"]["SubnetType"];
            tags: string[];
        };
        /**
         * @description Fields that subnets can be ordered/grouped by.
         * @enum {string}
         */
        SubnetOrderField: "created_at" | "name" | "cidr" | "subnet_type" | "updated_at" | "network_id";
        /** @enum {string} */
        SubnetType: "Internet" | "Remote" | "Gateway" | "VpnTunnel" | "Dmz" | "Lan" | "WiFi" | "IoT" | "Guest" | "DockerBridge" | "MacVlan" | "IpVlan" | "Management" | "Storage" | "Unknown" | "None";
        /**
         * @example {
         *       "color": "Green",
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "description": "Production environment resources",
         *       "id": "550e8400-e29b-41d4-a716-44665544000a",
         *       "name": "production",
         *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
         *       "updated_at": "2026-01-15T10:30:00Z"
         *     }
         */
        Tag: components["schemas"]["TagBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        TagBase: {
            color: components["schemas"]["Color"];
            description?: string | null;
            name: string;
            /** Format: uuid */
            organization_id: string;
        };
        /**
         * @description Fields that tags can be ordered/grouped by.
         * @enum {string}
         */
        TagOrderField: "created_at" | "name" | "color" | "updated_at";
        /** @enum {string} */
        TelemetryOperation: "OrgCreated" | "OnboardingModalCompleted" | "PlanSelected" | "PersonalPlanSelected" | "CommercialPlanSelected" | "CheckoutStarted" | "CheckoutCompleted" | "TrialStarted" | "TrialEnded" | "TrialWillEnd" | "SubscriptionCancelled" | "PlanChanged" | "PaymentFailed" | "PaymentActionRequired" | "PaymentRecovered" | "FirstDaemonRegistered" | "FirstTopologyRebuild" | "FirstDiscoveryCompleted" | "FirstHostDiscovered" | "SecondNetworkCreated" | "FirstTagCreated" | "FirstUserApiKeyCreated" | "FirstSnmpCredentialCreated" | "InviteSent" | "InviteAccepted" | "FirstApiKeyCreated" | "FirstNetworkCreated";
        Topology: components["schemas"]["TopologyBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        TopologyBase: {
            bindings: components["schemas"]["Binding"][];
            edges: components["schemas"]["Edge"][];
            entity_tags: components["schemas"]["Tag"][];
            groups: components["schemas"]["Group"][];
            hosts: components["schemas"]["Host"][];
            if_entries: components["schemas"]["IfEntry"][];
            interfaces: components["schemas"]["Interface"][];
            is_locked: boolean;
            is_stale: boolean;
            /** Format: date-time */
            last_refreshed: string;
            /** Format: date-time */
            locked_at?: string | null;
            /** Format: uuid */
            locked_by?: string | null;
            name: string;
            /** Format: uuid */
            network_id: string;
            nodes: components["schemas"]["Node"][];
            options: components["schemas"]["TopologyOptions"];
            /** Format: uuid */
            parent_id?: string | null;
            ports: components["schemas"]["Port"][];
            removed_bindings: string[];
            removed_groups: string[];
            removed_hosts: string[];
            removed_if_entries: string[];
            removed_interfaces: string[];
            removed_ports: string[];
            removed_services: string[];
            removed_subnets: string[];
            services: components["schemas"]["Service"][];
            subnets: components["schemas"]["Subnet"][];
            tags: string[];
        };
        /**
         * @description Lightweight request type for updating an edge's handles.
         *
         *     Used for edge reconnect operations - instead of sending the entire topology,
         *     only sends the edge ID and new handle positions.
         *     Fixes HTTP 413 errors on edge reconnect operations.
         */
        TopologyEdgeHandleUpdate: {
            /**
             * Format: uuid
             * @description ID of the edge to update
             */
            edge_id: string;
            /**
             * Format: uuid
             * @description Network ID for authorization
             */
            network_id: string;
            /** @description New source handle position */
            source_handle: components["schemas"]["EdgeHandle"];
            /** @description New target handle position */
            target_handle: components["schemas"]["EdgeHandle"];
        };
        TopologyLocalOptions: {
            hide_edge_types: components["schemas"]["EdgeTypeDiscriminants"][];
            hide_resize_handles: boolean;
            left_zone_title: string;
            no_fade_edges: boolean;
            tag_filter?: components["schemas"]["TopologyTagFilter"];
        };
        /**
         * @description Lightweight request type for updating topology metadata.
         *
         *     Used for editing topology name/parent - instead of sending the entire topology
         *     (which includes all hosts, interfaces, services, etc.), only sends the metadata fields.
         *     Fixes HTTP 413 errors on metadata edit operations.
         */
        TopologyMetadataUpdate: {
            /** @description New name for the topology */
            name: string;
            /**
             * Format: uuid
             * @description Network ID for authorization
             */
            network_id: string;
            /**
             * Format: uuid
             * @description New parent topology ID (optional)
             */
            parent_id?: string | null;
        };
        /**
         * @description Lightweight request type for updating a single node's position.
         *
         *     Used for drag operations - instead of sending the entire topology (which can be
         *     several megabytes for large networks), only sends the node ID and new position.
         *     Fixes HTTP 413 errors on drag operations.
         */
        TopologyNodePositionUpdate: {
            /**
             * Format: uuid
             * @description Network ID for authorization
             */
            network_id: string;
            /**
             * Format: uuid
             * @description ID of the node to update
             */
            node_id: string;
            /** @description New position for the node */
            position: components["schemas"]["Ixy"];
        };
        /**
         * @description Lightweight request type for updating a node's size and position.
         *
         *     Used for subnet resize operations - instead of sending the entire topology,
         *     only sends the node ID, new size, and new position.
         *     Fixes HTTP 413 errors on resize operations.
         */
        TopologyNodeResizeUpdate: {
            /**
             * Format: uuid
             * @description Network ID for authorization
             */
            network_id: string;
            /**
             * Format: uuid
             * @description ID of the node to update
             */
            node_id: string;
            /** @description New position for the node */
            position: components["schemas"]["Ixy"];
            /** @description New size for the node */
            size: components["schemas"]["Uxy"];
        };
        TopologyOptions: {
            local: components["schemas"]["TopologyLocalOptions"];
            request: components["schemas"]["TopologyRequestOptions"];
        };
        /**
         * @description Lightweight request type for topology rebuild/refresh operations.
         *
         *     This type only includes the fields actually needed by the server - entity data
         *     (hosts, interfaces, services, etc.) is fetched fresh from the database.
         *     Using this instead of the full Topology dramatically reduces payload size
         *     for large networks (from MBs to KBs), fixing HTTP 413 errors.
         */
        TopologyRebuildRequest: {
            /** @description Existing edges for reference during rebuild */
            edges?: components["schemas"]["Edge"][];
            /**
             * Format: uuid
             * @description Network ID for authorization and data fetching
             */
            network_id: string;
            /** @description Existing nodes for position preservation during rebuild */
            nodes?: components["schemas"]["Node"][];
            /** @description Topology options for graph building */
            options: components["schemas"]["TopologyOptions"];
        };
        TopologyRequestOptions: {
            group_docker_bridges_by_host: boolean;
            hide_ports: boolean;
            hide_service_categories: components["schemas"]["ServiceCategory"][];
            hide_vm_title_on_docker_container: boolean;
            left_zone_service_categories: components["schemas"]["ServiceCategory"][];
            show_gateway_in_left_zone: boolean;
        };
        /** @description Filter settings for hiding entities by tag in topology visualization. */
        TopologyTagFilter: {
            /** @description Host tag IDs to hide (hosts with these tags will fade out) */
            hidden_host_tag_ids?: string[];
            /** @description Service tag IDs to hide (services with these tags will be hidden from nodes) */
            hidden_service_tag_ids?: string[];
            /** @description Subnet tag IDs to hide (subnets with these tags will fade out) */
            hidden_subnet_tag_ids?: string[];
        };
        /** @enum {string} */
        TransportProtocol: "Udp" | "Tcp";
        TypeMetadata: {
            category: string | null;
            color: components["schemas"]["Color"];
            description: string | null;
            icon: string | null;
            id: string;
            metadata: unknown;
            name: string | null;
        };
        /**
         * @description Request type for updating a host with its children.
         *     Uses the same input types as CreateHostRequest.
         *     Server will sync children (create new, update existing, delete removed) only if provided.
         */
        UpdateHostRequest: {
            description?: string | null;
            /**
             * Format: date-time
             * @description Optional: expected updated_at timestamp for optimistic locking.
             */
            expected_updated_at?: string | null;
            hidden: boolean;
            hostname?: string | null;
            /** Format: uuid */
            id: string;
            /**
             * @description Interfaces to sync with this host.
             *     If Some, server will create/update/delete to match this list.
             *     If None, existing interfaces are preserved.
             */
            interfaces?: components["schemas"]["InterfaceInput"][] | null;
            name: string;
            /**
             * @description Ports to sync with this host.
             *     If Some, server will create/update/delete to match this list.
             *     If None, existing ports are preserved.
             */
            ports?: components["schemas"]["PortInput"][] | null;
            /**
             * @description Services to sync with this host.
             *     If Some, server will create/update/delete to match this list.
             *     If None, existing services are preserved.
             */
            services?: components["schemas"]["ServiceInput"][] | null;
            tags: string[];
            virtualization?: null | components["schemas"]["HostVirtualization"];
        };
        UpdatePasswordRequest: {
            /**
             * @description Current password — required if the user already has a password set.
             *     Not required for OIDC-only users adding their first password.
             */
            current_password?: string | null;
            /** @description New password to set */
            new_password: string;
        };
        User: components["schemas"]["UserBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        UserApiKey: components["schemas"]["UserApiKeyBase"] & {
            /** Format: date-time */
            readonly created_at: string;
            /** Format: uuid */
            readonly id: string;
            /** Format: date-time */
            readonly updated_at: string;
        };
        UserApiKeyBase: {
            /** Format: date-time */
            expires_at?: string | null;
            is_enabled?: boolean;
            readonly key: string;
            /** Format: date-time */
            readonly last_used: string | null;
            name: string;
            /** @description Network IDs this key has access to (hydrated from junction table) */
            network_ids?: string[];
            /** Format: uuid */
            organization_id: string;
            permissions?: components["schemas"]["UserOrgPermissions"];
            tags: string[];
            /** Format: uuid */
            user_id: string;
        };
        /**
         * @description Response for user API key creation/rotation
         *     Contains the full API key record plus the plaintext key (shown only once)
         */
        UserApiKeyResponse: {
            api_key: components["schemas"]["UserApiKey"];
            /** @description The plaintext API key - only returned once during creation or rotation */
            key: string;
        };
        UserBase: {
            email: string;
            /** @description Whether the user has verified their email address */
            email_verified?: boolean;
            /** @description Whether the user has a password set — computed from password_hash, never stored in DB */
            readonly has_password?: boolean;
            network_ids: string[];
            /** Format: date-time */
            oidc_linked_at?: string | null;
            oidc_provider?: string | null;
            /** Format: uuid */
            organization_id: string;
            permissions: components["schemas"]["UserOrgPermissions"];
            /** Format: date-time */
            readonly terms_accepted_at?: string | null;
        };
        /** @enum {string} */
        UserOrgPermissions: "Owner" | "Admin" | "Member" | "Viewer";
        Uxy: {
            x: number;
            y: number;
        };
        /** @description Request to verify email using token */
        VerifyEmailRequest: {
            token: string;
        };
        /**
         * @description Health status for daemon versions
         * @enum {string}
         */
        VersionHealthStatus: "Current" | "Outdated" | "Deprecated";
        /** @description Version information for API compatibility checking */
        VersionInfo: {
            /**
             * Format: int32
             * @description Current API version (integer, increments on breaking changes)
             */
            api_version: number;
            /** @description Minimum client version that can use this API (optional, for future use) */
            min_compatible_client?: string | null;
            /**
             * @description Server version (semver)
             * @example 0.12.10
             */
            server_version: string;
        };
    };
    responses: never;
    parameters: never;
    requestBodies: never;
    headers: never;
    pathItems: never;
}
export type $defs = Record<string, never>;
export interface operations {
    check_email: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CheckEmailRequest"];
            };
        };
        responses: {
            /** @description Email is available */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Email already in use */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    forgot_password: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ForgotPasswordRequest"];
            };
        };
        responses: {
            /** @description Password reset email sent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    login: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["LoginRequest"];
            };
        };
        responses: {
            /** @description Login successful */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Invalid credentials */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Login forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    logout: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Logout successful */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    get_current_user: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Current user */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Not authenticated */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    unlink_oidc_account: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description OIDC provider slug */
                slug: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description OIDC account unlinked */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Not authenticated */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Blocked in demo mode */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Provider not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    onboarding_state: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Onboarding state */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_OnboardingStateResponse"];
                };
            };
        };
    };
    onboarding_step: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["OnboardingStepRequest"];
            };
        };
        responses: {
            /** @description Step saved */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    register: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RegisterRequest"];
            };
        };
        responses: {
            /** @description User registered successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Registration disabled */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Email already exists */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    request_email_change: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["RequestEmailChangeRequest"];
            };
        };
        responses: {
            /** @description Verification email sent to new address */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Not authenticated */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    resend_verification: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ResendVerificationRequest"];
            };
        };
        responses: {
            /** @description Verification email sent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid request or already verified */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Rate limited */
            429: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    reset_password: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ResetPasswordRequest"];
            };
        };
        responses: {
            /** @description Password reset successful */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Invalid or expired token */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    setup: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetupRequest"];
            };
        };
        responses: {
            /** @description Setup data stored */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_SetupResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_password_auth: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdatePasswordRequest"];
            };
        };
        responses: {
            /** @description Password updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Not authenticated */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Blocked in demo mode */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    verify_email: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["VerifyEmailRequest"];
            };
        };
        responses: {
            /** @description Email verified successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Invalid or expired token */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    change_plan: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ChangePlanRequest"];
            };
        };
        responses: {
            /** @description Plan change initiated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Invalid plan or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    preview_plan_change: {
        parameters: {
            query: {
                /** @description Target plan (JSON) */
                plan: string;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Plan change preview */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_ChangePlanPreview"];
                };
            };
            /** @description Billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    create_checkout_session: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateCheckoutRequest"];
            };
        };
        responses: {
            /** @description Checkout session URL */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Invalid plan or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    submit_enterprise_inquiry: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["EnterpriseInquiryRequest"];
            };
        };
        responses: {
            /** @description Inquiry submitted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid request or Brevo not configured */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Authentication required */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_billing_plans: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of available billing plans */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vec_BillingPlan"];
                };
            };
            /** @description Billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    create_portal_session: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "text/plain": string;
            };
        };
        responses: {
            /** @description Portal session URL */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    setup_payment_method: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetupPaymentMethodRequest"];
            };
        };
        responses: {
            /** @description Setup session URL */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    handle_webhook: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Webhook processed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid signature or billing not enabled */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_public_config: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Public server configuration */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_PublicConfigResponse"];
                };
            };
        };
    };
    register_daemon: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonRegistrationRequest"];
            };
        };
        responses: {
            /** @description Daemon registered successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DaemonRegistrationResponse"];
                };
            };
            /** @description Daemon registration disabled in demo mode */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    receive_heartbeat: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonHeartbeatPayload"];
            };
        };
        responses: {
            /** @description Heartbeat received */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    receive_work_request: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonStatusPayload"];
            };
        };
        responses: {
            /** @description Work request processed - returns (Option<DiscoveryUpdatePayload>, bool) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content?: never;
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    daemon_startup: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonStartupRequest"];
            };
        };
        responses: {
            /** @description Startup acknowledged */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_ServerCapabilities"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_capabilities: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonCapabilities"];
            };
        };
        responses: {
            /** @description Capabilities updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_stars: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description GitHub star count */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_u32"];
                };
            };
        };
    };
    get_metadata_registry: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Metadata registry */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_MetadataRegistry"];
                };
            };
        };
    };
    list_daemon_api_keys: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Daemon API Keys */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["DaemonApiKey"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_daemon_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonApiKey"];
            };
        };
        responses: {
            /** @description Daemon API key created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DaemonApiKeyResponse"];
                };
            };
            /** @description Bad request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Insufficient permissions (member+ required) */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Internal server error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_daemon_api_keys: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of daemon_api_key IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description daemon_api_keys deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description One or more API keys are in use by daemons */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_daemon_api_keys_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Daemon API Keys */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_daemon_api_key_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon API Key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Daemon API Key found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DaemonApiKey"];
                };
            };
            /** @description Daemon API Key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_daemon_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonApiKey"];
            };
        };
        responses: {
            /** @description Daemon API key updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DaemonApiKey"];
                };
            };
            /** @description Daemon API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_daemon_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description daemon_api_key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description daemon_api_key deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description daemon_api_key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description API key is in use by a daemon */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    rotate_key_handler: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Daemon API key rotated, returns new key */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Daemon API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_user_api_keys: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of user API keys */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_UserApiKey"];
                };
            };
            /** @description Not authenticated */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Internal server error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    create_user_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UserApiKey"];
            };
        };
        responses: {
            /** @description API key created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_UserApiKeyResponse"];
                };
            };
            /** @description Bad request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Invalid permissions or network access */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Internal server error */
            500: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_user_api_keys: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of API key IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description API keys deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_user_api_keys_csv: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing User API Keys */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_user_api_key_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description API key found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_UserApiKey"];
                };
            };
            /** @description API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_user_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UserApiKey"];
            };
        };
        responses: {
            /** @description API key updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_UserApiKey"];
                };
            };
            /** @description Not authorized to update this key */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_user_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description API key deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    rotate_user_api_key: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description API key ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description API key rotated, returns new key */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Not authorized to rotate this key */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description API key not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_bindings: {
        parameters: {
            query?: {
                /** @description Filter by service ID */
                service_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by port ID */
                port_id?: string | null;
                /** @description Filter by interface ID */
                interface_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Bindings */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Binding"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_binding: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Binding"];
            };
        };
        responses: {
            /** @description Binding created (superseded bindings may be removed) */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Binding"];
                };
            };
            /** @description Referenced port or interface does not exist */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Conflict with existing binding type */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_bindings: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Binding IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Bindings deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_bindings_csv: {
        parameters: {
            query?: {
                /** @description Filter by service ID */
                service_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by port ID */
                port_id?: string | null;
                /** @description Filter by interface ID */
                interface_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Bindings */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_binding_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Binding ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Binding found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Binding"];
                };
            };
            /** @description Binding not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_binding: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Binding ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Binding"];
            };
        };
        responses: {
            /** @description Binding updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Binding"];
                };
            };
            /** @description Referenced port or interface does not exist */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Conflict with existing binding type */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_binding: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Binding ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Binding deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Binding not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_daemons: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["DaemonOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["DaemonOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of daemons */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_DaemonResponse"];
                };
            };
        };
    };
    bulk_delete_daemons: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Daemon IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Daemons deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_daemons_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["DaemonOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["DaemonOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Daemons */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    provision_daemon: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ProvisionDaemonRequest"];
            };
        };
        responses: {
            /** @description Daemon provisioned successfully */
            201: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_ProvisionDaemonResponse"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Forbidden */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_daemon_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Daemon found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DaemonResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_daemon: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Daemon deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    retry_daemon_connection: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Connection retry initiated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_dashboard_summary: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Dashboard summary */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DashboardSummary"];
                };
            };
        };
    };
    list_discoveries: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by daemon ID */
                daemon_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Discoveries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Discovery"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_discovery: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Discovery"];
            };
        };
        responses: {
            /** @description Discovery created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Discovery"];
                };
            };
            /** @description Can't create historical discovery */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_active_sessions: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of active discovery sessions */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vec_DiscoveryUpdatePayload"];
                };
            };
        };
    };
    bulk_delete_discoveries: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Discovery IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Discoveries deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_discoveries_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by daemon ID */
                daemon_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Discoveries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    start_session: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "text/plain": string;
            };
        };
        responses: {
            /** @description Discovery session started */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DiscoveryUpdatePayload"];
                };
            };
            /** @description Discovery not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_discovery_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Discovery ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Discovery found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Discovery"];
                };
            };
            /** @description Discovery not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_discovery: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Discovery ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Discovery"];
            };
        };
        responses: {
            /** @description Discovery updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Discovery"];
                };
            };
            /** @description Can't update historical discovery */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_discovery: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Discovery ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Discovery deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Discovery not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    cancel_discovery: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Session ID */
                session_id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Discovery session cancelled */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    receive_discovery_update: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Discovery session ID */
                session_id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DiscoveryUpdatePayload"];
            };
        };
        responses: {
            /** @description Update received */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
        };
    };
    get_all_groups: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["GroupOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["GroupOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of groups */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Group"];
                };
            };
        };
    };
    create_group: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Group"];
            };
        };
        responses: {
            /** @description Group created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Group"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_groups: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Group IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Groups deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_groups_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["GroupOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["GroupOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Groups */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_group_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Group ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Group found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Group"];
                };
            };
            /** @description Group not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_group: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Group ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Group"];
            };
        };
        responses: {
            /** @description Group updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Group"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Group not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_group: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Group ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Group deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Group not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_hosts: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns hosts that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["HostOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["HostOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of hosts with their children */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_HostResponse"];
                };
            };
        };
    };
    create_host: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateHostRequest"];
            };
        };
        responses: {
            /** @description Host created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_HostResponse"];
                };
            };
            /** @description Validation error: network not found, subnet mismatch, or invalid tags */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description No access to the specified network */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_hosts: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of host IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Hosts deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description One or more hosts has an associated daemon - delete daemons first */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    create_host_discovery: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DiscoveryHostRequest"];
            };
        };
        responses: {
            /** @description Host discovered/updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_HostResponse"];
                };
            };
            /** @description Daemon cannot create hosts on other networks */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_hosts_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns hosts that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["HostOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["HostOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Hosts */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    export_hosts_zip: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns hosts that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["HostOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["HostOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ZIP file containing CSVs */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/zip": unknown;
                };
            };
        };
    };
    consolidate_hosts: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Destination host ID - will receive all children */
                destination_host: string;
                /** @description Host to merge into destination - will be deleted */
                other_host: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Hosts consolidated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_HostResponse"];
                };
            };
            /** @description Validation error: same host, has daemon, or different networks */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description One or both hosts not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_host_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Host ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Host found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_HostResponse"];
                };
            };
            /** @description Host not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_host: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Host ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["UpdateHostRequest"];
            };
        };
        responses: {
            /** @description Host updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_HostResponse"];
                };
            };
            /** @description Validation error: invalid tags */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Host not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_host: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Host ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Host deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Host not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Host has associated daemon */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_iftable_entries: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of ifTable Entries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["IfEntry"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_if_entry: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["IfEntry"];
            };
        };
        responses: {
            /** @description If entry created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_IfEntry"];
                };
            };
            /** @description Network mismatch or duplicate if_index */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_iftable_entries: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of ifTable Entry IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description ifTable Entries deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_iftable_entries_csv: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing ifTable Entries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_iftable_entry_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ifTable Entry ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ifTable Entry found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_IfEntry"];
                };
            };
            /** @description ifTable Entry not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_if_entry: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description If entry ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["IfEntry"];
            };
        };
        responses: {
            /** @description If entry updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_IfEntry"];
                };
            };
            /** @description Network mismatch or invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description If entry not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_iftable_entry: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description ifTable Entry ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description ifTable Entry deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description ifTable Entry not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_interfaces: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by subnet ID */
                subnet_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Interfaces */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Interface"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_interface: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Interface"];
            };
        };
        responses: {
            /** @description Interface created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Interface"];
                };
            };
            /** @description Network mismatch or invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_interfaces: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Interfaces deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description No IDs provided */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_interfaces_csv: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by subnet ID */
                subnet_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Interfaces */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_interface_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Interface ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Interface found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Interface"];
                };
            };
            /** @description Interface not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_interface: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Interface ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Interface"];
            };
        };
        responses: {
            /** @description Interface updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Interface"];
                };
            };
            /** @description Network mismatch or invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Interface not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_interface: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Interface ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Interface deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Interface not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_invites: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of active invites */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vec_Invite"];
                };
            };
        };
    };
    create_invite: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateInviteRequest"];
            };
        };
        responses: {
            /** @description Invite created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Invite"];
                };
            };
            /** @description Cannot create invite with higher permissions */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_invite: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Invite ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Invite details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Invite"];
                };
            };
            /** @description Invalid or expired invite */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    revoke_invite: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Invite ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Invite revoked */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid invite */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Cannot revoke this invite */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_networks: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Networks */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Network"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_network: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Network"];
            };
        };
        responses: {
            /** @description Network created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Network"];
                };
            };
        };
    };
    bulk_delete_networks: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of network IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Networks deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description User not admin */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_networks_csv: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Networks */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_network_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Network ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Network found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Network"];
                };
            };
            /** @description Network not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_network: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Network ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Network"];
            };
        };
        responses: {
            /** @description Network updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Network"];
                };
            };
            /** @description User not admin */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Network not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_network: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Network ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Network deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description User not admin */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Network not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_organization: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Organization details */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Organization"];
                };
            };
            /** @description Organization not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_org_name: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "text/plain": string;
            };
        };
        responses: {
            /** @description Organization updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Organization"];
                };
            };
            /** @description Only owners can update organization */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Organization not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    populate_demo_data: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Demo data populated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Only available for demo organizations */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Organization not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    reset: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Organization ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Organization reset */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Cannot reset another organization */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Organization not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_ports: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Ports */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Port"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_port: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Port"];
            };
        };
        responses: {
            /** @description Port created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Port"];
                };
            };
            /** @description Network mismatch or duplicate port */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_ports: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Port IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Ports deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_ports_csv: {
        parameters: {
            query?: {
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Ports */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_port_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Port ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Port found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Port"];
                };
            };
            /** @description Port not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_port: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Port ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Port"];
            };
        };
        responses: {
            /** @description Port updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Port"];
                };
            };
            /** @description Network mismatch or invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Port not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_port: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Port ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Port deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Port not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_services: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns services that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of services */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Service"];
                };
            };
        };
    };
    create_service: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateServiceRequest"];
            };
        };
        responses: {
            /** @description Service created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Service"];
                };
            };
            /** @description Validation error: host network mismatch, cross-host binding, or binding conflict */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_services: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Service IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Services deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_services_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by host ID */
                host_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns services that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Services */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_service_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Service ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Service found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Service"];
                };
            };
            /** @description Service not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_service: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Service ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Service"];
            };
        };
        responses: {
            /** @description Service updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Service"];
                };
            };
            /** @description Validation error: host network mismatch, cross-host binding, or binding conflict */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Service not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_service: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Service ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Service deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Service not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_shares: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by topology ID */
                topology_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of Shares */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        data: components["schemas"]["Share"][];
                        error?: string | null;
                        meta: components["schemas"]["PaginatedApiMeta"];
                        success: boolean;
                    };
                };
            };
        };
    };
    create_share: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateUpdateShareRequest"];
            };
        };
        responses: {
            /** @description Share created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Share"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_shares: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Share IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Shares deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_shares_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by topology ID */
                topology_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Shares */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_public_share_metadata: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Share ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Share metadata */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_PublicShareMetadata"];
                };
            };
            /** @description Share not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    verify_share_password: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Share ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "text/plain": string;
            };
        };
        responses: {
            /** @description Password verified */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_bool"];
                };
            };
            /** @description Invalid password */
            401: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Share not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_share_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Share ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Share found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Share"];
                };
            };
            /** @description Share not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_share: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Share ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateUpdateShareRequest"];
            };
        };
        responses: {
            /** @description Share updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Share"];
                };
            };
            /** @description Share not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_share: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Share ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Share deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Share not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_snmp_credentials: {
        parameters: {
            query?: {
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["SnmpCredentialOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["SnmpCredentialOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of SNMP credentials */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_SnmpCredential"];
                };
            };
        };
    };
    create_snmp_credential: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SnmpCredential"];
            };
        };
        responses: {
            /** @description SNMP credential created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_SnmpCredential"];
                };
            };
            /** @description Validation error */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Credential name already exists in this organization */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_snmp_credentials: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description SNMP Credentials deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description Validation error */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_snmp_credentials_csv: {
        parameters: {
            query?: {
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["SnmpCredentialOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["SnmpCredentialOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing SNMP Credentials */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_snmp_credential_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description SNMP Credential ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description SNMP Credential found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_SnmpCredential"];
                };
            };
            /** @description SNMP Credential not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_snmp_credential: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description snmp_credential ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SnmpCredential"];
            };
        };
        responses: {
            /** @description snmp_credential updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_SnmpCredential"];
                };
            };
            /** @description Validation error */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description snmp_credential not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_snmp_credential: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description snmp_credential ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description snmp_credential deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description snmp_credential not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    list_subnets: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["SubnetOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["SubnetOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of subnets */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Subnet"];
                };
            };
        };
    };
    create_subnet: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Subnet"];
            };
        };
        responses: {
            /** @description Subnet created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Subnet"];
                };
            };
            /** @description Invalid request */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_subnets: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Subnet IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Subnets deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_subnets_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["SubnetOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["SubnetOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Subnets */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_subnet_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Subnet ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Subnet found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Subnet"];
                };
            };
            /** @description Subnet not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_subnet: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Subnet ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Subnet"];
            };
        };
        responses: {
            /** @description Subnet updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Subnet"];
                };
            };
            /** @description CIDR change would orphan existing interfaces */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Subnet not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_subnet: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Subnet ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Subnet deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Subnet not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_tags: {
        parameters: {
            query?: {
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["TagOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["TagOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of tags */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Tag"];
                };
            };
        };
    };
    create_tag: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Tag"];
            };
        };
        responses: {
            /** @description Tag created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Tag"];
                };
            };
            /** @description Validation error: name empty or too long */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Tag name already exists in this organization */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    set_entity_tags: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["SetTagsRequest"];
            };
        };
        responses: {
            /** @description Tags set successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Invalid entity type or tag */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Tag not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_add_tag: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BulkTagRequest"];
            };
        };
        responses: {
            /** @description Tag added successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkTagResponse"];
                };
            };
            /** @description Invalid entity type or tag */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Tag not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_remove_tag: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["BulkTagRequest"];
            };
        };
        responses: {
            /** @description Tag removed successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkTagResponse"];
                };
            };
            /** @description Invalid entity type */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    bulk_delete_tags: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Tag IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Tags deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
        };
    };
    export_tags_csv: {
        parameters: {
            query?: {
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["TagOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["TagOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Tags */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_tag_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tag ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Tag found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Tag"];
                };
            };
            /** @description Tag not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_tag: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tag ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Tag"];
            };
        };
        responses: {
            /** @description Tag updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Tag"];
                };
            };
            /** @description Tag not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_tag: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Tag ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Tag deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Tag not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_topologies: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of topologies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Topology"];
                };
            };
        };
    };
    create_topology: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Topology"];
            };
        };
        responses: {
            /** @description Topology created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Topology"];
                };
            };
            /** @description Validation failed */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_topologies_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Topologies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_topology_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Topology found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Topology"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_topology: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Topology"];
            };
        };
        responses: {
            /** @description Topology updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Topology"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_topology: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Topology deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_edge_handles: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TopologyEdgeHandleUpdate"];
            };
        };
        responses: {
            /** @description Edge handles updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Topology or edge not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    lock: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Topology locked */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Topology"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_metadata: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TopologyMetadataUpdate"];
            };
        };
        responses: {
            /** @description Metadata updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_node_position: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TopologyNodePositionUpdate"];
            };
        };
        responses: {
            /** @description Node position updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Topology or node not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_node_resize: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TopologyNodeResizeUpdate"];
            };
        };
        responses: {
            /** @description Node resized */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Topology or node not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    rebuild: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TopologyRebuildRequest"];
            };
        };
        responses: {
            /** @description Topology rebuilt */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    refresh: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TopologyRebuildRequest"];
            };
        };
        responses: {
            /** @description Topology refreshed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    unlock: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Topology unlocked */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Topology"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Topology not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_all_users: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of users */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_User"];
                };
            };
        };
    };
    bulk_delete_users: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of user IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Users deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description Cannot delete users with higher permissions */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    export_users_csv: {
        parameters: {
            query?: {
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Users */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": unknown;
                };
            };
        };
    };
    get_user_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description User ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description User found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Access denied */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description User not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    update_user: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description User ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["User"];
            };
        };
        responses: {
            /** @description User updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Cannot update another user's record */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description User not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    delete_user: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description User ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description User deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Cannot delete user with higher permissions */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description User not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Cannot delete the only owner */
            409: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    admin_update_user: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description User ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["User"];
            };
        };
        responses: {
            /** @description User updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_User"];
                };
            };
            /** @description Cannot update user with higher permissions */
            403: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description User not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
        };
    };
    get_version: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Version information */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_VersionInfo"];
                };
            };
        };
    };
}
