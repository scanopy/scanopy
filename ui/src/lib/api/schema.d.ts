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
    "/api/billing/cancel": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Cancel subscription
         * @description In-app cancel modal endpoint. Sets Stripe `cancel_at` to the current
         *     period end (via Stripe's `MaxPeriodEnd` sentinel), stashes the canonical
         *     Scanopy reason in subscription metadata, returns the period end so the
         *     modal can render the retention disclosure.
         */
        post: operations["cancel_subscription"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/cancel/apply-discount": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Apply the discount save offer
         * @description Applies the configured Stripe coupon to the subscription. Returns 400
         *     when `STRIPE_SAVE_OFFER_COUPON_ID` is unset.
         */
        post: operations["apply_discount_save_offer"];
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
    "/api/billing/extend-trial": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Self-serve trial extend (+7 days, once per org lifetime) */
        post: operations["extend_trial"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/finalize-payment-method": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Finalize a client-confirmed SetupIntent (set the card as default) */
        post: operations["finalize_payment_method"];
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
    "/api/billing/pause": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Pause subscription billing
         * @description Pauses billing for a 30/60/90 day window. Eligibility: rolling 6-month
         *     cooldown anchored on the org's `last_paused_at`.
         */
        post: operations["pause_subscription"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/payment-method-setup-intent": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Create a SetupIntent for in-app card collection (Stripe Payment Element) */
        post: operations["create_payment_method_setup_intent"];
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
    "/api/billing/reactivate": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Reactivate a subscription pending cancellation
         * @description Clears Stripe's scheduled-cancellation state (`cancel_at` → None).
         *     Available while `plan_status === 'pending_cancellation'`.
         */
        post: operations["reactivate_subscription"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/resume": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Resume a paused subscription
         * @description Clears Stripe pause collection and re-activates billing. Available while
         *     `plan_status === 'paused'`. The prorated pause credit is posted to the
         *     customer's Stripe balance asynchronously by the webhook arm that fires
         *     for the `pause_collection` clear — the endpoint just returns success.
         */
        post: operations["resume_subscription"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/billing/save-offer-coupon": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Read live terms for the configured save-offer coupon
         * @description Returns the coupon's `percent_off` and `duration_in_months` so the
         *     cancel modal's Discount panel can render the offer dynamically. The
         *     payload is `null` when `STRIPE_SAVE_OFFER_COUPON_ID` is unset — the
         *     modal hides the panel in that case.
         */
        get: operations["get_save_offer_coupon"];
        put?: never;
        post?: never;
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
         * @description Legacy internal endpoint for pre-0.15 daemons to report their interfaced
         *     subnets as bare ids. Modern daemons report them via the status heartbeat's
         *     `interfaced_subnets` channel; this remains functional so older daemons in a
         *     rolling deploy keep reporting (and don't 404).
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
         *     - **Port binding (specific ip_address)**: Service listens on a specific port on a specific interface.
         *     - **Port binding (all ip_addresses)**: Service listens on a specific port on all ip_addresses
         *       (`ip_address_id: null`).
         *
         *     ### Validation and Deduplication Rules
         *
         *     - **Conflict detection**: Interface bindings conflict with port bindings on the same interface.
         *       A port binding on all ip_addresses conflicts with any interface binding for the same service.
         *     - **All-interfaces precedence**: When creating a port binding with `ip_address_id: null`,
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
    "/api/v1/credentials": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all Credentials
         * @description Returns all credentials in the authenticated user's organization.
         *     Optionally filter by type (e.g. `?type=SnmpV2c`).
         */
        get: operations["get_all_credentials"];
        put?: never;
        /**
         * Create a new Credential
         * @description Creates a credential scoped to your organization.
         */
        post: operations["create_credential"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/credentials/bulk": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk create Credentials
         * @description Creates multiple credentials in one request. Validation is atomic — if any
         *     credential has an invalid type, none are created. Individual creates are
         *     sequential, so a mid-batch DB error leaves earlier credentials committed.
         */
        post: operations["bulk_create_credentials"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/credentials/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Credentials */
        post: operations["bulk_delete_credentials"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/credentials/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Credentials to CSV
         * @description Export all Credentials matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_credentials_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/credentials/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get a Credential by ID */
        get: operations["get_by_id_credential"];
        /** Update Credential */
        put: operations["update_credential"];
        post?: never;
        /** Delete Credential */
        delete: operations["delete_credential"];
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
        /** Bulk delete daemons */
        post: operations["bulk_delete_daemons"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/email-install-command": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Email install command to current user
         * @description Session-only, and `IsUser` says so at the extractor rather than in the body. "The current user"
         *     has no answer for an automation identity: a user API key carries `user_id` but no address, so
         *     this endpoint could never serve one. An API key that wants the command reads it directly from
         *     `GET /api/v1/daemons/{id}/install-command`.
         */
        post: operations["email_install_command"];
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
         * Provision a daemon, or re-provision an existing one
         * @description Creates a daemon record on the server before the daemon is installed and mints an API key
         *     bound to it 1:1. Returns the daemon record and that key, which is shown only once and must
         *     be configured on the daemon.
         *
         *     When `daemon_id` is supplied the existing record is reused instead of creating a new one,
         *     giving a legacy daemon (one with no bound key) a pathway to a dedicated key without losing
         *     its host, discovery jobs, or history. Re-provisioning always mints a fresh key.
         *
         *     Install commands are not built here — fetch them from the install-command endpoint, which
         *     builds them idempotently and fills in the key this returns. That keeps a display-only
         *     regenerate (an OS switch, an advanced-setting change) from re-minting the key.
         */
        post: operations["provision_daemon"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/test-reachability": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Test reachability of a daemon URL
         * @description Performs a TCP connection test and optionally an HTTP health check
         *     to verify that a daemon URL is reachable from the server.
         */
        post: operations["test_daemon_reachability"];
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
        /**
         * Update daemon
         * @description Edits the server-side daemon record: its name, maintainer, tags, and — for ServerPoll —
         *     the url the server dials. Identity and server-managed fields (network, mode, host, key
         *     binding, version, liveness) are restored from the existing record by
         *     `preserve_immutable_fields`.
         */
        put: operations["update_daemon"];
        post?: never;
        /** Delete daemon */
        delete: operations["delete_daemon"];
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/daemons/{id}/install-command": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Generate daemon install command
         * @description A pure, idempotent builder — it never mints or persists anything. The api key in an `install`
         *     command is a placeholder (`<API_KEY>`) the caller substitutes from the plaintext it holds; a
         *     `reconfigure` command carries no key at all. Minting is a separate mutation
         *     (`POST /provision`), so regenerating a command here (advanced-setting change, OS switch, the
         *     Details reconfigure view) never rotates the daemon's key.
         *
         *     The server derives the exact command shape from the record: DaemonPoll vs ServerPoll for the
         *     flags, and — for `install` — whether the daemon has checked in (`last_seen`) to decide between
         *     a first-install and a minimal re-key command.
         */
        get: operations["get_daemon_install_command"];
        put?: never;
        post?: never;
        delete?: never;
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
    "/api/v1/dependencies": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all Dependencies
         * @description Returns all dependencies the authenticated user has access to.
         *     Supports pagination via `limit` and `offset` query parameters,
         *     and ordering via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["get_all_dependencies"];
        put?: never;
        /** Create a new Dependency */
        post: operations["create_dependency"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/dependencies/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Dependencies */
        post: operations["bulk_delete_dependencies"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/dependencies/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Dependencies to CSV
         * @description Export all Dependencies matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_dependencies_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/dependencies/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Dependency by ID */
        get: operations["get_dependency_by_id"];
        /** Update a Dependency */
        put: operations["update_dependency"];
        post?: never;
        /** Delete Dependency */
        delete: operations["delete_dependency"];
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
        /**
         * List discoveries
         * @description Returns discoveries the authenticated user has access to. The run history
         *     grows without bound, so this is paginated and ordered server-side rather
         *     than filtered in the browser.
         */
        get: operations["get_all_discoveries"];
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
        /** Bulk delete discoveries */
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
        /** Delete discovery */
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
         *     ip_addresses, ports, services and interfaces included — pass
         *     `include_children=false` to omit those and get a much smaller payload.
         *     Supports pagination via `limit` and `offset` query parameters, and ordering
         *     via `group_by`, `order_by`, and `order_direction`.
         */
        get: operations["get_all_hosts"];
        put?: never;
        /**
         * Create a new host
         * @description Creates a host with optional ip_addresses, ports, and services.
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
         *     (ip_addresses, ports, services, interfaces) as a ZIP archive containing
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
         * @description Merges all ip_addresses, ports, and services from `other_host` into
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
         * @description Returns a single host with its ip_addresses, ports, and services.
         */
        get: operations["get_host_by_id"];
        /**
         * Update a host
         * @description Updates host properties. Children (ip_addresses, ports, services)
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
    "/api/v1/hosts/{id}/rescan": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Rescan a host
         * @description Starts a one-shot scan of this host's addresses and nothing else, answering
         *     "is this host still there, and is its data current?" without sweeping the
         *     whole subnet.
         *
         *     The scan runs on the daemon that last discovered this host — evidence it can
         *     reach the address — and only if that daemon still has an interface on a
         *     subnet containing one of the host's scannable IPs. Where that interface has a
         *     MAC the daemon ARPs the target, which sees a live host even when every port
         *     is firewalled; on a MAC-less interface (a point-to-point tunnel) it falls
         *     back to a TCP probe. When no interface covers any of the host's addresses the
         *     request is refused with the specific reason. A loopback address is not a
         *     scannable IP — it is reached locally and is excluded from the target set.
         *
         *     Returns the session, which streams progress over `/api/v1/discovery/stream`
         *     like any other scan. A `Queued` phase means the daemon is busy; it will start
         *     when the running scan finishes.
         */
        post: operations["rescan_host"];
        delete?: never;
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
        /** List all Interfaces */
        get: operations["list_interfaces"];
        put?: never;
        /**
         * Create a new Interface
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
        /** Bulk delete Interfaces */
        post: operations["bulk_delete_interfaces"];
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
    "/api/v1/if-entries/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Interface by ID */
        get: operations["get_interface_by_id"];
        /** Update an Interface */
        put: operations["update_if_entry"];
        post?: never;
        /** Delete Interface */
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
    "/api/v1/ip-addresses": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all IP Addresses */
        get: operations["list_ip_addresses"];
        put?: never;
        /**
         * Create a new IP address
         *     Position is automatically assigned to the end of the host's IP address list.
         */
        post: operations["create_ip_address"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ip-addresses/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk delete IP addresses
         *     Remaining IP addresses for affected hosts are renumbered to maintain sequential positions.
         */
        post: operations["bulk_delete_ip_addresses"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ip-addresses/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export IP Addresses to CSV
         * @description Export all IP Addresses matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_ip_addresses_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/ip-addresses/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get IP Address by ID */
        get: operations["get_ip_address_by_id"];
        /**
         * Update an IP address
         *     Position must be within valid range and not conflict with other IP addresses.
         */
        put: operations["update_ip_address"];
        post?: never;
        /**
         * Delete an IP address
         *     Remaining IP addresses for the host are renumbered to maintain sequential positions.
         */
        delete: operations["delete_ip_address"];
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
        /** List all networks */
        get: operations["get_all_networks"];
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
        /** Get a network by ID */
        get: operations["get_by_id_network"];
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
    "/api/v1/organizations/daemon-prompt-response": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Record the user's response to the daemon-install prompt so it is not shown again.
         *     Each CTA persists a distinct onboarding milestone (the org subscriber dedups); the
         *     PostHog subscriber turns these into funnel events, so no client-side telemetry is needed.
         */
        post: operations["daemon_prompt_response"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/profile": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Update user profile with deferred marketing fields */
        post: operations["update_profile"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/referral-source": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Submit referral source (how did you hear about us) */
        post: operations["submit_referral_source"];
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
        /** Delete the organization entirely, including all data and users */
        delete: operations["delete_organization"];
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
        /**
         * Populate demo data (only available for demo organizations).
         * @description Runs the population off the request thread (a `tokio::spawn`) and returns
         *     `202` immediately — the work is a few hundred sequential DB round-trips and
         *     would otherwise exceed the reverse-proxy request timeout against a remote
         *     database. Poll `GET /{id}/populate-demo/status` for completion/failure.
         */
        post: operations["populate_demo_data"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/organizations/{id}/populate-demo/status": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Poll the status of an org's background demo-populate task. */
        get: operations["populate_demo_status"];
        put?: never;
        post?: never;
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
         * @description Creates a service with optional bindings to ip_addresses or ports.
         *     The `id`, `created_at`, `updated_at`, and `source` fields are generated server-side.
         *     Bindings are specified without `service_id` or `network_id` - these are assigned automatically.
         *
         *     ### Binding Validation Rules
         *
         *     - **Cross-host validation**: All bindings must reference ports/interfaces that belong to the
         *       service's host. Bindings referencing entities from other hosts will be rejected.
         *     - **Deduplication**: Duplicate bindings in the same request are automatically deduplicated.
         *     - **All-interfaces precedence**: If a port binding with `ip_address_id: null` (all ip_addresses)
         *       is included, any specific-interface bindings for the same port are automatically removed.
         *     - **Conflict detection**: Interface bindings conflict with port bindings on the same interface.
         *       A port binding on all ip_addresses conflicts with any interface binding.
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
         *     - **All-interfaces precedence**: If a port binding with `ip_address_id: null` (all ip_addresses)
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
        /**
         * Verify password for a password-protected share and return an access token.
         * @description The returned token is an HS256 JWT tied to the share's current password
         *     hash; subsequent `/topology` calls send the token instead of the raw
         *     password. Changing the share password invalidates outstanding tokens.
         */
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
    "/api/v1/snapshots": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** List all Snapshots */
        get: operations["list_snapshots"];
        put?: never;
        /**
         * Take a snapshot of the current live topology + entity state for a network.
         *     Acquires the discovery snapshot lock, creates the snapshots row, runs
         *     close-and-clone to stamp every Snapshotable entity row with `snapshot_id`
         *     and close them. The topology subscriber inserts the snapshot's topology
         *     row off the back of the `Snapshot::Created` event.
         */
        post: operations["create_snapshot"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/snapshots/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Snapshot by ID */
        get: operations["get_snapshot_by_id"];
        put?: never;
        post?: never;
        /** Delete Snapshot */
        delete: operations["delete_snapshot"];
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
         *     all existing ip_addresses on this subnet have IPs within the new CIDR range.
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
        /**
         * Get all topologies for the authenticated user's networks.
         * @description Returns both live-view rows (`snapshot_id IS NULL`) and snapshot-pinned
         *     rows. The frontend renders the live one by default and renders snapshot
         *     rows when the user picks one from the snapshots dropdown.
         */
        get: operations["get_all_topologies"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/data": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Unified entity-set endpoint for the topology view.
         * @description `?snapshot_id=<id>` resolves to the snapshot's `taken_at` and returns the
         *     as-of-T entity set; otherwise returns live entities. The frontend
         *     `TopologyTab` is the sole intended consumer.
         */
        get: operations["get_topology_data"];
        put?: never;
        post?: never;
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
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/export/confluence": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Export topology as Confluence wiki markup */
        get: operations["export_confluence"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/topology/{id}/export/mermaid": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Export topology as Mermaid flowchart */
        get: operations["export_mermaid"];
        put?: never;
        post?: never;
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
    "/api/v1/vlans": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * List all VLANs
         * @description Returns VLANs accessible to the authenticated user, optionally filtered by network.
         */
        get: operations["get_all_vlans"];
        put?: never;
        /**
         * Create a new VLAN
         * @description Creates a VLAN scoped to a network. VLAN numbers must be unique within a network.
         */
        post: operations["create_vlan"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/vlans/bulk-delete": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /** Bulk delete Vlans */
        post: operations["bulk_delete_vlans"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/vlans/discovery": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        get?: never;
        put?: never;
        /**
         * Bulk upsert VLANs from discovery
         * @description Used by daemons to report discovered VLANs. Creates new VLANs or updates names.
         *     Returns the mapping of VLAN numbers to entity UUIDs for Interface construction.
         */
        post: operations["discovery_upsert_vlans"];
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/vlans/export/csv": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /**
         * Export Vlans to CSV
         * @description Export all Vlans matching the filter criteria to CSV format. Ignores pagination parameters (limit/offset) and exports all matching records.
         */
        get: operations["export_vlans_csv"];
        put?: never;
        post?: never;
        delete?: never;
        options?: never;
        head?: never;
        patch?: never;
        trace?: never;
    };
    "/api/v1/vlans/{id}": {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** Get Vlan by ID */
        get: operations["get_vlan_by_id"];
        /** Update Vlan */
        put: operations["update_vlan"];
        post?: never;
        /** Delete Vlan */
        delete: operations["delete_vlan"];
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
            /** @description Human-readable failure message. */
            error?: string | null;
            /** @description API metadata (version info) */
            meta: components["schemas"]["ApiMeta"];
            /** @description Parameters for interpolating into the translated error message */
            params?: {
                [key: string]: unknown;
            } | null;
            /** @description Always `false` on this response shape. */
            success: boolean;
        };
        /**
         * @description API metadata included in all responses
         * @example {
         *       "api_version": 1,
         *       "server_version": "0.17.12"
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
             * @example 0.17.12
             */
            server_version: string;
        };
        ApiResponse: {
            /** @description The result payload. Omitted on failure. */
            data?: null | components["schemas"]["TupleUnit"];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Binding: {
            /**
             * @description Association between a service and a port / interface that the service is listening on
             * @example {
             *       "created_at": "2026-08-25T22:05:35.421764Z",
             *       "first_discovery_id": null,
             *       "id": "2ea2fc46-ee8c-4ba9-92dd-772e83f51503",
             *       "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
             *       "last_discovery_id": null,
             *       "last_seen_at": "2026-08-25T22:05:35.421764Z",
             *       "lineage_id": null,
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "port_id": "550e8400-e29b-41d4-a716-446655440006",
             *       "service_id": "550e8400-e29b-41d4-a716-446655440007",
             *       "type": "Port",
             *       "updated_at": "2026-08-25T22:05:35.421764Z",
             *       "valid_from": "2026-08-25T22:05:35.421764Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["BindingBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description The discovery that first observed this entity.
                 */
                readonly first_discovery_id?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description The most recent discovery that observed this entity.
                 */
                readonly last_discovery_id?: string | null;
                /**
                 * Format: date-time
                 * @description When a discovery last observed this entity.
                 */
                readonly last_seen_at?: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_BulkDeleteResponse: {
            /** @description The result payload. Omitted on failure. */
            data?: {
                /** @description How many records were actually deleted. */
                deleted_count: number;
                /** @description How many IDs the request asked to delete. */
                requested_count: number;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_BulkTagResponse: {
            /** @description Response for bulk tag operations */
            data?: {
                /** @description Number of entities affected */
                affected_count: number;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_CancelSubscriptionResponse: {
            /** @description The result payload. Omitted on failure. */
            data?: {
                /**
                 * Format: date-time
                 * @description When the current paid period ends and access drops to the free tier.
                 */
                period_end: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_ChangePlanPreview: {
            /** @description The result payload. Omitted on failure. */
            data?: {
                /**
                 * Format: int64
                 * @description Hosts over the target plan's allowance, which would be billed as overage.
                 */
                excess_hosts: number;
                /**
                 * Format: int64
                 * @description Networks over the target plan's allowance.
                 */
                excess_networks: number;
                /**
                 * Format: int64
                 * @description Seats over the target plan's allowance.
                 */
                excess_seats: number;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Credential: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["CredentialBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Daemon: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["DaemonBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_DaemonApiKey: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["DaemonApiKeyBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_DaemonApiKeyResponse: {
            /** @description The result payload. Omitted on failure. */
            data?: {
                /** @description The stored key record. */
                api_key: components["schemas"]["DaemonApiKey"];
                /**
                 * Format: password
                 * @description The plaintext API key - only returned once during creation or rotation.
                 */
                readonly key: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_DaemonRegistrationResponse: {
            /** @description Daemon registration response from server to daemon */
            data?: {
                /** @description The registered daemon record. */
                daemon: components["schemas"]["Daemon"];
                /**
                 * Format: uuid
                 * @description The host this entity belongs to.
                 */
                host_id: string;
                server_capabilities?: null | components["schemas"]["ServerCapabilities"];
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_DaemonResponse: {
            /** @description Daemon response for UI including computed version status */
            data?: components["schemas"]["DaemonBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                id: string;
                /**
                 * @description Subnets this daemon has interfaces on, loaded from the
                 *     `daemon_interfaced_subnets` junction (replaces the old
                 *     `capabilities.interfaced_subnet_ids` JSONB field).
                 */
                interfaced_subnet_ids: string[];
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                updated_at: string;
                /** @description Computed version status including health and warnings */
                version_status: components["schemas"]["DaemonVersionStatus"];
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_DashboardSummary: {
            /** @description Dashboard summary response */
            data?: {
                /** @description Daemons the caller can see, with their current status. */
                daemons: components["schemas"]["DaemonResponse"][];
                /** @description Per-network counts for every network the caller can see. */
                networks: components["schemas"]["NetworkSummary"][];
                /** @description Current usage against the organization's plan allowances. */
                plan_usage: components["schemas"]["PlanUsage"];
                /** @description The most recent discovery runs, newest first. */
                recent_discoveries: components["schemas"]["Discovery"][];
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_DemoPopulateStatus: {
            /**
             * @description Lifecycle of a demo-populate task. `Running` is set synchronously in the
             *     POST handler (before the `202`), then flipped to a terminal variant by the
             *     spawned task. `Failed` carries the error string so the UI can show why.
             */
            data?: {
                /**
                 * Format: date-time
                 * @description When population began.
                 */
                started_at: string;
                /** @enum {string} */
                state: "running";
            } | {
                /**
                 * Format: date-time
                 * @description When population finished.
                 */
                finished_at: string;
                /** @enum {string} */
                state: "complete";
            } | {
                /** @description Why population failed. */
                error: string;
                /**
                 * Format: date-time
                 * @description When it gave up.
                 */
                finished_at: string;
                /** @enum {string} */
                state: "failed";
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Dependency: {
            /**
             * @description The result payload. Omitted on failure.
             * @example {
             *       "color": "Blue",
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "dependency_type": "RequestPath",
             *       "description": "HTTP/HTTPS services dependency",
             *       "edge_style": "Bezier",
             *       "id": "550e8400-e29b-41d4-a716-446655440008",
             *       "lineage_id": null,
             *       "members": {
             *         "service_ids": [],
             *         "type": "Services"
             *       },
             *       "name": "Web Services",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["DependencyBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Discovery: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["DiscoveryBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /** @description When true, the next scan will be a full port scan regardless of interval */
                force_full_scan?: boolean;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * @description Per-daemon integration targeting: which integrations run on this daemon, and on which
                 *     IPs. Delivered via the init command at registration and editable via the discovery
                 *     modal. This is the single home for cred↔IP targeting; it replaces the global
                 *     `credential.target_ips` (race-prone, consumed once).
                 *
                 *     One-shot: a target is offered to the daemon until a scan completes successfully, then
                 *     dropped by [`Discovery::apply_successful_scan`]. Credentials that earned a durable home
                 *     during the scan keep being retried from there — `host_credentials` for one that probed
                 *     successfully, `network_credentials` for a broadcast one (see
                 *     [`Discovery::take_network_scope_credential_ids`]).
                 */
                integration_targets: components["schemas"]["IntegrationTarget"][];
                /**
                 * Format: int32
                 * @description Number of completed scans (incremented by server on session completion)
                 */
                readonly scan_count?: number;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_DiscoveryUpdatePayload: {
            /** @description Progress update from daemon to server during discovery */
            data?: {
                /**
                 * Format: uuid
                 * @description The daemon this entity refers to.
                 */
                daemon_id: string;
                /**
                 * Format: uuid
                 * @description The discovery configuration this session belongs to.
                 *     Always enriched server-side; daemons do not send this field.
                 */
                discovery_id?: string | null;
                /** @description What kind of discovery is running. */
                discovery_type: components["schemas"]["DiscoveryType"];
                /** @description Failure message, when the run did not complete. */
                error?: string | null;
                /**
                 * Format: int32
                 * @description Rough estimate of the time left, in seconds.
                 */
                estimated_remaining_secs?: number | null;
                /**
                 * Format: date-time
                 * @description When the run finished. `null` while it is still going.
                 */
                finished_at?: string | null;
                /**
                 * Format: int32
                 * @description Hosts found so far.
                 */
                hosts_discovered?: number | null;
                /**
                 * Format: uuid
                 * @description The network this entity belongs to.
                 */
                network_id: string;
                /** @description Which stage of the run is in progress. */
                phase: components["schemas"]["DiscoveryPhase"];
                /**
                 * Format: int32
                 * @description Completion of the current phase, from 0 to 1.
                 */
                progress: number;
                scanned?: null | components["schemas"]["ScannedEntityIds"];
                /**
                 * Format: uuid
                 * @description The discovery run this update belongs to.
                 */
                session_id: string;
                /**
                 * Format: date-time
                 * @description When the run started.
                 */
                started_at?: string | null;
                /**
                 * @description Non-fatal findings from a completed run — one per occurrence, each carrying the code that
                 *     identifies it and the detail that fills the sentence. Unlike `error`, these do not mark the
                 *     run failed.
                 *
                 *     Read through [`deserialize_warnings`] rather than the derived impl, which is what keeps
                 *     historical records and pre-coded daemons rendering: both send bare strings here, and both
                 *     land as `Unknown` carrying that text instead of failing the whole payload.
                 */
                warnings?: components["schemas"]["DiscoveryWarning"][];
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_HostResponse: {
            /**
             * @description Response type for host endpoints.
             *     Includes children (ip_addresses, ports, services, interfaces).
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "credential_assignments": [],
             *       "description": "Primary web server",
             *       "hidden": false,
             *       "hostname": "web-server-01.local",
             *       "id": "550e8400-e29b-41d4-a716-446655440003",
             *       "interfaces": [
             *         {
             *           "admin_status": "Up",
             *           "cdp_address": null,
             *           "cdp_device_id": null,
             *           "cdp_platform": null,
             *           "cdp_port_id": null,
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "first_discovery_id": null,
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-44665544000f",
             *           "if_alias": "Uplink to Core Switch",
             *           "if_descr": "GigabitEthernet0/1",
             *           "if_index": 1,
             *           "if_name": "Gi0/1",
             *           "if_type": 6,
             *           "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
             *           "last_discovery_id": null,
             *           "last_seen_at": "2026-01-15T10:30:00Z",
             *           "lineage_id": null,
             *           "lldp_chassis_id": null,
             *           "lldp_mgmt_addr": null,
             *           "lldp_port_desc": null,
             *           "lldp_port_id": null,
             *           "lldp_sys_desc": null,
             *           "lldp_sys_name": null,
             *           "mac_address": "DE:AD:BE:EF:CA:FE",
             *           "neighbor": null,
             *           "neighbor_seen_at": null,
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "oper_status": "Up",
             *           "speed_bps": 1000000000,
             *           "updated_at": "2026-01-15T10:30:00Z",
             *           "valid_from": "2026-01-15T10:30:00Z",
             *           "valid_to": null
             *         }
             *       ],
             *       "ip_addresses": [
             *         {
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "first_discovery_id": null,
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-446655440005",
             *           "ip_address": "192.168.1.100",
             *           "last_discovery_id": null,
             *           "last_seen_at": "2026-01-15T10:30:00Z",
             *           "lineage_id": null,
             *           "mac_address": "DE:AD:BE:EF:CA:FE",
             *           "name": "eth0",
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "position": 0,
             *           "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
             *           "updated_at": "2026-01-15T10:30:00Z",
             *           "valid_from": "2026-01-15T10:30:00Z",
             *           "valid_to": null
             *         }
             *       ],
             *       "last_seen_at": "2026-01-15T10:30:00Z",
             *       "name": "web-server-01",
             *       "name_source": "Manual",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "ports": [
             *         {
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "first_discovery_id": null,
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-446655440006",
             *           "last_discovery_id": null,
             *           "last_seen_at": "2026-01-15T10:30:00Z",
             *           "lineage_id": null,
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "number": 80,
             *           "protocol": "Tcp",
             *           "type": "Http",
             *           "updated_at": "2026-01-15T10:30:00Z",
             *           "valid_from": "2026-01-15T10:30:00Z",
             *           "valid_to": null
             *         }
             *       ],
             *       "services": [
             *         {
             *           "bindings": [
             *             {
             *               "created_at": "2026-08-25T22:05:35.390833Z",
             *               "first_discovery_id": null,
             *               "id": "bbc11a48-25c6-4596-b1ef-080c726c584f",
             *               "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
             *               "last_discovery_id": null,
             *               "last_seen_at": "2026-08-25T22:05:35.390833Z",
             *               "lineage_id": null,
             *               "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *               "port_id": "550e8400-e29b-41d4-a716-446655440006",
             *               "service_id": "550e8400-e29b-41d4-a716-446655440007",
             *               "type": "Port",
             *               "updated_at": "2026-08-25T22:05:35.390833Z",
             *               "valid_from": "2026-08-25T22:05:35.390833Z",
             *               "valid_to": null
             *             }
             *           ],
             *           "created_at": "2026-01-15T10:30:00Z",
             *           "first_discovery_id": null,
             *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *           "id": "550e8400-e29b-41d4-a716-446655440007",
             *           "last_discovery_id": null,
             *           "last_seen_at": "2026-01-15T10:30:00Z",
             *           "lineage_id": null,
             *           "name": "nginx",
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "position": 0,
             *           "service_definition": "Google Home",
             *           "source": {
             *             "type": "Manual"
             *           },
             *           "tags": [],
             *           "updated_at": "2026-01-15T10:30:00Z",
             *           "valid_from": "2026-01-15T10:30:00Z",
             *           "valid_to": null,
             *           "virtualization_metadata": null,
             *           "virtualization_service_id": null
             *         }
             *       ],
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "virtualization_metadata": null,
             *       "virtualization_service_id": null
             *     }
             */
            data?: {
                /** @description LLDP chassis identifier, used to match the host to its neighbours. */
                chassis_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                created_at: string;
                /** @description Credentials assigned to scan this host. */
                credential_assignments?: components["schemas"]["CredentialAssignment"][];
                /** @description Free-text notes about the host. */
                description?: string | null;
                /** @description Whether the host is hidden from topology views. */
                hidden: boolean;
                /** @description Hostname as resolved or reported by the host. */
                hostname?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                id: string;
                /** @description SNMP ifTable entries */
                interfaces: components["schemas"]["Interface"][];
                /** @description IP addresses on this host. */
                ip_addresses: components["schemas"]["IPAddress"][];
                /**
                 * Format: date-time
                 * @description Last time discovery observed this host. User-facing (drives the "Last
                 *     seen" column and the stale badge), which is why it is carried here while
                 *     the rest of the SCD2/audit columns are not.
                 */
                last_seen_at: string;
                /** @description Link to the host's own management interface. */
                management_url?: string | null;
                /** @description ENTITY-MIB entPhysicalMfgName — hardware manufacturer. Read-only, as above. */
                readonly manufacturer?: string | null;
                /** @description ENTITY-MIB entPhysicalModelName — hardware model. Read-only, as above. */
                readonly model?: string | null;
                /** @description Human-facing name for the host. */
                name: string;
                /**
                 * @description Which rung of the naming ladder produced `name`. Read-only: it is decided by whoever
                 *     supplied the name, not by the caller.
                 */
                name_source?: components["schemas"]["HostNameSource"];
                /**
                 * Format: uuid
                 * @description The network this entity belongs to.
                 */
                network_id: string;
                /** @description Open ports on this host. */
                ports: components["schemas"]["Port"][];
                /** @description ENTITY-MIB entPhysicalSerialNum — hardware serial number. Read-only, as above. */
                readonly serial_number?: string | null;
                /** @description Services running on this host. */
                services: components["schemas"]["Service"][];
                /** @description How this host came to be known — discovered, imported, or created by hand. */
                source: components["schemas"]["EntitySource"];
                /** @description SNMP sysContact — administrative contact as configured on the device. */
                sys_contact?: string | null;
                /** @description SNMP sysDescr — the device's own description of itself. */
                sys_descr?: string | null;
                /** @description SNMP sysLocation — physical location as configured on the device. */
                sys_location?: string | null;
                /**
                 * @description SNMP sysName.0 — the administratively-assigned hostname. Read-only: discovery collects it
                 *     from the device, so neither create nor update accepts it.
                 */
                readonly sys_name?: string | null;
                /** @description SNMP sysObjectID — the vendor's identifier for the device model. */
                sys_object_id?: string | null;
                /** @description Tags assigned to this entity. */
                tags: string[];
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                updated_at: string;
                virtualization_metadata?: null | components["schemas"]["HostVirtualization"];
                /**
                 * Format: uuid
                 * @description The hypervisor service this VM runs on.
                 */
                virtualization_service_id?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_IPAddress: {
            /**
             * @description The result payload. Omitted on failure.
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "first_discovery_id": null,
             *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *       "id": "550e8400-e29b-41d4-a716-446655440005",
             *       "ip_address": "192.168.1.100",
             *       "last_discovery_id": null,
             *       "last_seen_at": "2026-01-15T10:30:00Z",
             *       "lineage_id": null,
             *       "mac_address": "DE:AD:BE:EF:CA:FE",
             *       "name": "eth0",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "position": 0,
             *       "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["IPAddressBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description The discovery that first observed this entity.
                 */
                readonly first_discovery_id?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description The most recent discovery that observed this entity.
                 */
                readonly last_discovery_id?: string | null;
                /**
                 * Format: date-time
                 * @description When a discovery last observed this entity.
                 */
                readonly last_seen_at?: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_InstallArtifacts: {
            /**
             * @description Everything the UI needs to install (or reconfigure) a daemon, one field per install method so
             *     each is a first-class peer with its own content — no method is a special case bolted onto a
             *     list. The binary methods are ready-to-paste commands (any api key is the [`API_KEY_PLACEHOLDER`],
             *     filled in client-side); docker and msi carry their own structured content.
             */
            data?: {
                /** @description Container image reference. */
                docker: components["schemas"]["DockerInstall"];
                /** @description Download for FreeBSD. */
                freebsd: string;
                /** @description Download for Linux. */
                linux: string;
                /** @description Download for macOS. */
                macos: string;
                /** @description Windows installer package. */
                msi: components["schemas"]["MsiInstall"];
                /** @description Download for Windows. */
                windows: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Interface: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["InterfaceBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description The discovery that first observed this entity.
                 */
                readonly first_discovery_id?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description The most recent discovery that observed this entity.
                 */
                readonly last_discovery_id?: string | null;
                /**
                 * Format: date-time
                 * @description When a discovery last observed this entity.
                 */
                readonly last_seen_at?: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Invite: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["InviteBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Network: {
            /**
             * @description The result payload. Omitted on failure.
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "credential_ids": [],
             *       "effective_stale_after_hours": 672,
             *       "id": "550e8400-e29b-41d4-a716-446655440002",
             *       "name": "Home Network",
             *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
             *       "stale_after_hours": null,
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z"
             *     }
             */
            data?: components["schemas"]["NetworkBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: int64
                 * @description `stale_after_hours` with the server's default already applied.
                 *
                 *     Computed, never stored (excluded from `to_params`). Published so the
                 *     frontend derives staleness from the *same* number the digest uses rather
                 *     than re-declaring the default in TypeScript, where the two could drift
                 *     and a host could read stale in the app but current in the digest email.
                 */
                readonly effective_stale_after_hours?: number;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
                use_case?: null | components["schemas"]["UseCase"];
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Option_SaveOfferCoupon: {
            /** @description The result payload. Omitted on failure. */
            data?: null | {
                /** @description Billing interval the discount applies to. */
                billing_rate: components["schemas"]["BillingRate"];
                /**
                 * Format: int64
                 * @description How many months the discount lasts.
                 */
                duration_in_months: number;
                /**
                 * Format: date-time
                 * @description When the discounted subscription next renews.
                 */
                next_renewal_at: string;
                /**
                 * Format: int64
                 * @description Discount applied by the retention offer.
                 */
                percent_off: number;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Organization: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["OrganizationBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Port: {
            /**
             * @description Port entity with custom serialization that flattens PortType fields.
             * @example {
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "first_discovery_id": null,
             *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *       "id": "550e8400-e29b-41d4-a716-446655440006",
             *       "last_discovery_id": null,
             *       "last_seen_at": "2026-01-15T10:30:00Z",
             *       "lineage_id": null,
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "number": 80,
             *       "protocol": "Tcp",
             *       "type": "Http",
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["PortBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description The discovery that first observed this entity.
                 */
                readonly first_discovery_id?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description The most recent discovery that observed this entity.
                 */
                readonly last_discovery_id?: string | null;
                /**
                 * Format: date-time
                 * @description When a discovery last observed this entity.
                 */
                readonly last_seen_at?: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_ProvisionDaemonResponse: {
            /**
             * @description Response from provisioning a daemon.
             *     Contains the daemon record and the API key (shown only once).
             *
             *     Install commands are deliberately not here — fetch them from the install-command endpoint,
             *     which builds them idempotently and fills in this key. That keeps a display-only regenerate
             *     (advanced-setting change, OS switch) from re-minting the key.
             */
            data?: {
                /** @description The created daemon record (with version status). */
                daemon: components["schemas"]["DaemonResponse"];
                /**
                 * Format: password
                 * @description The API key (plaintext) for daemon authentication.
                 *     This is shown only once - store it securely.
                 */
                readonly daemon_api_key: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_PublicConfigResponse: {
            /** @description The result payload. Omitted on failure. */
            data?: {
                /** @description Whether this deployment has billing configured. */
                billing_enabled: boolean;
                /** @description How this instance is run: cloud, commercial self-hosted, or community. */
                deployment_type: components["schemas"]["DeploymentType"];
                /** @description Whether email/password login is turned off, leaving OIDC as the only method. */
                disable_password_login: boolean;
                /** @description Whether self-service sign-up is turned off on this deployment. */
                disable_registration: boolean;
                /**
                 * @description `STRIPE_SAVE_OFFER_COUPON_ID` env var is set. When false, the
                 *     cancel modal hides the discount save-offer panel so the user
                 *     doesn't see an option the deployment can't fulfil.
                 */
                discount_save_offer_available: boolean;
                /** @description Whether the deployment asks users to opt in to product email. */
                has_email_opt_in: boolean;
                /** @description Whether outbound email is configured. Invites and password resets need it. */
                has_email_service: boolean;
                /** @description Whether a daemon runs alongside the server, so no separate install is needed to start scanning. */
                has_integrated_daemon: boolean;
                /**
                 * Format: date
                 * @description Hard expiry — the drop-dead date after which the server rejects
                 *     the key. Referenced by the grace-period banner.
                 */
                license_expiry?: string | null;
                /**
                 * @description True when the license is past `intended_exp` but not yet past
                 *     the hard `exp` — the silent grace window.
                 */
                license_in_grace_period: boolean;
                /**
                 * Format: date
                 * @description User-visible expiry — the date displayed to end users under
                 *     normal operation. 7 days earlier than `license_expiry` for keys
                 *     issued after grace-period support landed.
                 */
                license_intended_expiry?: string | null;
                license_status?: null | components["schemas"]["LicenseStatusDiscriminants"];
                /** @description Whether the client should show a cookie-consent prompt. */
                needs_cookie_consent: boolean;
                /** @description Identity providers available on the login screen. */
                oidc_providers: components["schemas"]["OidcProviderMetadata"][];
                /**
                 * @description True when this self-hosted instance has reached its licensed
                 *     organization cap (`included_orgs`), so new-org registration is blocked.
                 *     Always false on cloud (multi-tenant) and on unlimited-org plans.
                 */
                org_limit_reached: boolean;
                /** @description Public analytics key, when analytics is enabled. */
                posthog_key?: string | null;
                /**
                 * Format: uri
                 * @description Base URL this server is reachable at, as configured by the operator.
                 */
                public_url: string;
                /**
                 * Format: email
                 * @description Admin contact email to show users blocked by `org_limit_reached`,
                 *     from `SCANOPY_SERVER_ADMIN_CONTACT_EMAIL`.
                 */
                server_admin_contact_email: string;
                /**
                 * Format: int32
                 * @description Port this server listens on.
                 */
                server_port: number;
                /**
                 * Format: int32
                 * @description `SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE` if set on this instance.
                 *     Frontend uses it inside the plan-comparison view to display the
                 *     effective retention for this deployment rather than the per-plan
                 *     fixture default.
                 */
                snapshot_retention_days_override?: number | null;
                /**
                 * @description Stripe publishable key, exposed so the frontend can mount Stripe
                 *     Elements (Payment Element) for in-app card collection. `None` when
                 *     billing isn't configured. Publishable keys are safe to expose to the
                 *     browser (same as `posthog_key`).
                 */
                stripe_publishable_key?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_PublicShareMetadata: {
            /** @description Public share metadata (returned without authentication) */
            data?: {
                /**
                 * @description Resolved list of available topology views for this share.
                 *     Filtered by both share configuration and data availability.
                 *     First element is the default view.
                 */
                enabled_views: components["schemas"]["TopologyView"][];
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                id: string;
                /** @description Human-facing name for this share. */
                name: string;
                /** @description What the viewer can see and do. */
                options: components["schemas"]["ShareOptions"];
                /** @description Whether a password must be supplied before the topology is returned. */
                requires_password: boolean;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Service: {
            /**
             * @description The result payload. Omitted on failure.
             * @example {
             *       "bindings": [
             *         {
             *           "created_at": "2026-08-25T22:05:35.414502Z",
             *           "first_discovery_id": null,
             *           "id": "b3c4c4d2-bfdd-4870-97ba-48714bcbd50b",
             *           "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
             *           "last_discovery_id": null,
             *           "last_seen_at": "2026-08-25T22:05:35.414502Z",
             *           "lineage_id": null,
             *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *           "port_id": "550e8400-e29b-41d4-a716-446655440006",
             *           "service_id": "550e8400-e29b-41d4-a716-446655440007",
             *           "type": "Port",
             *           "updated_at": "2026-08-25T22:05:35.414502Z",
             *           "valid_from": "2026-08-25T22:05:35.414502Z",
             *           "valid_to": null
             *         }
             *       ],
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "first_discovery_id": null,
             *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
             *       "id": "550e8400-e29b-41d4-a716-446655440007",
             *       "last_discovery_id": null,
             *       "last_seen_at": "2026-01-15T10:30:00Z",
             *       "lineage_id": null,
             *       "name": "nginx",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "position": 0,
             *       "service_definition": "Google Home",
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null,
             *       "virtualization_metadata": null,
             *       "virtualization_service_id": null
             *     }
             */
            data?: components["schemas"]["ServiceBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description The discovery that first observed this entity.
                 */
                readonly first_discovery_id?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description The most recent discovery that observed this entity.
                 */
                readonly last_discovery_id?: string | null;
                /**
                 * Format: date-time
                 * @description When a discovery last observed this entity.
                 */
                readonly last_seen_at?: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_SetupIntentResponse: {
            /**
             * @description Response for creating a SetupIntent — the client secret the frontend
             *     Payment Element uses to collect and confirm a card in-app.
             */
            data?: {
                /** @description Stripe SetupIntent client secret, used to mount the Payment Element. */
                client_secret: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_SetupResponse: {
            /** @description Response from setup endpoint */
            data?: {
                /**
                 * Format: uuid
                 * @description The network this entity belongs to.
                 */
                network_id: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Share: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["ShareBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_ShareAccessTokenResponse: {
            /**
             * @description Access token returned after successful password verification.
             *
             *     The token is an HS256 JWT tied to the share's `password_hash` — changing
             *     the share password implicitly invalidates all outstanding tokens.
             */
            data?: {
                /** @description Bearer token granting access to this share for the rest of the session. */
                access_token: string;
                /**
                 * Format: date-time
                 * @description When this record stops being valid.
                 */
                expires_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Snapshot: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["SnapshotBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_String: {
            /** @description The result payload. Omitted on failure. */
            data?: string;
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Subnet: {
            /**
             * @description The result payload. Omitted on failure.
             * @example {
             *       "cidr": "192.168.1.0/24",
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "description": "Local area network",
             *       "first_discovery_id": null,
             *       "id": "550e8400-e29b-41d4-a716-446655440004",
             *       "last_discovery_id": null,
             *       "last_seen_at": "2026-01-15T10:30:00Z",
             *       "lineage_id": null,
             *       "name": "LAN",
             *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
             *       "source": {
             *         "type": "Manual"
             *       },
             *       "subnet_type": "Lan",
             *       "tags": [],
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null,
             *       "virtualization_service_id": null
             *     }
             */
            data?: components["schemas"]["SubnetBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description The discovery that first observed this entity.
                 */
                readonly first_discovery_id?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description The most recent discovery that observed this entity.
                 */
                readonly last_discovery_id?: string | null;
                /**
                 * Format: date-time
                 * @description When a discovery last observed this entity.
                 */
                readonly last_seen_at?: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Tag: {
            /**
             * @description The result payload. Omitted on failure.
             * @example {
             *       "color": "Green",
             *       "created_at": "2026-01-15T10:30:00Z",
             *       "description": "Production environment resources",
             *       "id": "550e8400-e29b-41d4-a716-44665544000a",
             *       "is_application": false,
             *       "lineage_id": null,
             *       "name": "production",
             *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
             *       "updated_at": "2026-01-15T10:30:00Z",
             *       "valid_from": "2026-01-15T10:30:00Z",
             *       "valid_to": null
             *     }
             */
            data?: components["schemas"]["TagBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_TestReachabilityResponse: {
            /** @description Response from a reachability test. */
            data?: {
                /** @description Error message if not reachable */
                error?: string | null;
                /** @description Health check result (only present when check_health was true) */
                health?: boolean | null;
                /** @description Whether the TCP connection succeeded */
                reachable: boolean;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Topology: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["TopologyBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_TopologyData: {
            /**
             * @description Bundle of entities + the built graph that feed the topology render, export,
             *     and share pipelines.
             *
             *     Loaded by [`crate::server::topology::service::main::TopologyService::get_topology_data`]
             *     for either the live view (`snapshot_id = None`) or a point-in-time snapshot
             *     (`snapshot_id = Some(id)`). The per-view `nodes`/`edges` are built on request
             *     from these entities + the network's grouping options
             *     (`build_all_view_graphs`) — they are not persisted. The frontend selects the
             *     active view's slice client-side.
             */
            data?: {
                /**
                 * @description Views whose data is present in this entity set (L3/Workloads always;
                 *     L2 Physical iff LLDP/CDP neighbors exist; Application iff app-flagged
                 *     tags are used). The topology tab restricts a snapshot's view picker to
                 *     these — you can't set up SNMP or create app tags on a historical
                 *     snapshot — while the live view shows all views with setup prompts.
                 */
                available_views?: components["schemas"]["TopologyView"][];
                /** @description Service bindings included in this topology. */
                bindings: components["schemas"]["Binding"][];
                /** @description Dependencies included in this topology. */
                dependencies: components["schemas"]["Dependency"][];
                /** @description Connections between the nodes of the built graph. */
                edges?: {
                    [key: string]: components["schemas"]["Edge"][];
                };
                /** @description Hosts included in this topology. */
                hosts: components["schemas"]["Host"][];
                /** @description Interfaces included in this topology. */
                interfaces: components["schemas"]["Interface"][];
                /** @description IP addresses included in this topology. */
                ip_addresses: components["schemas"]["IPAddress"][];
                /**
                 * @description Per-view graph built on request from the entities above + grouping
                 *     options. Keyed by view so switching the active perspective is a
                 *     client-side slice selection.
                 */
                nodes?: {
                    [key: string]: components["schemas"]["Node"][];
                };
                /** @description Ports included in this topology. */
                ports: components["schemas"]["Port"][];
                /** @description Services included in this topology. */
                services: components["schemas"]["Service"][];
                /** @description Subnets included in this topology. */
                subnets: components["schemas"]["Subnet"][];
                /** @description Tags assigned to this entity. */
                tags: components["schemas"]["Tag"][];
                /** @description VLANs included in this topology. */
                vlans: components["schemas"]["Vlan"][];
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_User: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["UserBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_UserApiKey: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["UserApiKeyBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_UserApiKeyResponse: {
            /**
             * @description Response for user API key creation/rotation
             *     Contains the full API key record plus the plaintext key (shown only once)
             */
            data?: {
                /** @description The stored key record. */
                api_key: components["schemas"]["UserApiKey"];
                /**
                 * Format: password
                 * @description The plaintext API key - only returned once during creation or rotation
                 */
                readonly key: string;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Vec_BillingPlan: {
            /** @description The result payload. Omitted on failure. */
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
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "SelfHostedStandard";
            }) | (components["schemas"]["PlanConfig"] & {
                /** @enum {string} */
                type: "SelfHostedPlus";
            }))[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Vec_Credential: {
            /** @description The result payload. Omitted on failure. */
            data?: (components["schemas"]["CredentialBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Vec_DiscoveryUpdatePayload: {
            /** @description The result payload. Omitted on failure. */
            data?: {
                /**
                 * Format: uuid
                 * @description The daemon this entity refers to.
                 */
                daemon_id: string;
                /**
                 * Format: uuid
                 * @description The discovery configuration this session belongs to.
                 *     Always enriched server-side; daemons do not send this field.
                 */
                discovery_id?: string | null;
                /** @description What kind of discovery is running. */
                discovery_type: components["schemas"]["DiscoveryType"];
                /** @description Failure message, when the run did not complete. */
                error?: string | null;
                /**
                 * Format: int32
                 * @description Rough estimate of the time left, in seconds.
                 */
                estimated_remaining_secs?: number | null;
                /**
                 * Format: date-time
                 * @description When the run finished. `null` while it is still going.
                 */
                finished_at?: string | null;
                /**
                 * Format: int32
                 * @description Hosts found so far.
                 */
                hosts_discovered?: number | null;
                /**
                 * Format: uuid
                 * @description The network this entity belongs to.
                 */
                network_id: string;
                /** @description Which stage of the run is in progress. */
                phase: components["schemas"]["DiscoveryPhase"];
                /**
                 * Format: int32
                 * @description Completion of the current phase, from 0 to 1.
                 */
                progress: number;
                scanned?: null | components["schemas"]["ScannedEntityIds"];
                /**
                 * Format: uuid
                 * @description The discovery run this update belongs to.
                 */
                session_id: string;
                /**
                 * Format: date-time
                 * @description When the run started.
                 */
                started_at?: string | null;
                /**
                 * @description Non-fatal findings from a completed run — one per occurrence, each carrying the code that
                 *     identifies it and the detail that fills the sentence. Unlike `error`, these do not mark the
                 *     run failed.
                 *
                 *     Read through [`deserialize_warnings`] rather than the derived impl, which is what keeps
                 *     historical records and pre-coded daemons rendering: both send bare strings here, and both
                 *     land as `Unknown` carrying that text instead of failing the whole payload.
                 */
                warnings?: components["schemas"]["DiscoveryWarning"][];
            }[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Vec_Invite: {
            /** @description The result payload. Omitted on failure. */
            data?: (components["schemas"]["InviteBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_Vlan: {
            /** @description The result payload. Omitted on failure. */
            data?: components["schemas"]["VlanBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description The discovery that first observed this entity.
                 */
                readonly first_discovery_id?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description The most recent discovery that observed this entity.
                 */
                readonly last_discovery_id?: string | null;
                /**
                 * Format: date-time
                 * @description When a discovery last observed this entity.
                 */
                readonly last_seen_at?: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_VlanDiscoveryResponse: {
            /** @description Response for discovery upsert */
            data?: {
                /** @description Mapping of vlan_number → VLAN entity UUID */
                vlans: components["schemas"]["VlanDiscoveryResponseItem"][];
            };
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        ApiResponse_u32: {
            /**
             * Format: int32
             * @description The result payload. Omitted on failure.
             */
            data?: number;
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata. */
            meta: components["schemas"]["ApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "SelfHostedStandard";
        }) | (components["schemas"]["PlanConfig"] & {
            /** @enum {string} */
            type: "SelfHostedPlus";
        });
        /** @enum {string} */
        BillingRate: "Month" | "Year";
        /**
         * @description Association between a service and a port / interface that the service is listening on
         * @example {
         *       "created_at": "2026-08-25T22:05:35.391147Z",
         *       "first_discovery_id": null,
         *       "id": "917782d5-c989-410d-b9db-da93a003fcb3",
         *       "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-08-25T22:05:35.391147Z",
         *       "lineage_id": null,
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *       "service_id": "550e8400-e29b-41d4-a716-446655440007",
         *       "type": "Port",
         *       "updated_at": "2026-08-25T22:05:35.391147Z",
         *       "valid_from": "2026-08-25T22:05:35.391147Z",
         *       "valid_to": null
         *     }
         */
        Binding: components["schemas"]["BindingBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description The discovery that first observed this entity.
             */
            readonly first_discovery_id?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: uuid
             * @description The most recent discovery that observed this entity.
             */
            readonly last_discovery_id?: string | null;
            /**
             * Format: date-time
             * @description When a discovery last observed this entity.
             */
            readonly last_seen_at?: string;
            /**
             * Format: uuid
             * @description Stable identifier shared by every revision of the same entity across its history.
             */
            readonly lineage_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description Start of the interval this revision was current for (SCD2 history).
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description End of the interval this revision was current for. `null` while it is the live revision.
             */
            readonly valid_to?: string | null;
        };
        /** @description The base data for a Binding entity (everything except id, created_at, updated_at) */
        BindingBase: components["schemas"]["BindingType"] & {
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /**
             * Format: uuid
             * @description The service this entity refers to.
             */
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
            /**
             * Format: uuid
             * @description The IP address the service is present at.
             */
            ip_address_id: string;
            /** @enum {string} */
            type: "IPAddress";
        } | {
            /**
             * Format: uuid
             * @description Client-provided UUID for this binding
             */
            id: string;
            /**
             * Format: uuid
             * @description null = bind to all ip_addresses
             */
            ip_address_id?: string | null;
            /**
             * Format: uuid
             * @description The port the service listens on.
             */
            port_id: string;
            /** @enum {string} */
            type: "Port";
        };
        /**
         * @description The type of binding - either to an interface or to a port.
         *
         *     Bindings associate a service with network resources (ip_addresses/ports) on a host.
         *
         *     ## Validation Rules
         *
         *     - All bindings must reference ports/interfaces that belong to the same host as the service.
         *     - Interface bindings conflict with port bindings on the same interface.
         *     - A port binding on all ip_addresses (`ip_address_id: null`) conflicts with any interface binding.
         *     - When a port binding with `ip_address_id: null` is created, it supersedes (removes) any
         *       existing specific-interface bindings for the same port.
         */
        BindingType: {
            /**
             * Format: uuid
             * @description The IP address the service is present at.
             */
            ip_address_id: string;
            /** @enum {string} */
            type: "IPAddress";
        } | {
            /**
             * Format: uuid
             * @description The IP address this port binding applies to. If `null`, the binding applies to all
             *     IP addresses on the host (and supersedes specific-IP-address bindings for this port).
             */
            ip_address_id: string | null;
            /**
             * Format: uuid
             * @description The port the service listens on.
             */
            port_id: string;
            /** @enum {string} */
            type: "Port";
        };
        BulkDeleteResponse: {
            /** @description How many records were actually deleted. */
            deleted_count: number;
            /** @description How many IDs the request asked to delete. */
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
        /**
         * @description Cancellation reason captured in `SubscriptionCancelled` /
         *     `CancellationInitiated` events. Mirrors the values surfaced in the
         *     in-app cancel flow (Phase 5).
         * @enum {string}
         */
        CancelReason: "too_expensive" | "missing_features" | "switched_service" | "unused" | "customer_service" | "low_quality" | "too_complex" | "other";
        CancelSubscriptionRequest: {
            /** @description Free-text detail the customer added to their cancellation reason. */
            comment?: string | null;
            /** @description Why the customer is cancelling, as picked from the cancel flow. */
            reason_code: components["schemas"]["CancelReason"];
            save_offer_redeemed?: null | components["schemas"]["SaveOffer"];
            /** @description Whether the retention discount was offered during this flow. */
            save_offer_shown?: components["schemas"]["SaveOffer"][];
        };
        CancelSubscriptionResponse: {
            /**
             * Format: date-time
             * @description When the current paid period ends and access drops to the free tier.
             */
            period_end: string;
        };
        ChangePlanPreview: {
            /**
             * Format: int64
             * @description Hosts over the target plan's allowance, which would be billed as overage.
             */
            excess_hosts: number;
            /**
             * Format: int64
             * @description Networks over the target plan's allowance.
             */
            excess_networks: number;
            /**
             * Format: int64
             * @description Seats over the target plan's allowance.
             */
            excess_seats: number;
        };
        ChangePlanRequest: {
            /** @description Plan to move the subscription to. */
            plan: components["schemas"]["BillingPlan"];
            /** @description Billing interval to move to. */
            rate: components["schemas"]["BillingRate"];
        };
        /** @description Check email availability request */
        CheckEmailRequest: {
            /**
             * Format: email
             * @description Email address to check for an existing account.
             */
            email: string;
        };
        /**
         * @description Where a device's claim about itself came from.
         *
         *     Named rather than folded into a sentence because the operator's next step depends on it: a
         *     wrong `ifNumber` is a firmware bug to report upstream, while a set bridge bit over an empty
         *     bridge table is usually a missing SNMP view or VLAN context on their side.
         * @enum {string}
         */
        ClaimSource: "IfNumber" | "SysServicesBridgeBit" | "LldpLocalIdentity" | "Dot1dBaseNumPorts";
        /** @enum {string} */
        Color: "Pink" | "Rose" | "Red" | "Amber" | "Orange" | "Green" | "Emerald" | "Teal" | "Cyan" | "Blue" | "Indigo" | "Purple" | "Fuchsia" | "Violet" | "Sky" | "Gray" | "Lime" | "Yellow";
        /** @enum {string} */
        ContainerType: "Subnet" | "ServiceCategory" | "Application" | "ApplicationUngrouped" | "Root" | "Host" | "NestedTag" | "NestedServiceCategory" | "Hypervisor" | "ContainerRuntime" | "Stack" | "TrunkPort" | "VLAN" | "PortOpStatus";
        /**
         * @description Input for creating a binding with a service.
         *     `service_id` and `network_id` are assigned by the server after the service is created.
         */
        CreateBindingInput: {
            /**
             * Format: uuid
             * @description The IP address the service is present at.
             */
            ip_address_id: string;
            /** @enum {string} */
            type: "IPAddress";
        } | {
            /**
             * Format: uuid
             * @description The IP address this port binding applies to. `null` binds to every IP address on the host.
             */
            ip_address_id?: string | null;
            /**
             * Format: uuid
             * @description The port the service listens on.
             */
            port_id: string;
            /** @enum {string} */
            type: "Port";
        };
        CreateCheckoutRequest: {
            /** @description Plan to subscribe to. */
            plan: components["schemas"]["BillingPlan"];
            /** @description URL to return the user to after checkout completes. */
            url: string;
        };
        /**
         * @description Request type for creating a host with its associated ip_addresses, ports, and services.
         *     Server assigns `host_id`, `network_id`, and `source` to all children.
         *     Client must provide UUIDs for all entities, enabling services to reference
         *     ip_addresses/ports by ID in the same request.
         * @example {
         *       "credential_assignments": [],
         *       "description": "Primary web server",
         *       "hidden": false,
         *       "hostname": "web-server-01.local",
         *       "interfaces": [],
         *       "ip_addresses": [
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
         *               "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
         *               "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *               "type": "Port"
         *             }
         *           ],
         *           "id": "550e8400-e29b-41d4-a716-446655440007",
         *           "name": "nginx",
         *           "position": 0,
         *           "service_definition": "Google Home",
         *           "tags": [],
         *           "virtualization_metadata": null,
         *           "virtualization_service_id": null
         *         }
         *       ],
         *       "tags": [],
         *       "virtualization_metadata": null,
         *       "virtualization_service_id": null
         *     }
         */
        CreateHostRequest: {
            /** @description LLDP chassis identifier, used to match the host to its neighbours. */
            chassis_id?: string | null;
            /** @description Credentials to scan this host with. */
            credential_assignments?: components["schemas"]["CredentialAssignment"][];
            /** @description Free-text notes about the host. */
            description?: string | null;
            /** @description Hide the host from topology views without deleting it. */
            hidden?: boolean;
            /** @description Hostname as resolved or reported by the host. */
            hostname?: string | null;
            /** @description SNMP interface entries (ifTable data) - server assigns UUIDs */
            interfaces?: components["schemas"]["InterfaceInput"][];
            /** @description Interfaces to create with this host (client provides UUIDs) */
            ip_addresses?: components["schemas"]["IPAddressInput"][];
            /** @description Link to the host's own management interface. */
            management_url?: string | null;
            /** @description Human-facing name for the host. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description Ports to create with this host (client provides UUIDs) */
            ports?: components["schemas"]["PortInput"][];
            /** @description Services to create with this host (can reference ip_addresses/ports by their UUIDs) */
            services?: components["schemas"]["ServiceInput"][];
            /** @description SNMP sysContact — administrative contact as configured on the device. */
            sys_contact?: string | null;
            /** @description SNMP sysDescr — the device's own description of itself. */
            sys_descr?: string | null;
            /** @description SNMP sysLocation — physical location as configured on the device. */
            sys_location?: string | null;
            /** @description SNMP sysObjectID — the vendor's identifier for the device model. */
            sys_object_id?: string | null;
            /** @description Tags assigned to this entity. */
            tags: string[];
            virtualization_metadata?: null | components["schemas"]["HostVirtualization"];
            /**
             * Format: uuid
             * @description The hypervisor service this VM runs on.
             */
            virtualization_service_id?: string | null;
        };
        CreateInviteRequest: {
            /**
             * Format: int64
             * @description How long the invite stays valid, in hours.
             */
            expiration_hours?: number | null;
            /** @description The networks this entity applies to. */
            network_ids: string[];
            /** @description Role the invited user gets on acceptance. */
            permissions: components["schemas"]["UserOrgPermissions"];
            /** @description Address to email the invite to. Omit to create a link without sending. */
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
            /**
             * Format: uuid
             * @description The host this entity belongs to.
             */
            host_id: string;
            /** @description Human-facing name for the service. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description Which known software this service is, if identified. */
            service_definition: string;
            /** @description Tags assigned to this entity. */
            tags: string[];
            virtualization_metadata?: null | components["schemas"]["ServiceVirtualization"];
            /**
             * Format: uuid
             * @description The container runtime service hosting this container, if any.
             */
            virtualization_service_id?: string | null;
        };
        CreateSnapshotRequest: {
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
        };
        CreateUpdateShareRequest: {
            /** @description The share to create or replace. */
            share: components["schemas"]["Share"];
        };
        Credential: components["schemas"]["CredentialBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        /** @description A credential assigned to a host, optionally limited to specific ip_addresses. */
        CredentialAssignment: {
            /**
             * Format: uuid
             * @description The credential this entity refers to.
             */
            credential_id: string;
            /** @description Interface IDs to limit this credential to. None = all host ip_addresses. */
            ip_address_ids: string[] | null;
        };
        /** @description One credential's attempt against one address, and what the client library said about it. */
        CredentialAttempt: {
            address: string;
            /**
             * @description The library's own diagnostic — free text, so it can only ever be displayed. It is the one
             *     thing the code cannot supersede: the code says which failure mode, this says what actually
             *     came back ("connection refused (os error 111)"), and it is now attributable to this one
             *     address rather than being the first message of a whole batch.
             */
            detail: string | null;
            integration: components["schemas"]["CredentialQueryPayloadDiscriminants"];
        };
        CredentialBase: {
            /**
             * @description Networks this credential is assigned to (Broadcast scope).
             *     Hydrated from the `network_credentials` junction table.
             */
            assigned_network_ids: string[];
            /** @description Protocol this credential authenticates with, and its settings. */
            credential_type: components["schemas"]["CredentialType"];
            /**
             * @description Hosts this credential is assigned to (PerHost scope), with optional IP scoping.
             *     Hydrated from the `host_credentials` junction table.
             */
            host_assignments: components["schemas"]["CredentialHostAssignment"][];
            /** @description Human-facing name for this credential. */
            name: string;
            /**
             * Format: uuid
             * @description The organization that owns this record.
             */
            organization_id: string;
            /** @description Tags assigned to this entity. */
            tags: string[];
        };
        /**
         * @description Host-keyed mirror of [`CredentialAssignment`]: a host this credential is
         *     assigned to, optionally limited to specific ip_addresses. Hydrated onto a
         *     credential from the `host_credentials` junction (PerHost scope).
         */
        CredentialHostAssignment: {
            /**
             * Format: uuid
             * @description The host this entity belongs to.
             */
            host_id: string;
            /** @description IP address IDs to limit this credential to on the host. None = all host ip_addresses. */
            ip_address_ids: string[] | null;
        };
        /** @enum {string} */
        CredentialOrderField: "created_at" | "name" | "updated_at";
        /** @enum {string} */
        CredentialQueryPayloadDiscriminants: "Snmp" | "DockerProxy" | "DockerSocket" | "PodmanProxy" | "PodmanSocket" | "UnifiController" | "InstantOn" | "Gnmi" | "Unknown";
        /**
         * @description Release maturity of a credential type's integration.
         *
         *     Additive and exhaustive: a new credential variant will not compile until it declares its
         *     stability, and every existing type is `Stable` by explicit arm rather than by wildcard, so
         *     promoting an integration is a one-line reviewable change rather than a deletion nobody
         *     notices. This is presentation metadata about the *code*, like `minimum_daemon_version` —
         *     it is never stored on a credential row, so it carries no deploy-coexistence obligation.
         * @enum {string}
         */
        CredentialStability: "Stable" | "Beta";
        /**
         * @description Universal credential type — tagged enum stored as JSONB.
         *     Each variant represents a different credential protocol/method.
         */
        CredentialType: {
            /** @description SNMPv1 community string. */
            community: components["schemas"]["SecretValue"];
            /** @enum {string} */
            type: "SnmpV1";
        } | {
            /** @description SNMPv2c community string. */
            community: components["schemas"]["SecretValue"];
            /** @enum {string} */
            type: "SnmpV2c";
        } | {
            /** @description Authentication passphrase. */
            auth_password: components["schemas"]["SecretValue"];
            /** @description Hash algorithm used for authentication. */
            auth_protocol: components["schemas"]["SnmpV3AuthProtocol"];
            /** @description Optional context name (default/empty context used if unset). */
            context_name?: string | null;
            /** @description Privacy passphrase. */
            priv_password: components["schemas"]["SecretValue"];
            /** @description Cipher used for privacy (encryption). */
            priv_protocol: components["schemas"]["SnmpV3PrivProtocol"];
            /** @description USM security (user) name. */
            security_name: string;
            /** @enum {string} */
            type: "SnmpV3";
        } | {
            /** @description Password, sent as gRPC `password` metadata. */
            password: components["schemas"]["SecretValue"];
            /**
             * Format: int32
             * @description gNMI port. 9339 is IANA's; some NOSes listen on 6030 or 57400 instead.
             */
            port?: number;
            /**
             * @description Accept any server certificate. Only meaningful with `tls` — NOS gRPC endpoints
             *     commonly ship self-signed certs.
             */
            skip_verify?: boolean;
            /**
             * @description Use TLS. Off means plaintext gRPC (h2c) — the out-of-the-box mode of several NOSes
             *     (ArcOS among them: its server stays plaintext until transport-security is enabled).
             */
            tls?: boolean;
            /** @enum {string} */
            type: "Gnmi";
            /** @description Username, sent as gRPC `username` metadata (the openconfig convention). */
            username: string;
        } | {
            /** @description Optional URL path prefix (e.g. "/v1.43") */
            path?: string | null;
            /**
             * Format: int32
             * @description Port for the Docker API proxy (default 2375)
             */
            port?: number;
            ssl_cert?: null | components["schemas"]["FileOrInline"];
            ssl_chain?: null | components["schemas"]["FileOrInline"];
            ssl_key?: null | components["schemas"]["SecretValue"];
            /** @enum {string} */
            type: "DockerProxy";
        } | {
            /** @description Path to the Docker socket. Blank lets the daemon auto-detect it. */
            socket_path?: string | null;
            /** @enum {string} */
            type: "DockerSocket";
        } | {
            /** @description Optional URL path prefix (e.g. "/v1.43") */
            path?: string | null;
            /**
             * Format: int32
             * @description Port for the Podman API proxy (default 2375)
             */
            port?: number;
            ssl_cert?: null | components["schemas"]["FileOrInline"];
            ssl_chain?: null | components["schemas"]["FileOrInline"];
            ssl_key?: null | components["schemas"]["SecretValue"];
            /** @enum {string} */
            type: "PodmanProxy";
        } | {
            /** @description Path to the Podman socket. Blank lets the daemon auto-detect it. */
            socket_path?: string | null;
            /** @enum {string} */
            type: "PodmanSocket";
        } | {
            /** @description Network Application API key, sent as `X-API-KEY`. */
            api_key: components["schemas"]["SecretValue"];
            /**
             * Format: int32
             * @description Controller HTTPS port. 443 for a UniFi OS console, 11443 for UniFi OS Server.
             */
            port?: number;
            /** @description Internal site name from the controller URL (`/manage/site/<name>`). */
            site?: string;
            /** @enum {string} */
            type: "UnifiApiKey";
        } | {
            /** @description Password for that account. */
            password: components["schemas"]["SecretValue"];
            /**
             * Format: int32
             * @description Controller HTTPS port. 443 UniFi OS console, 11443 UniFi OS Server, 8443 legacy.
             */
            port?: number;
            /** @description Internal site name from the controller URL (`/manage/site/<name>`). */
            site?: string;
            /** @enum {string} */
            type: "UnifiLocalAdmin";
            /** @description Local admin account on the controller. */
            username: string;
        } | {
            /** @description Password for that account. */
            password: components["schemas"]["SecretValue"];
            /** @description Restrict the fetch to one site by name. Blank ⇒ every site the account can see. */
            site?: string | null;
            /** @enum {string} */
            type: "InstantOnAccount";
            /** @description Portal account email address. */
            username: string;
        };
        /** @enum {string} */
        CredentialTypeDiscriminants: "SnmpV1" | "SnmpV2c" | "SnmpV3" | "Gnmi" | "DockerProxy" | "DockerSocket" | "PodmanProxy" | "PodmanSocket" | "UnifiApiKey" | "UnifiLocalAdmin" | "InstantOnAccount";
        Daemon: components["schemas"]["DaemonBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        DaemonApiKey: components["schemas"]["DaemonApiKeyBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        DaemonApiKeyBase: {
            /**
             * Format: uuid
             * @description Daemon this key is bound to 1:1, when provisioned server-side.
             *     NULL for legacy network-shared keys created before 1:1 provisioning,
             *     which resolve daemon identity from the X-Daemon-ID header instead.
             */
            readonly daemon_id?: string | null;
            /**
             * Format: date-time
             * @description When this record stops being valid.
             */
            expires_at?: string | null;
            /** @description Whether the key may still be used. Disabled keys are rejected. */
            is_enabled?: boolean;
            /** @description The stored key. Returned redacted except on creation and rotation. */
            readonly key: string;
            /**
             * Format: date-time
             * @description When a daemon last authenticated with this key.
             */
            readonly last_used: string | null;
            /** @description Human-facing name for this key. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description Tags assigned to this entity. */
            tags: string[];
        };
        DaemonApiKeyResponse: {
            /** @description The stored key record. */
            api_key: components["schemas"]["DaemonApiKey"];
            /**
             * Format: password
             * @description The plaintext API key - only returned once during creation or rotation.
             */
            readonly key: string;
        };
        DaemonBase: {
            /**
             * Format: uuid
             * @description Foreign key to API key used for ServerPoll authentication.
             *     NULL for DaemonPoll daemons or those not yet linked to a key.
             */
            api_key_id?: string | null;
            /**
             * Format: uuid
             * @description The host this entity belongs to.
             */
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
            /** @description How the daemon connects: it polls the server, or the server polls it. */
            mode: components["schemas"]["DaemonMode"];
            /** @description Human-facing name for this daemon. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description Whether the daemon is on standby due to inactivity (no discovery in 30 days). */
            readonly standby?: boolean;
            /**
             * Format: date-time
             * @description Timestamp of the most recent standby → active transition. Set by
             *     `process_startup` when a restarted daemon is un-standby'd, and by
             *     the discovery auto-wake path. The nightly inactivity check skips
             *     daemons within the grace window (see `STANDBY_GRACE_PERIOD_DAYS`)
             *     to prevent the "restart → cleared → re-standby'd before discovery
             *     runs" race.
             */
            readonly standby_cleared_at?: string | null;
            /** @description Tags assigned to this entity. */
            tags: string[];
            /**
             * Format: uri
             * @description Address the *server* dials for a ServerPoll daemon. Editable (a daemon can move);
             *     unused and not editable for DaemonPoll, which dials out instead.
             *     Base URL the server reaches this daemon on.
             * @example https://daemon.example.com:60073
             */
            url: string;
            /**
             * Format: uuid
             * @description User responsible for maintaining this daemon
             */
            user_id: string;
            /**
             * @description Daemon software version (semver format)
             * @example 0.17.7
             */
            version?: string | null;
        };
        /**
         * @description Legacy heartbeat payload for backwards compatibility with pre-v0.14.0 daemons.
         *     Old daemons call POST /api/daemons/{id}/heartbeat with this payload.
         */
        DaemonHeartbeatPayload: {
            /** @description How the daemon connects: it polls the server, or the server polls it. */
            mode: components["schemas"]["DaemonMode"];
            /** @description Name the daemon reports for itself. */
            name: string;
            /** @description URL the daemon is reachable at, as it sees itself. */
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
        /**
         * @description Operating system the install command was generated for.
         * @enum {string}
         */
        DaemonOs: "linux" | "macos" | "windows" | "freebsd";
        /**
         * @description Which daemon-prompt CTA the user chose.
         * @enum {string}
         */
        DaemonPromptAction: "dismissed" | "accepted";
        /** @description Request recording the user's response to the "Start Discovering Your Network" prompt. */
        DaemonPromptResponseRequest: {
            /** @description What the user chose to do about the daemon prompt. */
            action: components["schemas"]["DaemonPromptAction"];
        };
        /** @description Daemon registration request from daemon to server */
        DaemonRegistrationRequest: {
            /**
             * @description Legacy pre-0.15 interfaced-subnet channel (deserialize-only; see
             *     [`LegacyCapabilities`]). Repopulated by the first heartbeat, so registration
             *     does not persist it.
             */
            capabilities?: components["schemas"]["LegacyCapabilities"];
            /**
             * Format: uuid
             * @description The daemon this entity refers to.
             */
            daemon_id: string;
            /**
             * @description Per-daemon integration targeting from the init command (credentialed cred↔IP and
             *     credential-less local sockets). Written to this daemon's Discovery at registration so
             *     it's present before the first session dispatches. Registration assumes new-daemon →
             *     new-server, so there is no legacy bare-`credential_ids` field — bare-uuid env back-compat
             *     is handled in the daemon's env parser, never on the wire.
             */
            integration_targets?: components["schemas"]["IntegrationTarget"][];
            /** @description How the daemon connects: it polls the server, or the server polls it. */
            mode: components["schemas"]["DaemonMode"];
            /** @description Name the daemon reports for itself. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
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
            /** @description The registered daemon record. */
            daemon: components["schemas"]["Daemon"];
            /**
             * Format: uuid
             * @description The host this entity belongs to.
             */
            host_id: string;
            server_capabilities?: null | components["schemas"]["ServerCapabilities"];
        };
        /** @description Daemon response for UI including computed version status */
        DaemonResponse: components["schemas"]["DaemonBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            id: string;
            /**
             * @description Subnets this daemon has interfaces on, loaded from the
             *     `daemon_interfaced_subnets` junction (replaces the old
             *     `capabilities.interfaced_subnet_ids` JSONB field).
             */
            interfaced_subnet_ids: string[];
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            updated_at: string;
            /** @description Computed version status including health and warnings */
            version_status: components["schemas"]["DaemonVersionStatus"];
        };
        /** @description Sent by daemon on startup to report version */
        DaemonStartupRequest: {
            /** @description Daemon software version (semver format) */
            daemon_version: string;
        };
        /** @description Lightweight daemon status for polling responses. */
        DaemonStatus: {
            /** @description Backwards compat: pre-v0.15.0 daemons send capabilities instead of interfaced_subnets. */
            capabilities?: components["schemas"]["LegacyCapabilities"];
            /**
             * @description Subnets detected from daemon's network ip_addresses. Server resolves these
             *     via SubnetService::create (create-or-match by CIDR) to get real IDs.
             *     v0.15.0+ daemons populate this; pre-v0.15.0 daemons leave it empty.
             */
            interfaced_subnets?: components["schemas"]["Subnet"][];
            /** @description How the daemon connects: it polls the server, or the server polls it. */
            mode: components["schemas"]["DaemonMode"];
            /** @description Name the daemon reports for itself. */
            name: string;
            /**
             * @description Whether the daemon can accept a new discovery session.
             *     Both DaemonPoll and ServerPoll use this to avoid dispatching work to a busy daemon.
             */
            ready_for_work?: boolean;
            /**
             * @description URL is not used by server - kept for backwards compat.
             *     Server never updates daemon URL from status (URL is set during provisioning).
             */
            url?: string | null;
            /** @description Daemon software version (semver format) */
            version?: string | null;
        };
        /** @description Daemon version status including health and any warnings */
        DaemonVersionStatus: {
            /** @description Whether a containerized daemon is mounted so it can read the Docker socket. */
            has_correct_docker_volume_mount?: boolean;
            /** @description Whether that version is current, ageing, or out of support. */
            status: components["schemas"]["VersionHealthStatus"];
            /**
             * @description The date this daemon's version stops being supported, if a sunset is
             *     scheduled for it. Surfaced top-level (not only inside `warnings`) so the
             *     UI can render a countdown from the same value the email uses.
             */
            sunset_date?: string | null;
            /**
             * @description Whether this daemon can run a single-host rescan. Server-computed so the
             *     frontend never has to hardcode a version floor.
             */
            supports_targeted_rescan?: boolean;
            /** @description Whether the daemon can run a combined discovery pass. */
            supports_unified_discovery?: boolean;
            /** @description Version the daemon reports. */
            version?: string | null;
            /** @description Upgrade warnings that apply to this version. */
            warnings?: components["schemas"]["DeprecationWarning"][];
        };
        /** @description Dashboard summary response */
        DashboardSummary: {
            /** @description Daemons the caller can see, with their current status. */
            daemons: components["schemas"]["DaemonResponse"][];
            /** @description Per-network counts for every network the caller can see. */
            networks: components["schemas"]["NetworkSummary"][];
            /** @description Current usage against the organization's plan allowances. */
            plan_usage: components["schemas"]["PlanUsage"];
            /** @description The most recent discovery runs, newest first. */
            recent_discoveries: components["schemas"]["Discovery"][];
        };
        /**
         * @description Lifecycle of a demo-populate task. `Running` is set synchronously in the
         *     POST handler (before the `202`), then flipped to a terminal variant by the
         *     spawned task. `Failed` carries the error string so the UI can show why.
         */
        DemoPopulateStatus: {
            /**
             * Format: date-time
             * @description When population began.
             */
            started_at: string;
            /** @enum {string} */
            state: "running";
        } | {
            /**
             * Format: date-time
             * @description When population finished.
             */
            finished_at: string;
            /** @enum {string} */
            state: "complete";
        } | {
            /** @description Why population failed. */
            error: string;
            /**
             * Format: date-time
             * @description When it gave up.
             */
            finished_at: string;
            /** @enum {string} */
            state: "failed";
        };
        /**
         * @example {
         *       "color": "Blue",
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "dependency_type": "RequestPath",
         *       "description": "HTTP/HTTPS services dependency",
         *       "edge_style": "Bezier",
         *       "id": "550e8400-e29b-41d4-a716-446655440008",
         *       "lineage_id": null,
         *       "members": {
         *         "service_ids": [],
         *         "type": "Services"
         *       },
         *       "name": "Web Services",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null
         *     }
         */
        Dependency: components["schemas"]["DependencyBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: uuid
             * @description Stable identifier shared by every revision of the same entity across its history.
             */
            readonly lineage_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description Start of the interval this revision was current for (SCD2 history).
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description End of the interval this revision was current for. `null` while it is the live revision.
             */
            readonly valid_to?: string | null;
        };
        DependencyBase: {
            /** @description Colour the dependency edge is drawn in. */
            color: components["schemas"]["Color"];
            /** @description What kind of relationship this dependency records. */
            dependency_type: components["schemas"]["DependencyType"];
            /** @description Free-text notes about the dependency. */
            description?: string | null;
            /** @description Line style the dependency edge is drawn with. */
            edge_style: components["schemas"]["EdgeStyle"];
            /** @description Members of this dependency: either service IDs or binding IDs. */
            members: components["schemas"]["DependencyMembers"];
            /** @description Human-facing name for this dependency. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description Will be automatically set to Manual for creation through API */
            source?: components["schemas"]["EntitySource"];
            /** @description Tags assigned to this entity. */
            tags: string[];
        };
        /**
         * @description The members of a dependency: either service-level or binding-level.
         *     Bindings are all-or-nothing: either every position has a binding (full L3 detail)
         *     or none do (Application-level only).
         */
        DependencyMembers: {
            /** @description The services in the chain, in order. */
            service_ids: string[];
            /** @enum {string} */
            type: "Services";
        } | {
            /** @description The bindings in the chain, in order — one per service. */
            binding_ids: string[];
            /** @enum {string} */
            type: "Bindings";
        };
        /**
         * @description Fields that dependencies can be ordered/grouped by.
         * @enum {string}
         */
        DependencyOrderField: "created_at" | "name" | "dependency_type" | "updated_at" | "network_id";
        /** @enum {string} */
        DependencyType: "RequestPath" | "HubAndSpoke";
        /** @enum {string} */
        DeploymentType: "cloud" | "commercial" | "community";
        /**
         * @description Severity level for deprecation warnings
         * @enum {string}
         */
        DeprecationSeverity: "Info" | "Warning" | "Critical" | "Unknown";
        /** @description Deprecation warning for daemon version */
        DeprecationWarning: {
            /** @description What the operator needs to do, and by when. */
            message: string;
            /** @description How urgent the upgrade is. */
            severity: components["schemas"]["DeprecationSeverity"];
            /** @description Date after which this daemon version stops being supported. */
            sunset_date?: string | null;
        };
        Discovery: components["schemas"]["DiscoveryBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /** @description When true, the next scan will be a full port scan regardless of interval */
            force_full_scan?: boolean;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * @description Per-daemon integration targeting: which integrations run on this daemon, and on which
             *     IPs. Delivered via the init command at registration and editable via the discovery
             *     modal. This is the single home for cred↔IP targeting; it replaces the global
             *     `credential.target_ips` (race-prone, consumed once).
             *
             *     One-shot: a target is offered to the daemon until a scan completes successfully, then
             *     dropped by [`Discovery::apply_successful_scan`]. Credentials that earned a durable home
             *     during the scan keep being retried from there — `host_credentials` for one that probed
             *     successfully, `network_credentials` for a broadcast one (see
             *     [`Discovery::take_network_scope_credential_ids`]).
             */
            integration_targets: components["schemas"]["IntegrationTarget"][];
            /**
             * Format: int32
             * @description Number of completed scans (incremented by server on session completion)
             */
            readonly scan_count?: number;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        DiscoveryBase: {
            /**
             * Format: uuid
             * @description The daemon this entity refers to.
             */
            daemon_id: string;
            /** @description What this run scans — a subnet, a single host, a container runtime, and so on. */
            discovery_type: components["schemas"]["DiscoveryType"];
            /** @description Human-facing name for this discovery. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description Whether this run was triggered by hand or on a schedule. */
            run_type: components["schemas"]["RunType"];
            /** @description Tags assigned to this entity. */
            tags: string[];
        };
        /**
         * @description Request type for daemon discovery - accepts full entities with IDs.
         *     Used internally by daemons for host creation/upsert, NOT the external API.
         *     This supports the discovery workflow where daemons manage entity IDs.
         *
         *     ## Backwards compatibility (daemons < v0.16.0)
         *
         *     Pre-v0.16.0 daemons send the old field layout:
         *       - `interfaces` → IPAddress data (now `ip_addresses`)
         *       - `if_entries` → SNMP Interface data (now `interfaces`)
         *
         *     The custom deserializer detects the old layout (missing `ip_addresses` field)
         *     and remaps fields automatically. This can be removed once all daemons are ≥ v0.16.0.
         */
        DiscoveryHostRequest: {
            /** @description The host as observed by the daemon. */
            host: components["schemas"]["Host"];
            /**
             * @description Which groups of per-interface data (LLDP, CDP, FDB, VLAN membership) this scan read in
             *     full. A group the daemon could not finish reading must not overwrite what is already
             *     stored: a cut-short walk returns the same empty result as a device with nothing to report,
             *     and for the neighbour fields that also drops the row out of L2 resolution for good.
             *     Daemons predating this field omit it; it defaults to all-complete so they behave as before.
             */
            interface_data_complete?: components["schemas"]["InterfaceDataComplete"];
            /** @description SNMP interface entries (ifTable data) - optional, populated when SNMP is enabled. */
            interfaces?: components["schemas"]["Interface"][];
            /**
             * @description Whether `interfaces` is a complete, authoritative ifTable. When false (a partial SNMP walk
             *     cut short by timeout/error), the server must NOT prune interfaces missing from this scan —
             *     otherwise a transient partial walk tears down the host's L2 topology (#649). Daemons that
             *     predate this field omit it; it defaults to true so their behavior is unchanged.
             */
            interfaces_complete?: boolean;
            /** @description IP addresses observed on the host. */
            ip_addresses: components["schemas"]["IPAddress"][];
            /** @description Open ports observed on the host. */
            ports: components["schemas"]["Port"][];
            /** @description Services identified on the host. */
            services: components["schemas"]["Service"][];
            /**
             * @description Integration-derived subnets (e.g., Docker bridge networks) — created during
             *     create_with_children after service dedup so virtualization.service_id is correct.
             */
            subnets?: components["schemas"]["Subnet"][];
        };
        /**
         * @description Fields that discoveries can be ordered/grouped by.
         * @enum {string}
         */
        DiscoveryOrderField: "created_at" | "name" | "updated_at" | "daemon_id" | "network_id" | "discovery_type";
        /** @enum {string} */
        DiscoveryPhase: "AwaitingSnapshot" | "Queued" | "Pending" | "Starting" | "Started" | "Scanning" | "Complete" | "Failed" | "Cancelled";
        /**
         * @description Protocol that discovered the physical link between network devices
         * @enum {string}
         */
        DiscoveryProtocol: "LLDP" | "CDP";
        DiscoveryType: {
            /**
             * Format: uuid
             * @description The host the daemon is running on.
             */
            host_id: string;
            /** @enum {string} */
            type: "SelfReport";
        } | {
            /** @description What to name a host by when reverse DNS gives nothing. */
            host_naming_fallback: components["schemas"]["HostNamingFallback"];
            /**
             * @description SNMP credentials for querying devices during discovery
             *     Server builds this mapping before initiating discovery
             */
            snmp_credentials?: Record<string, never>;
            /** @description Subnets to sweep. `null` sweeps every subnet on the network. */
            subnet_ids: string[] | null;
            /** @enum {string} */
            type: "Network";
        } | {
            /**
             * Format: uuid
             * @description The host the daemon is running on.
             */
            host_id: string;
            /** @description What to name a host by when reverse DNS gives nothing. */
            host_naming_fallback: components["schemas"]["HostNamingFallback"];
            /** @enum {string} */
            type: "Docker";
        } | {
            /**
             * Format: uuid
             * @description ID of the host that the daemon is running on — same meaning as every
             *     other variant. The host being rescanned is `target_host_id`.
             */
            host_id: string;
            /** @description Addresses to scan on that host. */
            ips: string[];
            /**
             * @description Ports already known on that host, re-checked to confirm they are
             *     still open. Scanned in addition to the standard discovery set, so a
             *     rescan also surfaces newly-opened services.
             */
            ports?: components["schemas"]["PortType"][];
            settings?: components["schemas"]["RescanSettings"];
            /**
             * Format: uuid
             * @description The host being rescanned.
             */
            target_host_id: string;
            /** @enum {string} */
            type: "Rescan";
        } | {
            /**
             * Format: uuid
             * @description ID of the host that the daemon is running on
             */
            host_id: string;
            /** @description Fallback strategy for naming discovered hosts */
            host_naming_fallback: components["schemas"]["HostNamingFallback"];
            /** @description Per-discovery scan performance settings */
            scan_settings?: components["schemas"]["ScanSettings"];
            /** @description Subnets to scan. None = scan all interfaced subnets. */
            subnet_ids: string[] | null;
            /** @enum {string} */
            type: "Unified";
        };
        /** @description Progress update from daemon to server during discovery */
        DiscoveryUpdatePayload: {
            /**
             * Format: uuid
             * @description The daemon this entity refers to.
             */
            daemon_id: string;
            /**
             * Format: uuid
             * @description The discovery configuration this session belongs to.
             *     Always enriched server-side; daemons do not send this field.
             */
            discovery_id?: string | null;
            /** @description What kind of discovery is running. */
            discovery_type: components["schemas"]["DiscoveryType"];
            /** @description Failure message, when the run did not complete. */
            error?: string | null;
            /**
             * Format: int32
             * @description Rough estimate of the time left, in seconds.
             */
            estimated_remaining_secs?: number | null;
            /**
             * Format: date-time
             * @description When the run finished. `null` while it is still going.
             */
            finished_at?: string | null;
            /**
             * Format: int32
             * @description Hosts found so far.
             */
            hosts_discovered?: number | null;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description Which stage of the run is in progress. */
            phase: components["schemas"]["DiscoveryPhase"];
            /**
             * Format: int32
             * @description Completion of the current phase, from 0 to 1.
             */
            progress: number;
            scanned?: null | components["schemas"]["ScannedEntityIds"];
            /**
             * Format: uuid
             * @description The discovery run this update belongs to.
             */
            session_id: string;
            /**
             * Format: date-time
             * @description When the run started.
             */
            started_at?: string | null;
            /**
             * @description Non-fatal findings from a completed run — one per occurrence, each carrying the code that
             *     identifies it and the detail that fills the sentence. Unlike `error`, these do not mark the
             *     run failed.
             *
             *     Read through [`deserialize_warnings`] rather than the derived impl, which is what keeps
             *     historical records and pre-coded daemons rendering: both send bare strings here, and both
             *     land as `Unknown` carrying that text instead of failing the whole payload.
             */
            warnings?: components["schemas"]["DiscoveryWarning"][];
        };
        /**
         * @description A single non-fatal finding from one discovery run, about one device, neighbour, or the scan
         *     itself.
         *
         *     Serialized with the code as the tag, so the generated TypeScript is a discriminated union the
         *     UI can switch on exhaustively. The derived `Deserialize` reads that shape; the leniency that
         *     keeps historical records and pre-coded daemons working lives in [`deserialize_warnings`],
         *     which is applied at the one field that holds these.
         */
        DiscoveryWarning: {
            address: string;
            /** @enum {string} */
            code: "InterfaceSetCutShort";
            /** Format: int32 */
            collected: number;
        } | {
            address: string;
            /** @enum {string} */
            code: "InterfaceDetailsCutShort";
            /** Format: int32 */
            collected: number;
        } | {
            address: string;
            /** @enum {string} */
            code: "SnmpWalkEntryCap";
            group: components["schemas"]["SnmpWalkGroup"];
            /** Format: int32 */
            limit: number;
        } | {
            address: string;
            /** @enum {string} */
            code: "SnmpWalkUnsupported";
            group: components["schemas"]["SnmpWalkGroup"];
        } | {
            address: string;
            /** @enum {string} */
            code: "SnmpWalkDesynchronised";
            group: components["schemas"]["SnmpWalkGroup"];
        } | {
            address: string;
            /** @enum {string} */
            code: "SnmpWalkPartialDiscarded";
            group: components["schemas"]["SnmpWalkGroup"];
        } | {
            address: string;
            /** @enum {string} */
            code: "SnmpWalkPartialRecorded";
            group: components["schemas"]["SnmpWalkGroup"];
        } | {
            address: string;
            /** @enum {string} */
            code: "SnmpWalkBridgeMibAbsent";
            group: components["schemas"]["SnmpWalkGroup"];
        } | {
            address: string;
            /** @enum {string} */
            code: "SnmpWalkNoAnswer";
            group: components["schemas"]["SnmpWalkGroup"];
        } | {
            address: string;
            /** @enum {string} */
            code: "ClaimedCountReadCutShort";
            /** Format: int32 */
            expected: number;
            group: components["schemas"]["SnmpWalkGroup"];
            /** Format: int32 */
            observed: number;
            source: components["schemas"]["ClaimSource"];
        } | {
            address: string;
            /** @enum {string} */
            code: "ClaimedCountUnderRead";
            /** Format: int32 */
            expected: number;
            group: components["schemas"]["SnmpWalkGroup"];
            /** Format: int32 */
            observed: number;
            source: components["schemas"]["ClaimSource"];
        } | {
            address: string;
            /** @enum {string} */
            code: "ClaimedCapabilityReadCutShort";
            group: components["schemas"]["SnmpWalkGroup"];
            source: components["schemas"]["ClaimSource"];
        } | {
            address: string;
            /** @enum {string} */
            code: "ClaimedCapabilityEmpty";
            group: components["schemas"]["SnmpWalkGroup"];
            source: components["schemas"]["ClaimSource"];
        } | {
            address: string;
            /** @enum {string} */
            code: "LldpLocalPortDropped";
            /** Format: int32 */
            dropped: number;
            /** Format: int32 */
            total: number;
        } | {
            address: string;
            /** @enum {string} */
            code: "LldpLocalPortMisplaced";
            /** Format: int32 */
            misplaced: number;
        } | (components["schemas"]["MalformedNeighbours"] & {
            /** @enum {string} */
            code: "MalformedNeighboursWalkCutShort";
        }) | (components["schemas"]["MalformedNeighbours"] & {
            /** @enum {string} */
            code: "MalformedNeighboursGhostRows";
        }) | (components["schemas"]["MalformedNeighbours"] & {
            /** @enum {string} */
            code: "MalformedNeighboursIncompleteRecords";
        }) | (components["schemas"]["MalformedNeighbours"] & {
            /** @enum {string} */
            code: "MalformedNeighboursUnexpectedType";
        }) | (components["schemas"]["MalformedNeighbours"] & {
            /** @enum {string} */
            code: "MalformedNeighboursUnreadableIndex";
        }) | {
            address: string;
            /** @enum {string} */
            code: "SnmpCollectedNothing";
        } | {
            address: string;
            /** @enum {string} */
            code: "VlanRecordingFailed";
        } | {
            address: string;
            /** @enum {string} */
            code: "CredentialTargetNotScanned";
            integration: components["schemas"]["CredentialQueryPayloadDiscriminants"];
        } | {
            address: string;
            /** @enum {string} */
            code: "CredentialTargetNotResponding";
            integration: components["schemas"]["CredentialQueryPayloadDiscriminants"];
        } | {
            address: string;
            /** @enum {string} */
            code: "CredentialGateClosed";
            integration: components["schemas"]["CredentialQueryPayloadDiscriminants"];
            ports: number[];
        } | (components["schemas"]["CredentialAttempt"] & {
            /** @enum {string} */
            code: "CredentialRejected";
        }) | (components["schemas"]["CredentialAttempt"] & {
            /** @enum {string} */
            code: "CredentialMalformed";
        }) | (components["schemas"]["CredentialAttempt"] & {
            /** @enum {string} */
            code: "CredentialTlsFailed";
        }) | (components["schemas"]["CredentialAttempt"] & {
            /** @enum {string} */
            code: "CredentialNotThisService";
        }) | (components["schemas"]["CredentialAttempt"] & {
            /** @enum {string} */
            code: "CredentialCollectionFailed";
        }) | (components["schemas"]["CredentialAttempt"] & {
            /** @enum {string} */
            code: "CredentialCollectionTimedOut";
        }) | (components["schemas"]["CredentialAttempt"] & {
            /** @enum {string} */
            code: "CredentialUnreachable";
        }) | (components["schemas"]["CredentialAttempt"] & {
            /** @enum {string} */
            code: "CredentialTimedOut";
        }) | {
            /** @enum {string} */
            code: "ScanTimeLimitWithEstimate";
            /** Format: int32 */
            hosts_not_scanned: number;
            /** Format: int32 */
            hours: number;
            /** Format: int32 */
            minutes_remaining: number;
        } | {
            /** @enum {string} */
            code: "ScanTimeLimit";
            /** Format: int32 */
            hosts_not_scanned: number;
            /** Format: int32 */
            hours: number;
        } | (components["schemas"]["UnmatchedNeighbour"] & {
            /** @enum {string} */
            code: "LldpNeighbourNotFound";
        }) | (components["schemas"]["UnmatchedNeighbour"] & {
            /** @enum {string} */
            code: "LldpNeighbourAmbiguous";
        }) | (components["schemas"]["UnresolvedPort"] & {
            /** @enum {string} */
            code: "LldpPortNoStrategy";
        }) | (components["schemas"]["UnresolvedPort"] & {
            /** @enum {string} */
            code: "LldpPortNotFound";
        }) | (components["schemas"]["UnresolvedPort"] & {
            /** @enum {string} */
            code: "LldpPortAmbiguous";
        }) | {
            /** @enum {string} */
            code: "WarningsTruncated";
            /** Format: int32 */
            elided: number;
        } | {
            /** @enum {string} */
            code: "Unknown";
            detail: string;
        };
        /** @description The docker install method. */
        DockerInstall: {
            /**
             * @description A ready-to-run `docker-compose.yml` for a first install. `None` for a reconfigure — the
             *     operator keeps their own compose and swaps in `env`, rather than replacing the whole file.
             */
            compose?: string | null;
            /**
             * @description The `SCANOPY_*` environment variables (`KEY=value`) this daemon is configured with. For a
             *     reconfigure these are exactly the vars that changed, so the UI can show them as a swap-in.
             */
            env: string[];
        };
        DockerVirtualization: {
            /** @description Compose project the container belongs to, when it was started by Compose. */
            compose_project?: string | null;
            /** @description Docker container ID. */
            container_id?: string | null;
            /** @description Container name as reported by Docker. */
            container_name?: string | null;
        };
        Edge: components["schemas"]["EdgeType"] & {
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            id: string;
            /** @description Whether the edge stands in for a path that crosses intermediate nodes. */
            is_multi_hop: boolean;
            /** @description Text drawn on the edge. */
            label: string | null;
            /**
             * @description Identity of the relation this edge stands for — see [`EdgeType::relation_key`]. Stamped
             *     centrally from `edge_type` once the graph is built, so no construction site can forget
             *     it. `None` marks an edge as interchangeable with its like.
             */
            relation_key: string | null;
            /**
             * Format: uuid
             * @description Node the edge starts at.
             */
            source: string;
            /** @description Which side of the source node the edge leaves from. */
            source_handle: components["schemas"]["EdgeHandle"];
            /**
             * Format: uuid
             * @description Node the edge ends at.
             */
            target: string;
            /** @description Which side of the target node the edge arrives at. */
            target_handle: components["schemas"]["EdgeHandle"];
            /** @description Per-view overrides for how this edge is drawn. */
            view_config?: components["schemas"]["EdgeViewConfig"];
        };
        /**
         * @description Whether an edge is visible by default or hidden behind a toggle
         * @enum {string}
         */
        EdgeDefaultVisibility: "visible" | "hidden";
        /** @enum {string} */
        EdgeHandle: "Top" | "Bottom" | "Left" | "Right";
        /**
         * @description Controls when an edge contributes to node highlighting on selection
         * @enum {string}
         */
        EdgeHighlightBehavior: "when_visible" | "always" | "never";
        /**
         * @description Visual stroke style for an edge
         * @enum {string}
         */
        EdgeStroke: "solid" | "dashed" | "dotted";
        /** @enum {string} */
        EdgeStyle: "Straight" | "SmoothStep" | "Bezier";
        EdgeType: {
            /** @enum {string} */
            edge_type: "SameHost";
            /**
             * Format: uuid
             * @description The host both endpoints sit on.
             */
            host_id: string;
        } | {
            /** @enum {string} */
            edge_type: "Hypervisor";
            /**
             * Format: uuid
             * @description The hypervisor service running the guest.
             */
            hypervisor_service_id: string;
        } | {
            /** @description The containerized services this edge stands for — the ones on those subnets. */
            containerized_service_ids: string[];
            /** @enum {string} */
            edge_type: "ContainerRuntime";
            /**
             * Format: uuid
             * @description The host running the container runtime.
             */
            host_id: string;
            /**
             * Format: uuid
             * @description The container runtime service itself.
             */
            service_id: string;
            /**
             * @description The bridge subnet(s) this edge reaches: one when they render as their own boxes,
             *     all of them when merged into a single box. Resolved here rather than in the
             *     inspector, which cannot tell which subnet an elevated edge landed on.
             */
            subnet_ids: string[];
        } | {
            /** @enum {string} */
            edge_type: "SameContainer";
            /**
             * Format: uuid
             * @description The containerized service reachable at several addresses.
             */
            service_id: string;
        } | {
            /**
             * Format: uuid
             * @description The dependency this edge was drawn from.
             */
            dependency_id: string;
            /** @enum {string} */
            edge_type: "RequestPath";
            /**
             * Format: uuid
             * @description Member the request starts at.
             */
            source_id: string;
            /**
             * Format: uuid
             * @description Member the request arrives at.
             */
            target_id: string;
        } | {
            /**
             * Format: uuid
             * @description The dependency this edge was drawn from.
             */
            dependency_id: string;
            /** @enum {string} */
            edge_type: "HubAndSpoke";
            /**
             * Format: uuid
             * @description The hub member.
             */
            source_id: string;
            /**
             * Format: uuid
             * @description The spoke member.
             */
            target_id: string;
        } | {
            /** @enum {string} */
            edge_type: "PhysicalLink";
            /** @description Neighbour-discovery protocol the link was learned from. */
            protocol: components["schemas"]["DiscoveryProtocol"];
            /**
             * Format: uuid
             * @description Interface at one end of the cable.
             */
            source_entity_id: string;
            /**
             * Format: uuid
             * @description Interface at the other end.
             */
            target_entity_id: string;
        } | {
            /** @enum {string} */
            edge_type: "NeighborLink";
            /** @description Neighbour-discovery protocol the adjacency was learned from. */
            protocol: components["schemas"]["DiscoveryProtocol"];
            /**
             * Format: uuid
             * @description One of the adjacent devices.
             */
            source_host_id: string;
            /**
             * Format: uuid
             * @description The other adjacent device.
             */
            target_host_id: string;
        };
        /** @enum {string} */
        EdgeTypeDiscriminants: "SameHost" | "Hypervisor" | "ContainerRuntime" | "SameContainer" | "RequestPath" | "HubAndSpoke" | "PhysicalLink" | "NeighborLink";
        /** @description Per-view configuration for an edge: disabled (not in this view) or active with properties */
        EdgeViewConfig: {
            /** @enum {string} */
            type: "disabled";
        } | {
            /** @description Whether ELK should use this edge for layout positioning */
            affects_layout: boolean;
            /** @description Whether the edge is shown by default or hidden behind a toggle */
            default_visibility: components["schemas"]["EdgeDefaultVisibility"];
            /** @description When this edge contributes to node highlighting on selection */
            highlight_behavior: components["schemas"]["EdgeHighlightBehavior"];
            /** @description Whether this edge should show directional animation when highlighted */
            show_directionality: boolean;
            /** @description Visual stroke style */
            stroke: components["schemas"]["EdgeStroke"];
            /** @enum {string} */
            type: "active";
            /**
             * @description Whether this edge should be elevated to target an accepting container
             *     instead of the element inside it
             */
            will_target_container: boolean;
        };
        ElementEntityType: {
            /** @enum {string} */
            element_type: "IPAddress";
            /**
             * Format: uuid
             * @description The IP address itself, when one is known.
             */
            ip_address_id?: string | null;
            /**
             * Format: uuid
             * @description Subnet the address sits in.
             */
            subnet_id: string;
        } | {
            /** @enum {string} */
            element_type: "Service";
        } | {
            /** @enum {string} */
            element_type: "Host";
        } | {
            /** @enum {string} */
            element_type: "Interface";
            /**
             * Format: uuid
             * @description The interface this element stands for.
             */
            interface_id: string;
        };
        /** @description Request body for emailing an install command to the authenticated user. */
        EmailInstallCommandRequest: {
            /** @description The install command to send, exactly as shown in the UI. */
            install_command: string;
            /** @description Operating system the command targets, used to pick the email wording. */
            os: components["schemas"]["DaemonOs"];
        };
        /**
         * @description Per-user toggles for the user-pausable email categories. Each field maps
         *     1:1 to a [`PausableCategory`]; required emails are never gated here.
         *
         *     Stored as a JSONB blob, so new categories are added as new fields rather
         *     than via migration. New fields carry `#[serde(default = "default_true")]`
         *     so a category is opted in by default if its key is absent from the stored
         *     JSON.
         */
        EmailSettings: {
            /** @description Send an alert when a daemon stops reporting. */
            daemon_alerts?: boolean;
            /** @description Send a periodic summary of what discovery found. */
            discovery_digest: boolean;
            /** @description Send getting-started guidance. */
            product_onboarding?: boolean;
            /** @description Send trial reminders and plan-usage warnings. */
            trial_and_usage?: boolean;
        };
        /** @description Enterprise plan inquiry request */
        EnterpriseInquiryRequest: {
            /** @description Company name */
            company: string;
            /**
             * Format: email
             * @description Contact email
             */
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
            /**
             * @description Plan the enquiry is about — the `type` tag of a `BillingPlan`
             *     (e.g. `Team`, `Business`, `Enterprise`).
             */
            plan_type?: string | null;
            /** @description Team/company size */
            team_size: components["schemas"]["TeamSize"];
            urgency?: null | components["schemas"]["InquiryTimeline"];
        };
        /** @enum {string} */
        EntityDiscriminants: "Organization" | "Invite" | "Share" | "Network" | "DaemonApiKey" | "UserApiKey" | "User" | "Tag" | "Discovery" | "Daemon" | "Host" | "Service" | "Port" | "Binding" | "IPAddress" | "Interface" | "Credential" | "Subnet" | "Vlan" | "Dependency" | "Topology" | "Snapshot" | "Unknown";
        /**
         * @description How recently discovery last observed an entity.
         *
         *     Derived, never persisted — computed from `last_seen_at` against the
         *     entity's network staleness window (`Network::stale_cutoff`). Shared by the
         *     discovery digest email and the UI so a host reported stale in the digest is
         *     the same host badged stale in the inventory and topology; running two
         *     different measures let them disagree (a scan-count measure calls an entity
         *     missing after 3 scans, which is 45 minutes on one network and 3 months on
         *     another).
         *
         *     Only discovery-managed entities can be `Stale` — see
         *     [`DiscoveryTracked::is_discovery_managed`](crate::server::shared::storage::snapshot::DiscoveryTracked::is_discovery_managed).
         * @enum {string}
         */
        EntityFreshness: "new" | "current" | "stale";
        EntitySource: {
            /** @enum {string} */
            type: "Manual";
        } | {
            /** @enum {string} */
            type: "System";
        } | {
            /** @enum {string} */
            type: "Discovery";
        } | {
            details: components["schemas"]["MatchDetails"];
            /** @enum {string} */
            type: "DiscoveryWithMatch";
        } | {
            /** @enum {string} */
            type: "Unknown";
        };
        EsxiVirtualization: {
            /** @description ESXi identifier of the guest. */
            vm_id?: string | null;
            /** @description Guest name as configured on the ESXi host. */
            vm_name?: string | null;
        };
        /** @description Non-secret value that can be inline content or a file path on daemon host. */
        FileOrInline: {
            /** @enum {string} */
            mode: "Inline";
            /** @description The value itself. */
            value: string;
        } | {
            /** @enum {string} */
            mode: "FilePath";
            /** @description Path to a file on the daemon host holding the value. */
            path: string;
        };
        /**
         * @description Request to finalize a client-confirmed SetupIntent (set the collected card
         *     as the customer's default payment method).
         */
        FinalizePaymentMethodRequest: {
            /** @description Stripe SetupIntent to attach as the organization's payment method. */
            setup_intent_id: string;
        };
        ForgotPasswordRequest: {
            /**
             * Format: email
             * @description Email address to send the password-reset link to.
             */
            email: string;
        };
        /** @description Size of one group in a grouped list, across every page. */
        GroupCount: {
            /**
             * Format: int64
             * @description How many rows fall in this group in total, not just on this page.
             */
            count: number;
            /**
             * @description The group's value, rendered as text. `null` for rows whose group key is
             *     NULL (the "ungrouped" bucket).
             */
            value?: string | null;
        };
        /**
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "credential_assignments": [],
         *       "description": "Primary web server",
         *       "first_discovery_id": null,
         *       "hidden": false,
         *       "hostname": "web-server-01.local",
         *       "id": "550e8400-e29b-41d4-a716-446655440003",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "lineage_id": null,
         *       "name": "web-server-01",
         *       "name_source": "Manual",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null,
         *       "virtualization_metadata": null,
         *       "virtualization_service_id": null
         *     }
         */
        Host: components["schemas"]["HostBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Discovery (historical row) that first observed this entity. Set once
             *     (post-terminal); immutable thereafter via the `IS NULL` guard in
             *     `update_discovery_fks`.
             */
            readonly first_discovery_id?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: uuid
             * @description Discovery (historical row) that last touched this entity. Populated
             *     post-terminal by the per-entity-service subscriber on
             *     `DiscoveryProcessed`. NULL until the first successful discovery
             *     session terminates after this row was created.
             */
            readonly last_discovery_id?: string | null;
            /**
             * Format: date-time
             * @description Last successful natural-key match by daemon discovery against this
             *     live row. Refreshed every scan, regardless of field changes.
             */
            readonly last_seen_at?: string;
            /**
             * Format: uuid
             * @description Lineage pointer on closed historical rows back to the live row whose
             *     state they capture. NULL on live rows.
             */
            readonly lineage_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description SCD2: when this row version became live. Equal to `created_at` for
             *     rows that have never ridden a snapshot; advanced to the snapshot's
             *     `taken_at` for live rows after a network snapshot fires.
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description SCD2: when this row was closed by a snapshot. NULL = currently live.
             */
            readonly valid_to?: string | null;
        };
        /**
         * @description Base data for a Host entity (stored in database).
         *     Child entities (ip_addresses, ports, services) are stored in their own tables
         *     and queried by `host_id`. They are NOT stored on the host.
         */
        HostBase: components["schemas"]["HostName"] & {
            /** @description LLDP lldpLocChassisId - globally unique device identifier for deduplication */
            chassis_id?: string | null;
            /** @description Credential assignments for this host (hydrated from junction table). */
            credential_assignments: components["schemas"]["CredentialAssignment"][];
            /** @description Free-text notes about the host. */
            description: string | null;
            /** @description Whether the host is hidden from topology views. */
            hidden: boolean;
            /** @description Hostname as resolved or reported by the host. */
            hostname: string | null;
            /**
             * Format: uri
             * @description URL for device management interface (manual or discovered)
             */
            management_url?: string | null;
            /** @description ENTITY-MIB entPhysicalMfgName - hardware manufacturer */
            manufacturer?: string | null;
            /** @description ENTITY-MIB entPhysicalModelName - hardware model */
            model?: string | null;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description ENTITY-MIB entPhysicalSerialNum - hardware serial number */
            serial_number?: string | null;
            /** @description How this host came to be known — discovered, imported, or created by hand. */
            source: components["schemas"]["EntitySource"];
            /** @description SNMP sysContact.0 - admin contact info */
            sys_contact?: string | null;
            /** @description SNMP sysDescr.0 - full system description */
            sys_descr?: string | null;
            /** @description SNMP sysLocation.0 - physical location */
            sys_location?: string | null;
            /** @description SNMP sysName.0 - administratively-assigned hostname */
            sys_name?: string | null;
            /** @description SNMP sysObjectID.0 - vendor OID for device identification */
            sys_object_id?: string | null;
            /** @description Tags assigned to this entity. */
            tags: string[];
            virtualization_metadata: null | components["schemas"]["HostVirtualization"];
            /**
             * Format: uuid
             * @description The service doing the virtualizing — the hypervisor this VM runs on.
             *
             *     Its own column with a foreign key rather than a field inside
             *     [`HostVirtualization`]: a reference that no longer resolves now fails the write instead of
             *     surviving as a value nothing matches, and `ON DELETE SET NULL` clears it when the
             *     hypervisor service goes away (GH #650).
             */
            virtualization_service_id: string | null;
        };
        HostName: {
            /** @description Human-facing name for the host. */
            name: string;
            name_source?: components["schemas"]["HostNameSource"];
        };
        /** @enum {string} */
        HostNameSource: "Unnamed" | "Unspecified" | "Ip" | "DetectedService" | "Hostname" | "DnsSd" | "Integration" | "Manual";
        /** @enum {string} */
        HostNamingFallback: "Ip" | "BestService";
        /**
         * @description Fields that hosts can be ordered/grouped by.
         * @enum {string}
         */
        HostOrderField: "created_at" | "name" | "hostname" | "updated_at" | "virtualized_by" | "network_id" | "interface_ip" | "last_seen_at";
        /**
         * @description Response type for host endpoints.
         *     Includes children (ip_addresses, ports, services, interfaces).
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "credential_assignments": [],
         *       "description": "Primary web server",
         *       "hidden": false,
         *       "hostname": "web-server-01.local",
         *       "id": "550e8400-e29b-41d4-a716-446655440003",
         *       "interfaces": [
         *         {
         *           "admin_status": "Up",
         *           "cdp_address": null,
         *           "cdp_device_id": null,
         *           "cdp_platform": null,
         *           "cdp_port_id": null,
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "first_discovery_id": null,
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-44665544000f",
         *           "if_alias": "Uplink to Core Switch",
         *           "if_descr": "GigabitEthernet0/1",
         *           "if_index": 1,
         *           "if_name": "Gi0/1",
         *           "if_type": 6,
         *           "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
         *           "last_discovery_id": null,
         *           "last_seen_at": "2026-01-15T10:30:00Z",
         *           "lineage_id": null,
         *           "lldp_chassis_id": null,
         *           "lldp_mgmt_addr": null,
         *           "lldp_port_desc": null,
         *           "lldp_port_id": null,
         *           "lldp_sys_desc": null,
         *           "lldp_sys_name": null,
         *           "mac_address": "DE:AD:BE:EF:CA:FE",
         *           "neighbor": null,
         *           "neighbor_seen_at": null,
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "oper_status": "Up",
         *           "speed_bps": 1000000000,
         *           "updated_at": "2026-01-15T10:30:00Z",
         *           "valid_from": "2026-01-15T10:30:00Z",
         *           "valid_to": null
         *         }
         *       ],
         *       "ip_addresses": [
         *         {
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "first_discovery_id": null,
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-446655440005",
         *           "ip_address": "192.168.1.100",
         *           "last_discovery_id": null,
         *           "last_seen_at": "2026-01-15T10:30:00Z",
         *           "lineage_id": null,
         *           "mac_address": "DE:AD:BE:EF:CA:FE",
         *           "name": "eth0",
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "position": 0,
         *           "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
         *           "updated_at": "2026-01-15T10:30:00Z",
         *           "valid_from": "2026-01-15T10:30:00Z",
         *           "valid_to": null
         *         }
         *       ],
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "name": "web-server-01",
         *       "name_source": "Manual",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "ports": [
         *         {
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "first_discovery_id": null,
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-446655440006",
         *           "last_discovery_id": null,
         *           "last_seen_at": "2026-01-15T10:30:00Z",
         *           "lineage_id": null,
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "number": 80,
         *           "protocol": "Tcp",
         *           "type": "Http",
         *           "updated_at": "2026-01-15T10:30:00Z",
         *           "valid_from": "2026-01-15T10:30:00Z",
         *           "valid_to": null
         *         }
         *       ],
         *       "services": [
         *         {
         *           "bindings": [
         *             {
         *               "created_at": "2026-08-25T22:05:35.390444Z",
         *               "first_discovery_id": null,
         *               "id": "6ef8015e-ce2f-4678-87c6-3590b0267165",
         *               "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
         *               "last_discovery_id": null,
         *               "last_seen_at": "2026-08-25T22:05:35.390444Z",
         *               "lineage_id": null,
         *               "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *               "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *               "service_id": "550e8400-e29b-41d4-a716-446655440007",
         *               "type": "Port",
         *               "updated_at": "2026-08-25T22:05:35.390444Z",
         *               "valid_from": "2026-08-25T22:05:35.390444Z",
         *               "valid_to": null
         *             }
         *           ],
         *           "created_at": "2026-01-15T10:30:00Z",
         *           "first_discovery_id": null,
         *           "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *           "id": "550e8400-e29b-41d4-a716-446655440007",
         *           "last_discovery_id": null,
         *           "last_seen_at": "2026-01-15T10:30:00Z",
         *           "lineage_id": null,
         *           "name": "nginx",
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "position": 0,
         *           "service_definition": "Google Home",
         *           "source": {
         *             "type": "Manual"
         *           },
         *           "tags": [],
         *           "updated_at": "2026-01-15T10:30:00Z",
         *           "valid_from": "2026-01-15T10:30:00Z",
         *           "valid_to": null,
         *           "virtualization_metadata": null,
         *           "virtualization_service_id": null
         *         }
         *       ],
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "virtualization_metadata": null,
         *       "virtualization_service_id": null
         *     }
         */
        HostResponse: {
            /** @description LLDP chassis identifier, used to match the host to its neighbours. */
            chassis_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            created_at: string;
            /** @description Credentials assigned to scan this host. */
            credential_assignments?: components["schemas"]["CredentialAssignment"][];
            /** @description Free-text notes about the host. */
            description?: string | null;
            /** @description Whether the host is hidden from topology views. */
            hidden: boolean;
            /** @description Hostname as resolved or reported by the host. */
            hostname?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            id: string;
            /** @description SNMP ifTable entries */
            interfaces: components["schemas"]["Interface"][];
            /** @description IP addresses on this host. */
            ip_addresses: components["schemas"]["IPAddress"][];
            /**
             * Format: date-time
             * @description Last time discovery observed this host. User-facing (drives the "Last
             *     seen" column and the stale badge), which is why it is carried here while
             *     the rest of the SCD2/audit columns are not.
             */
            last_seen_at: string;
            /** @description Link to the host's own management interface. */
            management_url?: string | null;
            /** @description ENTITY-MIB entPhysicalMfgName — hardware manufacturer. Read-only, as above. */
            readonly manufacturer?: string | null;
            /** @description ENTITY-MIB entPhysicalModelName — hardware model. Read-only, as above. */
            readonly model?: string | null;
            /** @description Human-facing name for the host. */
            name: string;
            /**
             * @description Which rung of the naming ladder produced `name`. Read-only: it is decided by whoever
             *     supplied the name, not by the caller.
             */
            name_source?: components["schemas"]["HostNameSource"];
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description Open ports on this host. */
            ports: components["schemas"]["Port"][];
            /** @description ENTITY-MIB entPhysicalSerialNum — hardware serial number. Read-only, as above. */
            readonly serial_number?: string | null;
            /** @description Services running on this host. */
            services: components["schemas"]["Service"][];
            /** @description How this host came to be known — discovered, imported, or created by hand. */
            source: components["schemas"]["EntitySource"];
            /** @description SNMP sysContact — administrative contact as configured on the device. */
            sys_contact?: string | null;
            /** @description SNMP sysDescr — the device's own description of itself. */
            sys_descr?: string | null;
            /** @description SNMP sysLocation — physical location as configured on the device. */
            sys_location?: string | null;
            /**
             * @description SNMP sysName.0 — the administratively-assigned hostname. Read-only: discovery collects it
             *     from the device, so neither create nor update accepts it.
             */
            readonly sys_name?: string | null;
            /** @description SNMP sysObjectID — the vendor's identifier for the device model. */
            sys_object_id?: string | null;
            /** @description Tags assigned to this entity. */
            tags: string[];
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            updated_at: string;
            virtualization_metadata?: null | components["schemas"]["HostVirtualization"];
            /**
             * Format: uuid
             * @description The hypervisor service this VM runs on.
             */
            virtualization_service_id?: string | null;
        };
        /** HostVirtualization */
        HostVirtualization: {
            details: components["schemas"]["ProxmoxVirtualization"];
            /** @enum {string} */
            type: "Proxmox";
        } | {
            details: components["schemas"]["VCenterVirtualization"];
            /** @enum {string} */
            type: "VCenter";
        } | {
            details: components["schemas"]["EsxiVirtualization"];
            /** @enum {string} */
            type: "ESXi";
        };
        /**
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "first_discovery_id": null,
         *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *       "id": "550e8400-e29b-41d4-a716-446655440005",
         *       "ip_address": "192.168.1.100",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "lineage_id": null,
         *       "mac_address": "DE:AD:BE:EF:CA:FE",
         *       "name": "eth0",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "position": 0,
         *       "subnet_id": "550e8400-e29b-41d4-a716-446655440004",
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null
         *     }
         */
        IPAddress: components["schemas"]["IPAddressBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description The discovery that first observed this entity.
             */
            readonly first_discovery_id?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: uuid
             * @description The most recent discovery that observed this entity.
             */
            readonly last_discovery_id?: string | null;
            /**
             * Format: date-time
             * @description When a discovery last observed this entity.
             */
            readonly last_seen_at?: string;
            /**
             * Format: uuid
             * @description Stable identifier shared by every revision of the same entity across its history.
             */
            readonly lineage_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description Start of the interval this revision was current for (SCD2 history).
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description End of the interval this revision was current for. `null` while it is the live revision.
             */
            readonly valid_to?: string | null;
        };
        IPAddressBase: {
            /**
             * Format: uuid
             * @description The host this entity belongs to.
             */
            host_id: string;
            /**
             * @description IPv4 or IPv6 address.
             * @example 192.168.1.10
             */
            ip_address: string;
            /**
             * @description MAC address discovered from ARP, SNMP, or Docker - immutable once set
             * @example a4:bb:6d:12:34:56
             */
            mac_address?: string | null;
            /** @description Human-facing name for this IP address. */
            name: string | null;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /**
             * Format: int32
             * @description Position of this IP address in the host's IP address list (for ordering)
             */
            position?: number;
            /**
             * Format: uuid
             * @description The subnet this entity belongs to.
             */
            subnet_id: string;
        };
        /**
         * @description Input for creating or updating an interface.
         *     Used in both CreateHostRequest and UpdateHostRequest.
         *     Client must provide a UUID for the interface.
         */
        IPAddressInput: {
            /**
             * Format: uuid
             * @description Client-provided UUID for this interface
             */
            id: string;
            /**
             * @description IPv4 or IPv6 address.
             * @example 192.168.1.10
             */
            ip_address: string;
            /**
             * @description MAC address, when known.
             * @example a4:bb:6d:12:34:56
             */
            mac_address?: string | null;
            /** @description Human-facing name for this IP address. */
            name?: string | null;
            /**
             * Format: int32
             * @description Position in the host's interface list (for ordering).
             *     If omitted on create: appends to end of list.
             *     If omitted on update: existing ip_addresses keep their positions; new ip_addresses append.
             *     Must be all specified or all omitted across all ip_addresses in the request.
             */
            position?: number | null;
            /**
             * Format: uuid
             * @description The subnet this entity belongs to.
             */
            subnet_id: string;
        };
        /** @description Generic wrapper that gives any rule type a stable UUID identity. */
        IdentifiedRule_ContainerRule: {
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            id: string;
            /**
             * @description Rules that change which containers exist and how they nest.
             *     Container titles are data-driven (subnet CIDR, host names), not user-configurable.
             */
            rule: "BySubnet" | "MergeContainerBridges" | {
                /** @description One container per application tag. */
                ByApplication: {
                    /** @description Application tags to draw containers for. Empty means every application tag. */
                    tag_ids?: string[];
                };
            } | "ByHost";
        };
        /** @description Generic wrapper that gives any rule type a stable UUID identity. */
        IdentifiedRule_ElementRule: {
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            id: string;
            /** @description Rules that organize nodes within a container into sub-groups. */
            rule: {
                /** @description One subcontainer per group of service categories. */
                ByServiceCategory: {
                    /** @description Service categories to group into this subcontainer. */
                    categories: components["schemas"]["ServiceCategory"][];
                    /**
                     * @description Set by the backend on the default infrastructure rule.
                     *     Frontend uses this to identify the infra container for auto-collapse.
                     */
                    readonly is_infra_rule?: boolean;
                    /** @description Heading for the subcontainer. Defaults to the category name. */
                    title?: string | null;
                };
            } | {
                /** @description One subcontainer per group of tags. */
                ByTag: {
                    /** @description Tags to group into this subcontainer. */
                    tag_ids: string[];
                    /** @description Heading for the subcontainer. Defaults to the tag name. */
                    title?: string | null;
                };
            } | "ByHypervisor" | "ByContainerRuntime" | "ByStack" | "ByTrunkPort" | "ByVLAN" | "ByPortOpStatus";
        };
        /**
         * @description SNMP ifAdminStatus values per IF-MIB RFC 2863
         * @enum {string}
         */
        IfAdminStatus: "Up" | "Down" | "Testing";
        /**
         * @description SNMP ifOperStatus values per IF-MIB RFC 2863
         * @enum {string}
         */
        IfOperStatus: "Up" | "Down" | "Testing" | "Unknown" | "Dormant" | "NotPresent" | "LowerLayerDown";
        /**
         * @description Visual grouping metadata for inlined entities.
         *     Entities sharing the same `group_id` are rendered together in the element card.
         */
        InlineGroup: {
            /**
             * Format: uuid
             * @description The inlined entity's ID (e.g., service ID).
             */
            entity_id: string;
            /**
             * Format: uuid
             * @description Shared by all members of the visual group.
             */
            group_id: string;
            /** @description Whether this entity heads the inline group or is a member of it. */
            role: components["schemas"]["InlineGroupRole"];
        };
        /**
         * @description Role of an inlined entity within its visual group.
         * @enum {string}
         */
        InlineGroupRole: "Header" | "Member";
        /**
         * @description How soon the enquirer wants to move.
         * @enum {string}
         */
        InquiryTimeline: "immediately" | "1-3 months" | "3-6 months" | "exploring";
        /**
         * @description Everything the UI needs to install (or reconfigure) a daemon, one field per install method so
         *     each is a first-class peer with its own content — no method is a special case bolted onto a
         *     list. The binary methods are ready-to-paste commands (any api key is the [`API_KEY_PLACEHOLDER`],
         *     filled in client-side); docker and msi carry their own structured content.
         */
        InstallArtifacts: {
            /** @description Container image reference. */
            docker: components["schemas"]["DockerInstall"];
            /** @description Download for FreeBSD. */
            freebsd: string;
            /** @description Download for Linux. */
            linux: string;
            /** @description Download for macOS. */
            macos: string;
            /** @description Windows installer package. */
            msi: components["schemas"]["MsiInstall"];
            /** @description Download for Windows. */
            windows: string;
        };
        /**
         * @description What the caller wants the command to do — the one axis that actually varies.
         *
         *     `install` brings a daemon up (or re-keys a legacy one): it carries the api-key placeholder,
         *     fetches the binary, and spells out the connectivity + advanced config. `reconfigure` adjusts
         *     an already-installed daemon in place: no key, no fetch, just the server-held connectivity —
         *     `scanopy-daemon install` layers it over the existing `config.json`. There is no third case:
         *     re-asserting the record's (correct) values on an installed daemon is harmless, so a first
         *     install and a re-key are the same command.
         * @enum {string}
         */
        InstallCommandKind: "install" | "reconfigure";
        /**
         * @description Per-daemon integration targeting, stored on the `Discovery` entity and delivered via the
         *     init command at registration. Each entry references exactly one stored credential and says
         *     where it applies on this daemon. This is the single home for cred↔IP targeting — it replaces
         *     the global, race-prone `credential.target_ips`.
         *
         *     The variants ARE the scopes; their strum [`Target`] discriminants are the capability enum that
         *     `CredentialType::targets()` returns and validates against (single source of truth). Every
         *     target carries a real `credential_id` — there is no credential-less branch and no nil
         *     sentinel; a local socket is just a credential whose type targets only the daemon host.
         */
        IntegrationTarget: {
            /**
             * Format: uuid
             * @description Credential to use on the daemon host.
             */
            credential_id: string;
            /** @enum {string} */
            scope: "DaemonHost";
        } | {
            /**
             * Format: uuid
             * @description Credential to use across the network.
             */
            credential_id: string;
            /** @enum {string} */
            scope: "Network";
        } | {
            /**
             * Format: uuid
             * @description Credential to use on the listed addresses.
             */
            credential_id: string;
            /** @description The host addresses this credential applies to. */
            ips: string[];
            /** @enum {string} */
            scope: "Hosts";
        };
        Interface: components["schemas"]["InterfaceBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description The discovery that first observed this entity.
             */
            readonly first_discovery_id?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: uuid
             * @description The most recent discovery that observed this entity.
             */
            readonly last_discovery_id?: string | null;
            /**
             * Format: date-time
             * @description When a discovery last observed this entity.
             */
            readonly last_seen_at?: string;
            /**
             * Format: uuid
             * @description Stable identifier shared by every revision of the same entity across its history.
             */
            readonly lineage_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description Start of the interval this revision was current for (SCD2 history).
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description End of the interval this revision was current for. `null` while it is the live revision.
             */
            readonly valid_to?: string | null;
        };
        InterfaceBase: {
            /** @description SNMP ifAdminStatus: 1=up, 2=down, 3=testing */
            admin_status: components["schemas"]["IfAdminStatus"];
            /**
             * @description Remote management IP from CDP (cdpCacheAddress). IPv4 or IPv6.
             * @example 192.168.1.1
             */
            cdp_address?: string | null;
            /** @description Remote device ID from CDP (typically hostname, locally unique) */
            cdp_device_id?: string | null;
            /** @description Remote platform from CDP (e.g., "Cisco IOS") */
            cdp_platform?: string | null;
            /** @description Remote port ID from CDP */
            cdp_port_id?: string | null;
            /**
             * @description Bridge FDB: learned MAC addresses on this switch port.
             *     Single-MAC ports can be resolved to neighbor links server-side.
             *     Multi-MAC ports indicate uplinks where LLDP/CDP is the better source.
             */
            fdb_macs?: string[] | null;
            /**
             * Format: uuid
             * @description The host this entity belongs to.
             */
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
            /** @description SNMP ifName - short interface name (e.g., Gi1/0/1) */
            if_name?: string | null;
            /**
             * Format: int32
             * @description SNMP ifType - IANAifType integer (6=ethernet, 24=loopback, etc.)
             */
            if_type: number;
            /**
             * Format: uuid
             * @description FK to IPAddress entity - this port's IP assignment (must be on same host).
             *     Old daemons send this as "interface_id".
             */
            ip_address_id?: string | null;
            lldp_chassis_id?: null | components["schemas"]["LldpChassisId"];
            /**
             * @description Remote management IP from LLDP neighbor (lldpRemManAddr). IPv4 or IPv6.
             * @example 192.168.1.1
             */
            lldp_mgmt_addr?: string | null;
            /** @description Remote port description from LLDP neighbor (lldpRemPortDesc) */
            lldp_port_desc?: string | null;
            lldp_port_id?: null | components["schemas"]["LldpPortId"];
            /** @description Remote system description from LLDP neighbor (lldpRemSysDesc) - platform info */
            lldp_sys_desc?: string | null;
            /** @description Remote system name from LLDP neighbor (lldpRemSysName) */
            lldp_sys_name?: string | null;
            /**
             * @description MAC address from SNMP ifPhysAddress - immutable once set
             * @example a4:bb:6d:12:34:56
             */
            mac_address?: string | null;
            /**
             * Format: uuid
             * @description Native/untagged VLAN entity ID on this port (resolved from Q-BRIDGE dot1qPvid)
             */
            native_vlan_id?: string | null;
            neighbor?: null | components["schemas"]["Neighbor"];
            /**
             * Format: date-time
             * @description When a scan last carried evidence that something is adjacent to this port.
             *
             *     The freshness subject for the *link*, as `last_seen_at` is for the port. A port keeps
             *     appearing in the ifTable long after its neighbour record stops arriving, so `last_seen_at`
             *     cannot tell a live adjacency from one whose evidence has vanished. Judged against the same
             *     `Network::stale_cutoff` as every other freshness verdict.
             *
             *     `None` means no scan has ever carried evidence for this row, and reads as *unknown* —
             *     never as stale. Server-owned: stamped on the discovery ingest path, never sent by a daemon.
             */
            readonly neighbor_seen_at?: string | null;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description SNMP ifOperStatus: 1=up, 2=down, 3=testing, 4=unknown, 5=dormant, 6=notPresent, 7=lowerLayerDown */
            oper_status: components["schemas"]["IfOperStatus"];
            /**
             * Format: int64
             * @description Interface speed from ifSpeed/ifHighSpeed in bits per second
             */
            speed_bps?: number | null;
            /** @description Tagged VLAN entity IDs on this port (resolved from Q-BRIDGE dot1qVlanCurrentEgressPorts) */
            vlan_ids?: string[] | null;
        };
        /**
         * @description Which groups of per-interface data the daemon read in full during a scan.
         *
         *     Each group comes from its own SNMP walk, and a walk cut short by a timeout yields exactly the
         *     same empty result as a device that genuinely has nothing to report. Without knowing which
         *     happened, the server overwrote good data with NULL on every truncation — and for the neighbour
         *     fields that also dropped the row out of L2 resolution permanently, since the resolution filter
         *     requires a chassis id or CDP device id to be present.
         *
         *     Every field defaults to `true`, so a daemon predating this behaves exactly as before: it
         *     reports everything as authoritative and the server overwrites.
         */
        InterfaceDataComplete: {
            /** @description `cdp_device_id`, `cdp_port_id`, `cdp_platform`, `cdp_address` */
            cdp?: boolean;
            /** @description `fdb_macs` */
            fdb?: boolean;
            /**
             * @description `lldp_chassis_id`, `lldp_port_id`, `lldp_sys_name`, `lldp_port_desc`, `lldp_mgmt_addr`,
             *     `lldp_sys_desc`
             */
            lldp?: boolean;
            /** @description `native_vlan_id`, `vlan_ids` */
            vlan_membership?: boolean;
        };
        /**
         * @description Input for creating an SNMP interface entry (ifTable data).
         *     Used in CreateHostRequest. Server assigns UUIDs since nothing references
         *     Interface IDs at creation time (neighbor resolution is done server-side).
         */
        InterfaceInput: {
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
            ip_address_id?: string | null;
            /**
             * @description MAC address from SNMP ifPhysAddress
             * @example a4:bb:6d:12:34:56
             */
            mac_address?: string | null;
            oper_status?: null | components["schemas"]["IfOperStatus"];
            /**
             * Format: int64
             * @description Interface speed in bits per second
             */
            speed_bps?: number | null;
        };
        Invite: components["schemas"]["InviteBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        InviteBase: {
            /**
             * Format: uuid
             * @description User who sent the invite.
             */
            created_by: string;
            /**
             * Format: date-time
             * @description When this record stops being valid.
             */
            expires_at: string;
            /** @description The networks this entity applies to. */
            network_ids: string[];
            /**
             * Format: uuid
             * @description The organization that owns this record.
             */
            organization_id: string;
            /** @description Role the invited user gets on acceptance. */
            permissions: components["schemas"]["UserOrgPermissions"];
            /** @description Optional email address to send the invite to */
            send_to: string | null;
            /** @description Link the recipient follows to accept the invite. */
            url: string;
        };
        Ixy: {
            /** @description Horizontal position, which may be negative. */
            x: number;
            /** @description Vertical position, which may be negative. */
            y: number;
        };
        /**
         * @description Legacy inbound-only capabilities blob.
         *
         *     Pre-0.15 daemons report their interfaced subnets as bare `subnet_id`s in this
         *     `capabilities` object (they predate the `interfaced_subnets: Vec<Subnet>`
         *     heartbeat channel). It is deserialize-only: the server never stores it, never
         *     echoes it in `DaemonResponse`, and it has no `SqlValue` variant. Reported ids
         *     are routed into the `daemon_interfaced_subnets` junction (existence-filtered)
         *     so legacy daemons keep reporting interfaced subnets. ≥0.15 daemons send the
         *     `Vec<Subnet>` channel instead and leave this empty.
         */
        LegacyCapabilities: {
            /** @description Subnets the daemon has an interface on, as reported by older daemons. */
            interfaced_subnet_ids: string[];
        };
        /**
         * @description Runtime license state as reported by the public config endpoint.
         * @enum {string}
         */
        LicenseStatusDiscriminants: "valid" | "expired" | "invalid";
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
            /**
             * Format: email
             * @description Email address of the account to sign in to.
             */
            email: string;
            /**
             * Format: password
             * @description The account password.
             */
            password: string;
        };
        /**
         * @description What discarding a device's malformed neighbour records cost it.
         *
         *     A slot value rather than two codes per reason: losing every link and losing some of them is a
         *     difference in severity, not in failure mode, and the metric asks about mode. Splitting it into
         *     codes would double the enum to say something the operator reads in one clause.
         * @enum {string}
         */
        MalformedNeighbourConsequence: "AllLinksLost" | "SomeLinksLost";
        /** @description Neighbour records discarded for want of the identifier that matches the far end. */
        MalformedNeighbours: {
            address: string;
            consequence: components["schemas"]["MalformedNeighbourConsequence"];
            /** Format: int32 */
            discarded: number;
            group: components["schemas"]["SnmpWalkGroup"];
            /**
             * Format: int32
             * @description Records that survived, which is what decides whether this cost the device some of its
             *     topology or all of it.
             */
            kept: number;
        };
        /** @enum {string} */
        MatchConfidence: "NotApplicable" | "Low" | "Medium" | "High" | "Certain";
        MatchDetails: {
            /** @description How strong the match is. */
            confidence: components["schemas"]["MatchConfidence"];
            /** @description Why the service was matched to this definition. */
            reason: components["schemas"]["MatchReason"];
        };
        /** @description Match reason - either a simple reason string or a container with nested reasons */
        MatchReason: {
            /** @description Why the service was matched. */
            data: string;
            /** @enum {string} */
            type: "reason";
        } | {
            /** @description Tuple of [name: string, children: MatchReason[]] */
            data: unknown[];
            /** @enum {string} */
            type: "container";
        };
        /**
         * @description The Windows MSI install method. The MSI itself is a static release asset the UI links to; only
         *     the per-daemon pre-fill data is tenant-specific.
         */
        MsiInstall: {
            /**
             * @description Filename encoding this daemon's non-secret config. Save or rename the downloaded MSI to
             *     this name to pre-fill the installer — parse-filename.js decodes it. The api key is never
             *     encoded. Renaming a signed MSI doesn't affect its signature.
             */
            filename: string;
            /**
             * @description Config keys that did not fit in `filename` (a filename is capped at 255 characters). Empty
             *     for any ordinary config. The MSI falls back to its built-in defaults for these, so the UI
             *     should tell the user to set them in the installer — the other methods carry the full config.
             */
            omitted_config_keys: string[];
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
            type: "Interface";
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
         *       "credential_ids": [],
         *       "effective_stale_after_hours": 672,
         *       "id": "550e8400-e29b-41d4-a716-446655440002",
         *       "name": "Home Network",
         *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
         *       "stale_after_hours": null,
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z"
         *     }
         */
        Network: components["schemas"]["NetworkBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: int64
             * @description `stale_after_hours` with the server's default already applied.
             *
             *     Computed, never stored (excluded from `to_params`). Published so the
             *     frontend derives staleness from the *same* number the digest uses rather
             *     than re-declaring the default in TypeScript, where the two could drift
             *     and a host could read stale in the app but current in the digest email.
             */
            readonly effective_stale_after_hours?: number;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        NetworkBase: {
            /** @description Credential IDs associated with this network (hydrated from junction table). */
            credential_ids: string[];
            /** @description Human-facing name for this network. */
            name: string;
            /**
             * Format: uuid
             * @description The organization that owns this record.
             */
            organization_id: string;
            /**
             * Format: int64
             * @description How long a discovery-managed entity on this network may go unobserved
             *     before it reads as stale. `None` = unset; callers resolve the effective
             *     value through [`Network::stale_after`], never by reading this directly.
             *
             *     Network-scoped because staleness is only meaningful relative to scan
             *     cadence, and cadence is a property of a network's discoveries.
             */
            stale_after_hours: number | null;
            /** @description Tags assigned to this entity. */
            tags: string[];
        };
        /** @description Network configuration for setup */
        NetworkSetup: {
            /** @description Name for the network created during setup. */
            name: string;
        };
        /** @description Per-network summary of entity counts */
        NetworkSummary: {
            /**
             * Format: int64
             * @description Daemons assigned to this network.
             */
            daemon_count: number;
            /**
             * Format: int64
             * @description Hosts currently discovered on this network.
             */
            host_count: number;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            id: string;
            /** @description Name of the network. */
            name: string;
            /**
             * Format: int64
             * @description Services currently discovered on this network.
             */
            service_count: number;
            /**
             * Format: int64
             * @description Subnets currently known on this network.
             */
            subnet_count: number;
        };
        Node: components["schemas"]["NodeType"] & {
            /** @description Heading drawn at the top of a container node. */
            header?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            id: string;
            /** @description Where the node sits in the layout. */
            position: components["schemas"]["Ixy"];
            /** @description Width and height of the node. */
            size: components["schemas"]["Uxy"];
        };
        NodeType: {
            /**
             * @description Service definition ID for logo rendering (e.g. "Docker", "Proxmox VE").
             *     Used by Hypervisor and Stack subcontainers to show the service's logo.
             */
            associated_service_definition?: string | null;
            color?: null | components["schemas"]["Color"];
            /** @description What this container groups — a host, a subnet, an application, and so on. */
            container_type?: components["schemas"]["ContainerType"];
            /**
             * Format: uuid
             * @description ID of the element rule that created this container (for subcontainers like NestedTag, Hypervisor, etc.)
             */
            element_rule_id?: string | null;
            /**
             * Format: uuid
             * @description The entity this container represents (e.g. host ID for Host containers,
             *     subnet ID for Subnet containers). Used for ownership mapping on the frontend.
             */
            entity_id?: string | null;
            /** @description Display icon name (set by graph builder from the source entity, e.g. subnet type) */
            icon?: string | null;
            /** @enum {string} */
            node_type: "Container";
            /**
             * Format: uuid
             * @description Container this one nests inside, for subcontainers.
             */
            parent_container_id?: string | null;
            /**
             * @description When true, this container accepts edges with `will_target_container`, causing
             *     them to visually attach here instead of at elements inside.
             */
            will_accept_edges?: boolean;
        } | (components["schemas"]["ElementEntityType"] & {
            /**
             * Format: uuid
             * @description Container this element is drawn inside.
             */
            container_id?: string;
            /**
             * Format: uuid
             * @description Host the element belongs to.
             */
            host_id: string;
            /**
             * @description Visual grouping metadata for services inlined on this element.
             *     Populated by element rules (e.g., Docker containers on a VM host
             *     get InlineGroups with Header/Member roles for dotted-border rendering).
             */
            inline_groups?: components["schemas"]["InlineGroup"][];
        } & {
            /** @enum {string} */
            node_type: "Element";
        });
        OidcProviderMetadata: {
            /** @description Logo to show on the login button, when the provider has one configured. */
            logo?: string | null;
            /** @description Display name of the identity provider, shown on the login button. */
            name: string;
            /** @description URL-safe identifier used in the provider's login and link endpoints. */
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
        };
        /** @enum {string} */
        OnboardingOperationDiscriminants: "OrgCreated" | "OnboardingModalCompleted" | "PlanSelected" | "DaemonPromptDismissed" | "DaemonPromptAccepted" | "FirstDaemonRegistered" | "FirstTopologyRebuild" | "FirstDiscoveryCompleted" | "FirstHostDiscovered" | "SecondNetworkCreated" | "FirstTagCreated" | "FirstDependencyCreated" | "FirstUserApiKeyCreated" | "FirstSnmpCredentialCreated" | "FirstApplicationTagCreated" | "FirstCredentialCreated" | "FirstSnapshotCreated" | "InviteSent" | "InviteAccepted" | "ProfileCompleted" | "ReferralSourceCompleted";
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
            use_case?: null | components["schemas"]["UseCase"];
        };
        /** @description Request to save onboarding step */
        OnboardingStepRequest: {
            /** @description Identifier of the onboarding step the user has reached. */
            step: string;
            use_case?: null | components["schemas"]["UseCase"];
        };
        /**
         * @description Direction for ORDER BY clauses.
         * @enum {string}
         */
        OrderDirection: "asc" | "desc";
        Organization: components["schemas"]["OrganizationBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        OrganizationBase: {
            /**
             * Format: date-time
             * @description When the currently-active save-offer discount window expires. The
             *     BillingTab chip renders only while `> now()`; expiry needs no
             *     cleanup job.
             */
            readonly discount_save_offer_active_until?: string | null;
            /**
             * Format: int64
             * @description Percent off the currently-active save-offer discount applies. Read
             *     live by the BillingTab chip so a future coupon swap renders the new
             *     value without a code change.
             */
            readonly discount_save_offer_percent_off?: number | null;
            /** @description Whether a payment method is on file. */
            readonly has_payment_method?: boolean;
            /**
             * Format: date-time
             * @description Most recent save-offer-discount application. NULL = never. Drives the
             *     once-per-org eligibility check in `apply_discount_save_offer` and
             *     hides the Discount panel on the cancel modal for any return visit.
             */
            readonly last_discount_at?: string | null;
            /**
             * Format: date-time
             * @description Most recent downgrade event timestamp (paid→cheaper, or paid→cancelled);
             *     powers the 14-day downgrade banner.
             */
            readonly last_downgrade_at?: string | null;
            last_downgrade_from_plan?: null | components["schemas"]["BillingPlan"];
            /**
             * Format: date-time
             * @description Most recent `Paused` billing event's timestamp; powers the 6-month
             *     rolling pause cooldown.
             */
            readonly last_paused_at?: string | null;
            /** @description Human-facing name for this organization. */
            name: string;
            /**
             * Format: date-time
             * @description Stripe `subscription.items.data[0].current_period_end`, mirrored on
             *     every billing event that re-anchors the period (checkout, trial start
             *     / end, plan change, renewal, pause/resume, reactivate). Cleared by
             *     SubscriptionCancelled. Powers the "Next renewal on …" line in
             *     BillingPlanModal; the UI interprets the value based on plan_status
             *     (hide for paused/cancelled/past_due where the stored value can be
             *     stale or meaningless).
             */
            readonly next_renewal_at?: string | null;
            /** @description Progress through first-run setup. */
            onboarding: components["schemas"]["OnboardingOperationDiscriminants"][];
            plan: null | components["schemas"]["BillingPlan"];
            plan_status: null | components["schemas"]["PlanStatus"];
            /**
             * Format: date-time
             * @description When the free trial ends, if one is running.
             */
            readonly trial_end_date?: string | null;
            /** @description Whether the org has used its one-time trial-extend perk. */
            readonly trial_extended_used?: boolean;
            /** @description Use case selection (homelab, company, msp, other) */
            use_case?: components["schemas"]["UseCase"];
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
         *       "server_version": "0.17.12"
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
             * @example 0.17.12
             */
            server_version: string;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Credential: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["CredentialBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_DaemonResponse: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["DaemonBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                id: string;
                /**
                 * @description Subnets this daemon has interfaces on, loaded from the
                 *     `daemon_interfaced_subnets` junction (replaces the old
                 *     `capabilities.interfaced_subnet_ids` JSONB field).
                 */
                interfaced_subnet_ids: string[];
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                updated_at: string;
                /** @description Computed version status including health and warnings */
                version_status: components["schemas"]["DaemonVersionStatus"];
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Dependency: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["DependencyBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Discovery: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["DiscoveryBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /** @description When true, the next scan will be a full port scan regardless of interval */
                force_full_scan?: boolean;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * @description Per-daemon integration targeting: which integrations run on this daemon, and on which
                 *     IPs. Delivered via the init command at registration and editable via the discovery
                 *     modal. This is the single home for cred↔IP targeting; it replaces the global
                 *     `credential.target_ips` (race-prone, consumed once).
                 *
                 *     One-shot: a target is offered to the daemon until a scan completes successfully, then
                 *     dropped by [`Discovery::apply_successful_scan`]. Credentials that earned a durable home
                 *     during the scan keep being retried from there — `host_credentials` for one that probed
                 *     successfully, `network_credentials` for a broadcast one (see
                 *     [`Discovery::take_network_scope_credential_ids`]).
                 */
                integration_targets: components["schemas"]["IntegrationTarget"][];
                /**
                 * Format: int32
                 * @description Number of completed scans (incremented by server on session completion)
                 */
                readonly scan_count?: number;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_HostResponse: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: {
                /** @description LLDP chassis identifier, used to match the host to its neighbours. */
                chassis_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                created_at: string;
                /** @description Credentials assigned to scan this host. */
                credential_assignments?: components["schemas"]["CredentialAssignment"][];
                /** @description Free-text notes about the host. */
                description?: string | null;
                /** @description Whether the host is hidden from topology views. */
                hidden: boolean;
                /** @description Hostname as resolved or reported by the host. */
                hostname?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                id: string;
                /** @description SNMP ifTable entries */
                interfaces: components["schemas"]["Interface"][];
                /** @description IP addresses on this host. */
                ip_addresses: components["schemas"]["IPAddress"][];
                /**
                 * Format: date-time
                 * @description Last time discovery observed this host. User-facing (drives the "Last
                 *     seen" column and the stale badge), which is why it is carried here while
                 *     the rest of the SCD2/audit columns are not.
                 */
                last_seen_at: string;
                /** @description Link to the host's own management interface. */
                management_url?: string | null;
                /** @description ENTITY-MIB entPhysicalMfgName — hardware manufacturer. Read-only, as above. */
                readonly manufacturer?: string | null;
                /** @description ENTITY-MIB entPhysicalModelName — hardware model. Read-only, as above. */
                readonly model?: string | null;
                /** @description Human-facing name for the host. */
                name: string;
                /**
                 * @description Which rung of the naming ladder produced `name`. Read-only: it is decided by whoever
                 *     supplied the name, not by the caller.
                 */
                name_source?: components["schemas"]["HostNameSource"];
                /**
                 * Format: uuid
                 * @description The network this entity belongs to.
                 */
                network_id: string;
                /** @description Open ports on this host. */
                ports: components["schemas"]["Port"][];
                /** @description ENTITY-MIB entPhysicalSerialNum — hardware serial number. Read-only, as above. */
                readonly serial_number?: string | null;
                /** @description Services running on this host. */
                services: components["schemas"]["Service"][];
                /** @description How this host came to be known — discovered, imported, or created by hand. */
                source: components["schemas"]["EntitySource"];
                /** @description SNMP sysContact — administrative contact as configured on the device. */
                sys_contact?: string | null;
                /** @description SNMP sysDescr — the device's own description of itself. */
                sys_descr?: string | null;
                /** @description SNMP sysLocation — physical location as configured on the device. */
                sys_location?: string | null;
                /**
                 * @description SNMP sysName.0 — the administratively-assigned hostname. Read-only: discovery collects it
                 *     from the device, so neither create nor update accepts it.
                 */
                readonly sys_name?: string | null;
                /** @description SNMP sysObjectID — the vendor's identifier for the device model. */
                sys_object_id?: string | null;
                /** @description Tags assigned to this entity. */
                tags: string[];
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                updated_at: string;
                virtualization_metadata?: null | components["schemas"]["HostVirtualization"];
                /**
                 * Format: uuid
                 * @description The hypervisor service this VM runs on.
                 */
                virtualization_service_id?: string | null;
            }[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Service: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["ServiceBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description The discovery that first observed this entity.
                 */
                readonly first_discovery_id?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description The most recent discovery that observed this entity.
                 */
                readonly last_discovery_id?: string | null;
                /**
                 * Format: date-time
                 * @description When a discovery last observed this entity.
                 */
                readonly last_seen_at?: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Subnet: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["SubnetBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description The discovery that first observed this entity.
                 */
                readonly first_discovery_id?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description The most recent discovery that observed this entity.
                 */
                readonly last_discovery_id?: string | null;
                /**
                 * Format: date-time
                 * @description When a discovery last observed this entity.
                 */
                readonly last_seen_at?: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Tag: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["TagBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Topology: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["TopologyBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_User: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["UserBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_UserApiKey: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["UserApiKeyBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
            success: boolean;
        };
        /** @description Response type for paginated list endpoints (pagination is always present in meta) */
        PaginatedApiResponse_Vlan: {
            /** @description The page of results. Empty when nothing matched the query. */
            data: (components["schemas"]["VlanBase"] & {
                /**
                 * Format: date-time
                 * @description When this record was first created.
                 */
                readonly created_at: string;
                /**
                 * Format: uuid
                 * @description The discovery that first observed this entity.
                 */
                readonly first_discovery_id?: string | null;
                /**
                 * Format: uuid
                 * @description Server-assigned unique identifier.
                 */
                readonly id: string;
                /**
                 * Format: uuid
                 * @description The most recent discovery that observed this entity.
                 */
                readonly last_discovery_id?: string | null;
                /**
                 * Format: date-time
                 * @description When a discovery last observed this entity.
                 */
                readonly last_seen_at?: string;
                /**
                 * Format: uuid
                 * @description Stable identifier shared by every revision of the same entity across its history.
                 */
                readonly lineage_id?: string | null;
                /**
                 * Format: date-time
                 * @description When this record was last modified.
                 */
                readonly updated_at: string;
                /**
                 * Format: date-time
                 * @description Start of the interval this revision was current for (SCD2 history).
                 */
                readonly valid_from?: string;
                /**
                 * Format: date-time
                 * @description End of the interval this revision was current for. `null` while it is the live revision.
                 */
                readonly valid_to?: string | null;
            })[];
            /** @description Human-readable failure message. Omitted on success. */
            error?: string | null;
            /** @description API and server version metadata, plus pagination counters. */
            meta: components["schemas"]["PaginatedApiMeta"];
            /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
            /**
             * @description Size of every group, in the same order the rows are grouped, when the
             *     request specified a `group_by`. Lets a paginated client show a group's
             *     true size instead of the slice of it that happens to be on this page.
             *     Absent when the list isn't grouped.
             */
            group_counts?: components["schemas"]["GroupCount"][] | null;
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
        /**
         * @description Pause subscription duration. The cancel modal's `RadioGroup` posts
         *     one of these enum variants verbatim — no integer parsing at the API
         *     boundary, the type is the contract.
         * @enum {string}
         */
        PauseDuration: "days30" | "days60" | "days90";
        PauseSubscriptionRequest: {
            /** @description How long to pause billing for, in days. */
            duration_days: components["schemas"]["PauseDuration"];
        };
        PlanConfig: {
            /**
             * Format: int64
             * @description Fixed charge per billing period, in cents.
             */
            base_cents: number;
            /**
             * Format: int64
             * @description Charge per host beyond `included_hosts`, in cents.
             */
            host_cents?: number | null;
            /**
             * Format: int64
             * @description Hosts included before per-host charges apply.
             */
            included_hosts?: number | null;
            /**
             * Format: int64
             * @description Networks included before per-network charges apply.
             */
            included_networks?: number | null;
            /**
             * Format: int64
             * @description Organizations allowed on one self-hosted server instance. `None` =
             *     unlimited. Only enforced for self-hosted deployments (see
             *     `provision_user`); cloud stays multi-tenant regardless. Defaulted so
             *     existing stored plan JSON deserializes unchanged.
             */
            included_orgs?: number | null;
            /**
             * Format: int64
             * @description Seats included before per-seat charges apply.
             */
            included_seats?: number | null;
            /**
             * Format: int64
             * @description Charge per network beyond `included_networks`, in cents.
             */
            network_cents?: number | null;
            /** @description Billing interval this configuration is priced for. */
            rate: components["schemas"]["BillingRate"];
            /**
             * Format: int64
             * @description Charge per seat beyond `included_seats`, in cents.
             */
            seat_cents?: number | null;
            /**
             * Format: int32
             * @description Length of the free trial, in days. Zero when the plan has no trial.
             */
            trial_days: number;
        };
        /**
         * @description Derived subscription status — our domain enum, never Stripe's raw status.
         *     Stripe webhook events map to typed `BillingOperation` variants at reception
         *     (in `billing/service.rs`); each variant deterministically implies a
         *     `PlanStatus` for downstream feature gates via
         *     `BillingOperation::implied_status`.
         *
         *     `FromStr` is derived (via strum) so the storage layer can round-trip a
         *     snake_case `text` column back into the typed value; `ToSchema` exposes
         *     the enum as a stricter string union in the generated OpenAPI schema so
         *     the frontend's `org.plan_status === 'paused'` comparisons are
         *     compile-checked against the canonical variant list.
         * @enum {string}
         */
        PlanStatus: "active" | "trialing" | "past_due" | "paused" | "pending_cancellation" | "cancelled";
        /** @description Plan usage limits and current counts */
        PlanUsage: {
            /**
             * Format: int64
             * @description Hosts currently counted against the plan.
             */
            host_count: number;
            /**
             * Format: int64
             * @description Hosts included in the current plan. `null` when unlimited.
             */
            host_limit?: number | null;
            /**
             * Format: int64
             * @description Networks currently counted against the plan.
             */
            network_count: number;
            /**
             * Format: int64
             * @description Networks included in the current plan. `null` when unlimited.
             */
            network_limit?: number | null;
            /**
             * Format: int64
             * @description Seats currently in use.
             */
            seat_count: number;
            /**
             * Format: int64
             * @description Seats included in the current plan. `null` when unlimited.
             */
            seat_limit?: number | null;
        };
        PodmanVirtualization: {
            /** @description Compose project the container belongs to, when it was started by Compose. */
            compose_project?: string | null;
            /** @description Podman container ID. */
            container_id?: string | null;
            /** @description Container name as reported by Podman. */
            container_name?: string | null;
        };
        /**
         * @description Port entity with custom serialization that flattens PortType fields.
         * @example {
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "first_discovery_id": null,
         *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *       "id": "550e8400-e29b-41d4-a716-446655440006",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "lineage_id": null,
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "number": 80,
         *       "protocol": "Tcp",
         *       "type": "Http",
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null
         *     }
         */
        Port: components["schemas"]["PortBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description The discovery that first observed this entity.
             */
            readonly first_discovery_id?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: uuid
             * @description The most recent discovery that observed this entity.
             */
            readonly last_discovery_id?: string | null;
            /**
             * Format: date-time
             * @description When a discovery last observed this entity.
             */
            readonly last_seen_at?: string;
            /**
             * Format: uuid
             * @description Stable identifier shared by every revision of the same entity across its history.
             */
            readonly lineage_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description Start of the interval this revision was current for (SCD2 history).
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description End of the interval this revision was current for. `null` while it is the live revision.
             */
            readonly valid_to?: string | null;
        };
        /** @description The base data for a Port entity (everything except id, created_at, updated_at) */
        PortBase: components["schemas"]["PortType"] & {
            /**
             * Format: uuid
             * @description The host this entity belongs to.
             */
            host_id: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
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
            /** @description TCP or UDP port number */
            number: number;
            /**
             * @description Transport protocol the port is open on.
             * @enum {string}
             */
            protocol: "Udp" | "Tcp";
            /**
             * @description Well-known port identifier. Auto-derived from number+protocol, so it is optional on create.
             * @enum {string}
             */
            type?: "Ssh" | "Telnet" | "DnsUdp" | "DnsTcp" | "Samba" | "Nfs" | "Ftp" | "Ipp" | "LdpTcp" | "LdpUdp" | "Ldap" | "Ldaps" | "Kerberos" | "Snmp" | "SnmpAlt" | "Rdp" | "Ntp" | "Sip" | "SipTls" | "Rtsp" | "Dhcp" | "Http" | "MySql" | "PostgreSQL" | "MongoDB" | "Redis" | "MsSql" | "Docker" | "DockerTls" | "Kubernetes" | "RabbitMqMgmt" | "Cassandra" | "Elasticsearch" | "InfluxDb" | "CouchDb" | "Kafka" | "Http3000" | "Http5000" | "Http8080" | "Http8081" | "Http8082" | "Http8888" | "Http9000" | "Https" | "Https8443" | "Https9443" | "Https10443" | "Mqtt" | "MqttTls" | "AMQP" | "AMQPTls" | "Wireguard" | "OpenVPN" | "BACnet" | "JetDirect" | "Custom";
        };
        /** @description Request to update user profile (deferred marketing fields) */
        ProfileUpdateRequest: {
            /** @description Company size bracket, collected during onboarding. */
            company_size?: string | null;
            /** @description The user's job title, collected during onboarding. */
            job_title?: string | null;
        };
        /**
         * @description Request to pre-provision a daemon (either mode) before it is installed.
         *     This creates the daemon record + its 1:1 API key on the server so the install
         *     command shrinks to two flags.
         */
        ProvisionDaemonRequest: {
            /**
             * Format: uuid
             * @description Mint a fresh 1:1 key for this existing daemon instead of creating a new record,
             *     keeping its host, discovery jobs and history. Used to give a legacy daemon (no bound
             *     key) a dedicated one. When set, `name`/`network_id`/`mode`/`url` are ignored — those
             *     come from the existing record.
             *
             *     Only accepted for a daemon that has never checked in or has no bound key; a live
             *     provisioned daemon is refused, since it has no way to learn the new key.
             *
             *     Note: install commands are not generated here — call the install-command endpoint,
             *     which builds them idempotently and fills in the key this response returns.
             */
            daemon_id?: string | null;
            /**
             * @description How the daemon communicates with the server. Defaults to DaemonPoll
             *     (the daemon dials out) for forward-compat with older clients.
             */
            mode?: components["schemas"]["DaemonMode"];
            /**
             * @description Human-readable name for the daemon. Required unless `daemon_id` is set, in which case
             *     the existing record's name is kept.
             */
            name?: string | null;
            /**
             * Format: uuid
             * @description Network this daemon will be associated with. Required unless `daemon_id` is set, in
             *     which case the existing record's network is kept.
             */
            network_id?: string | null;
            /**
             * @description Credential/integration references to seed onto the daemon's first
             *     discovery run. References only — never secret material. Empty by default.
             */
            seed_credential_refs?: components["schemas"]["IntegrationTarget"][];
            /**
             * @description Reachable URL where the *server* can dial the daemon. Required for
             *     ServerPoll, unused for DaemonPoll (the daemon dials out instead).
             */
            url?: string | null;
        };
        /**
         * @description Response from provisioning a daemon.
         *     Contains the daemon record and the API key (shown only once).
         *
         *     Install commands are deliberately not here — fetch them from the install-command endpoint,
         *     which builds them idempotently and fills in this key. That keeps a display-only regenerate
         *     (advanced-setting change, OS switch) from re-minting the key.
         */
        ProvisionDaemonResponse: {
            /** @description The created daemon record (with version status). */
            daemon: components["schemas"]["DaemonResponse"];
            /**
             * Format: password
             * @description The API key (plaintext) for daemon authentication.
             *     This is shown only once - store it securely.
             */
            readonly daemon_api_key: string;
        };
        ProxmoxVirtualization: {
            /** @description Proxmox VMID of the guest. */
            vm_id?: string | null;
            /** @description Guest name as configured in Proxmox. */
            vm_name?: string | null;
        };
        PublicConfigResponse: {
            /** @description Whether this deployment has billing configured. */
            billing_enabled: boolean;
            /** @description How this instance is run: cloud, commercial self-hosted, or community. */
            deployment_type: components["schemas"]["DeploymentType"];
            /** @description Whether email/password login is turned off, leaving OIDC as the only method. */
            disable_password_login: boolean;
            /** @description Whether self-service sign-up is turned off on this deployment. */
            disable_registration: boolean;
            /**
             * @description `STRIPE_SAVE_OFFER_COUPON_ID` env var is set. When false, the
             *     cancel modal hides the discount save-offer panel so the user
             *     doesn't see an option the deployment can't fulfil.
             */
            discount_save_offer_available: boolean;
            /** @description Whether the deployment asks users to opt in to product email. */
            has_email_opt_in: boolean;
            /** @description Whether outbound email is configured. Invites and password resets need it. */
            has_email_service: boolean;
            /** @description Whether a daemon runs alongside the server, so no separate install is needed to start scanning. */
            has_integrated_daemon: boolean;
            /**
             * Format: date
             * @description Hard expiry — the drop-dead date after which the server rejects
             *     the key. Referenced by the grace-period banner.
             */
            license_expiry?: string | null;
            /**
             * @description True when the license is past `intended_exp` but not yet past
             *     the hard `exp` — the silent grace window.
             */
            license_in_grace_period: boolean;
            /**
             * Format: date
             * @description User-visible expiry — the date displayed to end users under
             *     normal operation. 7 days earlier than `license_expiry` for keys
             *     issued after grace-period support landed.
             */
            license_intended_expiry?: string | null;
            license_status?: null | components["schemas"]["LicenseStatusDiscriminants"];
            /** @description Whether the client should show a cookie-consent prompt. */
            needs_cookie_consent: boolean;
            /** @description Identity providers available on the login screen. */
            oidc_providers: components["schemas"]["OidcProviderMetadata"][];
            /**
             * @description True when this self-hosted instance has reached its licensed
             *     organization cap (`included_orgs`), so new-org registration is blocked.
             *     Always false on cloud (multi-tenant) and on unlimited-org plans.
             */
            org_limit_reached: boolean;
            /** @description Public analytics key, when analytics is enabled. */
            posthog_key?: string | null;
            /**
             * Format: uri
             * @description Base URL this server is reachable at, as configured by the operator.
             */
            public_url: string;
            /**
             * Format: email
             * @description Admin contact email to show users blocked by `org_limit_reached`,
             *     from `SCANOPY_SERVER_ADMIN_CONTACT_EMAIL`.
             */
            server_admin_contact_email: string;
            /**
             * Format: int32
             * @description Port this server listens on.
             */
            server_port: number;
            /**
             * Format: int32
             * @description `SCANOPY_SNAPSHOT_RETENTION_DAYS_OVERRIDE` if set on this instance.
             *     Frontend uses it inside the plan-comparison view to display the
             *     effective retention for this deployment rather than the per-plan
             *     fixture default.
             */
            snapshot_retention_days_override?: number | null;
            /**
             * @description Stripe publishable key, exposed so the frontend can mount Stripe
             *     Elements (Payment Element) for in-app card collection. `None` when
             *     billing isn't configured. Publishable keys are safe to expose to the
             *     browser (same as `posthog_key`).
             */
            stripe_publishable_key?: string | null;
        };
        /** @description Public share metadata (returned without authentication) */
        PublicShareMetadata: {
            /**
             * @description Resolved list of available topology views for this share.
             *     Filtered by both share configuration and data availability.
             *     First element is the default view.
             */
            enabled_views: components["schemas"]["TopologyView"][];
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            id: string;
            /** @description Human-facing name for this share. */
            name: string;
            /** @description What the viewer can see and do. */
            options: components["schemas"]["ShareOptions"];
            /** @description Whether a password must be supplied before the topology is returned. */
            requires_password: boolean;
        };
        /**
         * @description How a user first heard about Scanopy, as offered by the onboarding prompt.
         * @enum {string}
         */
        ReferralSource: "search_engine" | "ai_assistant" | "youtube" | "tiktok" | "blog_article" | "reddit" | "hacker_news" | "social_media" | "word_of_mouth" | "proxmox_community_scripts" | "self_hosted" | "other" | "prefer_not_to_say";
        /** @description Request to submit referral source */
        ReferralSourceRequest: {
            /** @description How the user heard about Scanopy. */
            referral_source: components["schemas"]["ReferralSource"];
            /** @description Free-text detail, sent when `referral_source` is `other`. */
            referral_source_other?: string | null;
        };
        /** @description Registration request from client */
        RegisterRequest: {
            /** @description Honeypot field for bot detection */
            company_url?: string | null;
            /**
             * Format: email
             * @description Email address for the new account. Must be deliverable.
             */
            email: string;
            /** @description Whether the user agreed to receive product and marketing email. */
            marketing_opt_in?: boolean;
            /**
             * Format: password
             * @description Password for the new account. Minimum 10 characters.
             */
            password: string;
            /** @description Must be `true` — records that the user accepted the terms of service. */
            terms_accepted: boolean;
        };
        RequestEmailChangeRequest: {
            /**
             * Format: password
             * @description Current password — required if the user already has a password set.
             *     Not required for OIDC-only users.
             */
            current_password?: string | null;
            /**
             * Format: email
             * @description Address to move the account to. A confirmation link is sent there.
             */
            new_email: string;
        };
        /**
         * @description Scan settings that apply to a single-host rescan.
         *
         *     Deliberately narrower than [`ScanSettings`]: a rescan verifies a known host
         *     against a known port set, so the full-scan mechanism (`is_full_scan`,
         *     `full_scan_interval`) must not be expressible — promoting a rescan to a
         *     65,535-port sweep defeats the feature. The remaining omissions are settings
         *     that cannot bind on a one-or-two address target.
         */
        RescanSettings: {
            /**
             * Format: int32
             * @description ARP retry rounds. Matters more here than in a sweep: for a rescan, "did
             *     it answer" is the entire answer, so a missed round reads as a dead host.
             */
            arp_retries?: number | null;
            /** @description Ports scanned concurrently per host. */
            port_scan_batch_size?: number | null;
            /**
             * @description Whether to probe raw-socket ports 9100-9107. Correctness-affecting: with
             *     this off the scanner drops those ports from its results, so a printer's
             *     known JetDirect port would look like it had disappeared.
             */
            probe_raw_socket_ports?: boolean;
            /**
             * Format: int32
             * @description Port scan probes per second. Operators lower this for fragile devices or
             *     noisy IDS, and a rescan must respect that as much as a discovery does.
             */
            scan_rate_pps?: number | null;
            /** @description On Windows, use Npcap broadcast ARP instead of SendARP. */
            use_npcap_arp?: boolean;
        };
        /** @description Request to resend verification email */
        ResendVerificationRequest: {
            /**
             * Format: email
             * @description Address to resend the verification email to.
             */
            email: string;
        };
        ResetPasswordRequest: {
            /**
             * Format: password
             * @description The new password. Minimum 10 characters.
             */
            password: string;
            /** @description Single-use token from the password-reset email. */
            token: string;
        };
        RunType: {
            /** @description Cron expression deciding when the scan runs. */
            cron_schedule: string;
            /** @description Whether the schedule is active. */
            enabled: boolean;
            /**
             * Format: date-time
             * @description When the scan last ran.
             */
            readonly last_run?: string | null;
            /** @description IANA timezone for cron evaluation, e.g. "America/New_York". None = UTC. */
            timezone?: string | null;
            /** @enum {string} */
            type: "Scheduled";
        } | {
            /** @description The recorded outcome of the run. */
            results: components["schemas"]["DiscoveryUpdatePayload"];
            /** @enum {string} */
            type: "Historical";
        } | {
            /**
             * Format: date-time
             * @description When the scan last ran.
             */
            readonly last_run?: string | null;
            /** @enum {string} */
            type: "AdHoc";
        };
        /**
         * @description Save-offer choices presented during in-app cancellation (Phase 5).
         * @enum {string}
         */
        SaveOffer: "pause" | "discount" | "downgrade";
        /**
         * @description Live terms for the configured save-offer coupon, read directly from
         *     Stripe. Used by the cancel modal's Discount panel to render the offer
         *     dynamically instead of hard-coding the percent/duration.
         *
         *     Only returned when the coupon would actually catch the user's next
         *     invoice — i.e. `next_renewal_at` falls within the coupon's `duration_in_months`
         *     window. Yearly subscribers partway through a cycle whose next renewal
         *     lands after the coupon's window get `None` from the endpoint and the
         *     cancel modal's Discount panel doesn't render.
         *
         *     `billing_rate` lets the frontend pick monthly vs yearly copy: a monthly
         *     subscriber thinks in terms of "N months of discount"; a yearly subscriber
         *     thinks in terms of "my next renewal on {date}."
         */
        SaveOfferCoupon: {
            /** @description Billing interval the discount applies to. */
            billing_rate: components["schemas"]["BillingRate"];
            /**
             * Format: int64
             * @description How many months the discount lasts.
             */
            duration_in_months: number;
            /**
             * Format: date-time
             * @description When the discounted subscription next renews.
             */
            next_renewal_at: string;
            /**
             * Format: int64
             * @description Discount applied by the retention offer.
             */
            percent_off: number;
        };
        /**
         * @description Scan performance settings. Lives on the discovery entity.
         *     Numeric fields are `Option<T>` — `None` means "use daemon default".
         *     The daemon unwraps with defaults at point of use.
         */
        ScanSettings: {
            /**
             * Format: int32
             * @description ARP packets per second (default: 50)
             */
            arp_rate_pps?: number | null;
            /**
             * Format: int32
             * @description ARP retry rounds for non-responsive targets (default: 2 = 3 total attempts)
             */
            arp_retries?: number | null;
            /**
             * Format: int32
             * @description ARP scan cutoff prefix. Interfaced subnets larger than this prefix are
             *     truncated to this many IPs. Default: 15 (= /15, ~131K IPs).
             *     Lower values scan more IPs — increase arp_rate_pps accordingly.
             */
            arp_scan_cutoff?: number | null;
            /**
             * Format: int32
             * @description Run a full 65k port scan every N scans. Other scans use a light port set.
             *     Default: 3. Value of 0 means never full scan. Value of 1 means every scan is full.
             */
            full_scan_interval?: number | null;
            /**
             * @description Whether this specific scan run should do a full 65k port scan.
             *     Set by the server before dispatching to the daemon — not user-configurable.
             */
            is_full_scan?: boolean;
            /**
             * Format: int32
             * @description Hard ceiling on how long a single discovery run may take, in seconds
             *     (default: 21600 = 6h). When hit, the run force-completes and any hosts
             *     still queued are left un-scanned until the next run. Raise this for very
             *     large networks that legitimately need more than the default window.
             */
            max_discovery_duration?: number | null;
            /** @description Ports scanned concurrently per host (default: 200, clamped 16-1000) */
            port_scan_batch_size?: number | null;
            /**
             * @description Whether to probe raw-socket ports 9100-9107 (default: false).
             *     Disabled by default to prevent ghost printing on JetDirect printers.
             */
            probe_raw_socket_ports?: boolean;
            /**
             * Format: int32
             * @description Port scan probes per second (default: 500)
             */
            scan_rate_pps?: number | null;
            /** @description On Windows, use Npcap broadcast ARP instead of SendARP (default: false) */
            use_npcap_arp?: boolean;
        };
        /**
         * @description Canonical IDs of entities scanned in a discovery session.
         *
         *     Populated daemon-side at terminal phase from `EntityBuffer`'s `Created`
         *     entries. Travels with the terminal `DiscoveryUpdatePayload` to the server,
         *     rides the in-memory `EntityOperation::Created` event published for the
         *     historical Discovery row (the event scope carries `Entity::Discovery` with
         *     the full struct, including `run_type::Historical { results }`), then is
         *     stripped before persisting into the historical Discovery row's JSONB (see
         *     the `SqlValue::RunType` bind_value handler in
         *     `backend/src/server/shared/storage/generic.rs`). Per-entity-service
         *     subscribers extract `results.scanned` from the in-memory event and call
         *     `DiscoveryFkUpdater::update_discovery_fks` to backfill
         *     `last_discovery_id` / `first_discovery_id` on the matched rows.
         *
         *     Naming: `scanned_*` because the daemon scans entities — some submissions
         *     match existing rows (refresh), others insert new rows. Both populate the
         *     EntityBuffer with canonical (server-assigned) IDs.
         */
        ScannedEntityIds: {
            /** @description Service bindings touched by this discovery. */
            binding_ids?: string[];
            /** @description Hosts touched by this discovery. */
            host_ids?: string[];
            /** @description Interfaces touched by this discovery. */
            interface_ids?: string[];
            /** @description IP addresses touched by this discovery. */
            ip_address_ids?: string[];
            /** @description Ports touched by this discovery. */
            port_ids?: string[];
            /** @description Services touched by this discovery. */
            service_ids?: string[];
            /** @description Subnets touched by this discovery. */
            subnet_ids?: string[];
            /** @description VLANs touched by this discovery. */
            vlan_ids?: string[];
        };
        /** @description Secret value that can be either inline content or a file path on the daemon host. */
        SecretValue: {
            /** @enum {string} */
            mode: "Inline";
            /** @description The secret itself. Write-only — reads return a redacted placeholder. */
            value: string;
        } | {
            /** @enum {string} */
            mode: "FilePath";
            /** @description Path to a file on the daemon host holding the secret. */
            path: string;
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
         *           "created_at": "2026-08-25T22:05:35.391052Z",
         *           "first_discovery_id": null,
         *           "id": "22a0a107-efcf-47f6-8b25-f0734015b517",
         *           "ip_address_id": "550e8400-e29b-41d4-a716-446655440005",
         *           "last_discovery_id": null,
         *           "last_seen_at": "2026-08-25T22:05:35.391052Z",
         *           "lineage_id": null,
         *           "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *           "port_id": "550e8400-e29b-41d4-a716-446655440006",
         *           "service_id": "550e8400-e29b-41d4-a716-446655440007",
         *           "type": "Port",
         *           "updated_at": "2026-08-25T22:05:35.391052Z",
         *           "valid_from": "2026-08-25T22:05:35.391052Z",
         *           "valid_to": null
         *         }
         *       ],
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "first_discovery_id": null,
         *       "host_id": "550e8400-e29b-41d4-a716-446655440003",
         *       "id": "550e8400-e29b-41d4-a716-446655440007",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "lineage_id": null,
         *       "name": "nginx",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "position": 0,
         *       "service_definition": "Google Home",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null,
         *       "virtualization_metadata": null,
         *       "virtualization_service_id": null
         *     }
         */
        Service: components["schemas"]["ServiceBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description The discovery that first observed this entity.
             */
            readonly first_discovery_id?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: uuid
             * @description The most recent discovery that observed this entity.
             */
            readonly last_discovery_id?: string | null;
            /**
             * Format: date-time
             * @description When a discovery last observed this entity.
             */
            readonly last_seen_at?: string;
            /**
             * Format: uuid
             * @description Stable identifier shared by every revision of the same entity across its history.
             */
            readonly lineage_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description Start of the interval this revision was current for (SCD2 history).
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description End of the interval this revision was current for. `null` while it is the live revision.
             */
            readonly valid_to?: string | null;
        };
        ServiceBase: {
            /** @description Ports and IP addresses this service is reachable on. */
            bindings: components["schemas"]["Binding"][];
            /**
             * Format: uuid
             * @description The host this entity belongs to.
             */
            host_id: string;
            /** @description Human-facing name for the service. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /**
             * Format: int32
             * @description Position of this service in the host's service list (for ordering)
             */
            position: number;
            /** @description Which known software this service is, if identified. */
            service_definition: string;
            /** @description Will be automatically set to Manual for creation through API */
            source: components["schemas"]["EntitySource"];
            /** @description Tags assigned to this entity. */
            tags: string[];
            virtualization_metadata?: null | components["schemas"]["ServiceVirtualization"];
            /**
             * Format: uuid
             * @description The container runtime service hosting this container — see the note on
             *     `HostBase::virtualization_service_id`.
             */
            virtualization_service_id: string | null;
        };
        /** @enum {string} */
        ServiceCategory: "NetworkCore" | "NetworkAccess" | "NetworkAppliance" | "RemoteAccess" | "Storage" | "Backup" | "Media" | "HomeAutomation" | "Hypervisor" | "ContainerRuntime" | "Container" | "Orchestrator" | "DNS" | "VPN" | "Monitoring" | "AdBlock" | "ReverseProxy" | "Workstation" | "Mobile" | "IoT" | "Printer" | "Database" | "Development" | "Dashboard" | "MessageQueue" | "IdentityAndAccess" | "Integration" | "Office" | "ProjectManagement" | "Messaging" | "Conferencing" | "Telephony" | "Email" | "Publishing" | "Unknown" | "Custom" | "Scanopy" | "OpenPorts";
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
            virtualization_metadata?: null | components["schemas"]["ServiceVirtualization"];
            /**
             * Format: uuid
             * @description The container runtime service hosting this container, if any.
             */
            virtualization_service_id?: string | null;
        };
        /**
         * @description Fields that services can be ordered/grouped by.
         * @enum {string}
         */
        ServiceOrderField: "created_at" | "name" | "updated_at" | "host" | "network_id" | "position" | "service_definition" | "last_seen_at";
        /** ServiceVirtualization */
        ServiceVirtualization: {
            details: components["schemas"]["DockerVirtualization"];
            /** @enum {string} */
            type: "Docker";
        } | {
            details: components["schemas"]["PodmanVirtualization"];
            /** @enum {string} */
            type: "Podman";
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
        /**
         * @description Response for creating a SetupIntent — the client secret the frontend
         *     Payment Element uses to collect and confirm a card in-app.
         */
        SetupIntentResponse: {
            /** @description Stripe SetupIntent client secret, used to mount the Payment Element. */
            client_secret: string;
        };
        /** @description Setup request for pre-registration org/network configuration */
        SetupRequest: {
            /** @description The first network to create alongside the organization. */
            network: components["schemas"]["NetworkSetup"];
            /** @description Name for the organization created during setup. */
            organization_name: string;
        };
        /** @description Response from setup endpoint */
        SetupResponse: {
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
        };
        Share: components["schemas"]["ShareBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        /**
         * @description Access token returned after successful password verification.
         *
         *     The token is an HS256 JWT tied to the share's `password_hash` — changing
         *     the share password implicitly invalidates all outstanding tokens.
         */
        ShareAccessTokenResponse: {
            /** @description Bearer token granting access to this share for the rest of the session. */
            access_token: string;
            /**
             * Format: date-time
             * @description When this record stops being valid.
             */
            expires_at: string;
        };
        ShareBase: {
            /** @description Domains permitted to embed this share. Empty means no restriction. */
            allowed_domains: string[] | null;
            /**
             * Format: uuid
             * @description User who created the share.
             */
            created_by: string;
            /**
             * @description Which topology views are enabled for this share.
             *     None = all views (subject to data availability). Some(list) = only these views in order.
             *     First element is the default view shown on load.
             */
            enabled_views: components["schemas"]["TopologyView"][] | null;
            /**
             * Format: date-time
             * @description When this record stops being valid.
             */
            expires_at: string | null;
            /** @description Whether the link still resolves. Disabled shares return 404. */
            is_enabled: boolean;
            /** @description Human-facing name for this share. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description What the viewer can see and do. */
            options: components["schemas"]["ShareOptions"];
            /**
             * Format: password
             * @description Plaintext password on ingest; redacted sentinel (`"********"`) or `None` on egress.
             *     Never stored — `password_hash` is the DB column. Wrapped in `SecretString` so
             *     `Debug`/logging shows `[REDACTED]` during the window between request
             *     deserialization and hashing.
             */
            password?: string | null;
            /**
             * Format: uuid
             * @description The topology this share exposes.
             */
            topology_id: string;
        };
        /** @description Share display options */
        ShareOptions: {
            /** @description Viewer sees the export button. */
            show_export_button: boolean;
            /** @description Viewer can open the inspector for a selected element. */
            show_inspect_panel: boolean;
            /** @description Viewer sees the minimap. */
            show_minimap: boolean;
            /** @description Viewer sees the zoom controls. */
            show_zoom_controls: boolean;
        };
        Snapshot: components["schemas"]["SnapshotBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        SnapshotBase: {
            /**
             * Format: uuid
             * @description User who took the snapshot.
             */
            created_by_user_id?: string | null;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /**
             * Format: date-time
             * @description The point in time this snapshot captures.
             */
            taken_at: string;
        };
        /**
         * @description SNMPv3 USM authentication protocol. Variants are limited to the modern,
         *     secure set Scanopy supports; MD5 / SHA-2 variants beyond these are
         *     intentionally excluded. Serialized form (e.g. "Sha256") is the wire value
         *     stored in the credential and used as the frontend select option value.
         * @enum {string}
         */
        SnmpV3AuthProtocol: "Sha1" | "Sha256";
        /**
         * @description SNMPv3 USM privacy (encryption) protocol.
         * @enum {string}
         */
        SnmpV3PrivProtocol: "Aes128" | "Aes256";
        /**
         * @description An SNMP data group a walk may come up short on.
         *
         *     An enum rather than a free string so the code derivation below is exhaustive: every group has
         *     to declare which consequence sentence describes it, and a new one cannot be added without
         *     choosing.
         * @enum {string}
         */
        SnmpWalkGroup: "Lldp" | "Cdp" | "Interfaces" | "BridgePortNumbering" | "BridgeForwarding" | "VlanMembership" | "ArpTable" | "DeviceInventory" | "IpAddresses" | "LldpLocalPorts" | "VlanNames";
        /**
         * @example {
         *       "cidr": "192.168.1.0/24",
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "description": "Local area network",
         *       "first_discovery_id": null,
         *       "id": "550e8400-e29b-41d4-a716-446655440004",
         *       "last_discovery_id": null,
         *       "last_seen_at": "2026-01-15T10:30:00Z",
         *       "lineage_id": null,
         *       "name": "LAN",
         *       "network_id": "550e8400-e29b-41d4-a716-446655440002",
         *       "source": {
         *         "type": "Manual"
         *       },
         *       "subnet_type": "Lan",
         *       "tags": [],
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null,
         *       "virtualization_service_id": null
         *     }
         */
        Subnet: components["schemas"]["SubnetBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description The discovery that first observed this entity.
             */
            readonly first_discovery_id?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: uuid
             * @description The most recent discovery that observed this entity.
             */
            readonly last_discovery_id?: string | null;
            /**
             * Format: date-time
             * @description When a discovery last observed this entity.
             */
            readonly last_seen_at?: string;
            /**
             * Format: uuid
             * @description Stable identifier shared by every revision of the same entity across its history.
             */
            readonly lineage_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description Start of the interval this revision was current for (SCD2 history).
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description End of the interval this revision was current for. `null` while it is the live revision.
             */
            readonly valid_to?: string | null;
        };
        SubnetBase: {
            /**
             * @description Subnet in CIDR notation, IPv4 or IPv6.
             * @example 192.168.1.0/24
             */
            cidr: string;
            /** @description Free-text notes about the subnet. */
            description?: string | null;
            /** @description Human-facing name for this subnet. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description Will be automatically set to Manual for creation through API */
            source: components["schemas"]["EntitySource"];
            /** @description What kind of subnet this is — physical, virtual, container bridge, and so on. */
            subnet_type: components["schemas"]["SubnetType"];
            /** @description Tags assigned to this entity. */
            tags: string[];
            /**
             * Format: uuid
             * @description The container runtime service that owns this bridge network.
             *
             *     Load-bearing for dedup: the same CIDR on two different Docker daemons is two distinct
             *     subnets, so bridge rows only merge when this matches as well as the CIDR and network.
             *     A foreign key rather than a field inside a JSONB blob because a stale value here is
             *     precisely what made a scan add a duplicate bridge row every time (GH #650) — now it
             *     cannot be written at all.
             */
            virtualization_service_id: string | null;
        };
        /**
         * @description Fields that subnets can be ordered/grouped by.
         * @enum {string}
         */
        SubnetOrderField: "created_at" | "name" | "cidr" | "subnet_type" | "updated_at" | "network_id" | "last_seen_at";
        /** @enum {string} */
        SubnetType: "Internet" | "Remote" | "Gateway" | "VpnTunnel" | "Dmz" | "Lan" | "WiFi" | "IoT" | "Guest" | "DockerBridge" | "PodmanBridge" | "MacVlan" | "IpVlan" | "Management" | "Storage" | "Loopback" | "Unknown";
        /**
         * @example {
         *       "color": "Green",
         *       "created_at": "2026-01-15T10:30:00Z",
         *       "description": "Production environment resources",
         *       "id": "550e8400-e29b-41d4-a716-44665544000a",
         *       "is_application": false,
         *       "lineage_id": null,
         *       "name": "production",
         *       "organization_id": "550e8400-e29b-41d4-a716-446655440001",
         *       "updated_at": "2026-01-15T10:30:00Z",
         *       "valid_from": "2026-01-15T10:30:00Z",
         *       "valid_to": null
         *     }
         */
        Tag: components["schemas"]["TagBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: uuid
             * @description Stable identifier shared by every revision of the same entity across its history.
             */
            readonly lineage_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description Start of the interval this revision was current for (SCD2 history).
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description End of the interval this revision was current for. `null` while it is the live revision.
             */
            readonly valid_to?: string | null;
        };
        TagBase: {
            /** @description Colour the tag is drawn in. */
            color: components["schemas"]["Color"];
            /** @description Free-text notes about the tag. */
            description?: string | null;
            /** @description Whether this tag groups an application, so it drives the application view. */
            is_application?: boolean;
            /** @description Human-facing name for this tag. */
            name: string;
            /**
             * Format: uuid
             * @description The organization that owns this record.
             */
            organization_id: string;
        };
        /**
         * @description Fields that tags can be ordered/grouped by.
         * @enum {string}
         */
        TagOrderField: "created_at" | "name" | "color" | "updated_at" | "is_application";
        /**
         * @description Company size bracket offered by the plan-inquiry form.
         * @enum {string}
         */
        TeamSize: "1-10" | "11-25" | "26-50" | "51-100" | "101-250" | "251-500" | "501-1000" | "1001+";
        /** @description Request to test reachability of a daemon URL. */
        TestReachabilityRequest: {
            /** @description If true, also perform an HTTP GET to {url}/health after the TCP check */
            check_health?: boolean;
            /** @description Full URL of the daemon (e.g. "https://daemon.example.com:60073") */
            url: string;
        };
        /** @description Response from a reachability test. */
        TestReachabilityResponse: {
            /** @description Error message if not reachable */
            error?: string | null;
            /** @description Health check result (only present when check_health was true) */
            health?: boolean | null;
            /** @description Whether the TCP connection succeeded */
            reachable: boolean;
        };
        Topology: components["schemas"]["TopologyBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        TopologyBase: {
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description Saved layout and view settings for this topology. */
            options: components["schemas"]["TopologyOptions"];
        };
        /**
         * @description Bundle of entities + the built graph that feed the topology render, export,
         *     and share pipelines.
         *
         *     Loaded by [`crate::server::topology::service::main::TopologyService::get_topology_data`]
         *     for either the live view (`snapshot_id = None`) or a point-in-time snapshot
         *     (`snapshot_id = Some(id)`). The per-view `nodes`/`edges` are built on request
         *     from these entities + the network's grouping options
         *     (`build_all_view_graphs`) — they are not persisted. The frontend selects the
         *     active view's slice client-side.
         */
        TopologyData: {
            /**
             * @description Views whose data is present in this entity set (L3/Workloads always;
             *     L2 Physical iff LLDP/CDP neighbors exist; Application iff app-flagged
             *     tags are used). The topology tab restricts a snapshot's view picker to
             *     these — you can't set up SNMP or create app tags on a historical
             *     snapshot — while the live view shows all views with setup prompts.
             */
            available_views?: components["schemas"]["TopologyView"][];
            /** @description Service bindings included in this topology. */
            bindings: components["schemas"]["Binding"][];
            /** @description Dependencies included in this topology. */
            dependencies: components["schemas"]["Dependency"][];
            /** @description Connections between the nodes of the built graph. */
            edges?: {
                [key: string]: components["schemas"]["Edge"][];
            };
            /** @description Hosts included in this topology. */
            hosts: components["schemas"]["Host"][];
            /** @description Interfaces included in this topology. */
            interfaces: components["schemas"]["Interface"][];
            /** @description IP addresses included in this topology. */
            ip_addresses: components["schemas"]["IPAddress"][];
            /**
             * @description Per-view graph built on request from the entities above + grouping
             *     options. Keyed by view so switching the active perspective is a
             *     client-side slice selection.
             */
            nodes?: {
                [key: string]: components["schemas"]["Node"][];
            };
            /** @description Ports included in this topology. */
            ports: components["schemas"]["Port"][];
            /** @description Services included in this topology. */
            services: components["schemas"]["Service"][];
            /** @description Subnets included in this topology. */
            subnets: components["schemas"]["Subnet"][];
            /** @description Tags assigned to this entity. */
            tags: components["schemas"]["Tag"][];
            /** @description VLANs included in this topology. */
            vlans: components["schemas"]["Vlan"][];
        };
        TopologyLocalOptions: {
            /**
             * @description Collapse parallel edges between the same pair of nodes into one.
             * @default true
             */
            bundle_edges: boolean;
            /**
             * @description Edge types to leave out of the drawing.
             * @default [
             *       "Hypervisor"
             *     ]
             */
            hide_edge_types: components["schemas"]["EdgeTypeDiscriminants"][];
            /**
             * @description Keep unrelated edges at full opacity when something is selected.
             * @default false
             */
            no_fade_edges: boolean;
            /**
             * @description Show the minimap.
             * @default true
             */
            show_minimap: boolean;
            /**
             * @default {
             *       "hidden_host_tag_ids": [],
             *       "hidden_service_tag_ids": [],
             *       "hidden_subnet_tag_ids": []
             *     }
             */
            tag_filter: components["schemas"]["TopologyTagFilter"];
        };
        TopologyOptions: {
            /** @description Settings applied in the viewer, which do not change what the server returns. */
            local: components["schemas"]["TopologyLocalOptions"];
            /** @description Settings that change how the server builds the graph. */
            request: components["schemas"]["TopologyRequestOptions"];
        };
        TopologyRequestOptions: {
            /**
             * @description Rules deciding how nodes are grouped into containers.
             * @default {
             *       "Application": [
             *         {
             *           "id": "ed91b0a7-a4aa-4d13-ac46-7391a1ee671f",
             *           "rule": {
             *             "ByApplication": {
             *               "tag_ids": []
             *             }
             *           }
             *         }
             *       ],
             *       "L2Physical": [
             *         {
             *           "id": "41f1b5e0-b0ca-428a-9678-00dab0be9101",
             *           "rule": "ByHost"
             *         }
             *       ],
             *       "L3Logical": [
             *         {
             *           "id": "fdb43ba1-1ab9-43ae-abc5-893485681291",
             *           "rule": "BySubnet"
             *         },
             *         {
             *           "id": "ec52c714-75f5-4780-9003-3ec8bfe99637",
             *           "rule": "MergeContainerBridges"
             *         }
             *       ],
             *       "Workloads": [
             *         {
             *           "id": "41f1b5e0-b0ca-428a-9678-00dab0be9101",
             *           "rule": "ByHost"
             *         }
             *       ]
             *     }
             */
            container_rules: {
                [key: string]: components["schemas"]["IdentifiedRule_ContainerRule"][];
            };
            /**
             * @description Rules deciding how entities are placed and inlined within containers.
             * @default [
             *       {
             *         "id": "9813531e-b5ef-4067-bf63-687a6496913f",
             *         "rule": "ByTrunkPort"
             *       },
             *       {
             *         "id": "db481467-a1d3-4496-b159-d4c08345ac91",
             *         "rule": "ByVLAN"
             *       },
             *       {
             *         "id": "e25e521b-0cfc-476b-91f8-c755cc010f69",
             *         "rule": "ByPortOpStatus"
             *       },
             *       {
             *         "id": "a35605ab-d9e1-439b-8e18-06c350dbfeee",
             *         "rule": {
             *           "ByServiceCategory": {
             *             "categories": [
             *               "NetworkCore",
             *               "NetworkAccess",
             *               "RemoteAccess",
             *               "Workstation",
             *               "Mobile",
             *               "Printer",
             *               "OpenPorts"
             *             ],
             *             "is_infra_rule": true,
             *             "title": "Infrastructure"
             *           }
             *         }
             *       },
             *       {
             *         "id": "07342dd6-9bbb-4fe6-9889-23746abb6c5d",
             *         "rule": {
             *           "ByTag": {
             *             "tag_ids": [],
             *             "title": null
             *           }
             *         }
             *       },
             *       {
             *         "id": "6adc6471-c148-4a48-aa19-ebd951421ba7",
             *         "rule": "ByHypervisor"
             *       },
             *       {
             *         "id": "2f28b9f4-49d6-40cb-9745-5a45f1f0ace0",
             *         "rule": "ByContainerRuntime"
             *       },
             *       {
             *         "id": "53e9f36d-50e5-487f-a976-8228e8399341",
             *         "rule": "ByStack"
             *       }
             *     ]
             */
            element_rules: components["schemas"]["IdentifiedRule_ElementRule"][];
            /**
             * @description Entity types hidden per view. Keyed by TopologyView, values are entity
             *     types (matching those declared as container/element/inline in the
             *     view's element_config). Hides every manifestation of the entity in
             *     that view — element nodes, container nodes, and inline rows on
             *     element cards. Supersedes the old `hide_ports` (L3-only, inline-only).
             * @default {}
             */
            hide_entities: {
                [key: string]: components["schemas"]["EntityDiscriminants"][];
            };
            /**
             * @description Generic per-(view, entity, filter) hide-set for metadata filters
             *     (Category, Virtualization, etc). Supersedes the old
             *     `hide_service_categories`; nested so JSON keys are strings all the
             *     way down.
             * @default {
             *       "Application": {
             *         "Service": {
             *           "Category": [
             *             "OpenPorts"
             *           ]
             *         }
             *       },
             *       "L2Physical": {
             *         "Interface": {
             *           "LinkState": [
             *             "Unlinked"
             *           ]
             *         },
             *         "Service": {
             *           "Category": [
             *             "OpenPorts"
             *           ]
             *         }
             *       },
             *       "L3Logical": {
             *         "Service": {
             *           "Category": [
             *             "OpenPorts"
             *           ]
             *         }
             *       },
             *       "Workloads": {
             *         "Service": {
             *           "Category": [
             *             "OpenPorts"
             *           ]
             *         }
             *       }
             *     }
             */
            hide_metadata_values: {
                [key: string]: {
                    [key: string]: {
                        [key: string]: string[];
                    };
                };
            };
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
        /**
         * @description Which topology view is being rendered
         * @enum {string}
         */
        TopologyView: "L2Physical" | "L3Logical" | "Workloads" | "Application";
        /** @enum {string} */
        TransportProtocol: "Udp" | "Tcp";
        /** @description No payload. Present only so the envelope keeps its shape. */
        TupleUnit: Record<string, never>;
        /** @description A neighbour advertised by a local interface whose far end could not be placed on a host. */
        UnmatchedNeighbour: {
            /**
             * Format: uuid
             * @description The local device that saw the neighbour, not the far end — the far end is what could not
             *     be identified.
             */
            host_id: string;
            /** @description The chassis ID (LLDP) or device id (CDP) that did not identify one host. */
            identifier: string;
            if_descr: string;
            sys_name: string | null;
        };
        /** @description A neighbour whose far-end host resolved but whose far-end *port* did not. */
        UnresolvedPort: {
            /**
             * Format: uuid
             * @description The local device that saw the neighbour, and the port it saw it on.
             */
            host_id: string;
            if_descr: string;
            /**
             * @description `lldpRemPortDesc`, the last-resort tier. Present because "the id failed and the description
             *     was empty" and "both were tried and neither matched" call for different fixes.
             */
            port_desc: string | null;
            /**
             * @description The advertised port id in `Debug` form, which carries subtype and value together
             *     (`MacAddress("00:ad:24:af:4e:00")`, `InterfaceName("2")`). Both halves are needed: the
             *     subtype says which tier ran and the value says what it looked for.
             */
            port_id: string | null;
            /**
             * Format: uuid
             * @description The far-end device, already resolved — this is what makes it distinct from
             *     [`UnmatchedNeighbour`].
             */
            remote_host_id: string;
        };
        /**
         * @description Request type for updating a host with its children.
         *     Uses the same input types as CreateHostRequest.
         *     Server will sync children (create new, update existing, delete removed) only if provided.
         */
        UpdateHostRequest: {
            /**
             * @description Credential assignments for this host.
             *     If provided, replaces all existing credential assignments.
             */
            credential_assignments?: components["schemas"]["CredentialAssignment"][] | null;
            /** @description Free-text notes about the host. */
            description?: string | null;
            /**
             * Format: date-time
             * @description Optional: expected updated_at timestamp for optimistic locking.
             */
            expected_updated_at?: string | null;
            /** @description Hide the host from topology views without deleting it. */
            hidden: boolean;
            /** @description Hostname as resolved or reported by the host. */
            hostname?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            id: string;
            /**
             * @description Interfaces to sync with this host.
             *     If Some, server will create/update/delete to match this list.
             *     If None, existing ip_addresses are preserved.
             */
            ip_addresses?: components["schemas"]["IPAddressInput"][] | null;
            /** @description Human-facing name for the host. */
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
            /** @description Tags assigned to this entity. */
            tags: string[];
            virtualization_metadata?: null | components["schemas"]["HostVirtualization"];
            /**
             * Format: uuid
             * @description The hypervisor service this VM runs on.
             */
            virtualization_service_id?: string | null;
        };
        UpdatePasswordRequest: {
            /**
             * Format: password
             * @description Current password — required if the user already has a password set.
             *     Not required for OIDC-only users adding their first password.
             */
            current_password?: string | null;
            /**
             * Format: password
             * @description New password to set
             */
            new_password: string;
        };
        /**
         * @description Whether the vendor publishes and supports the API a credential type talks to.
         *
         *     Deliberately *not* folded into [`CredentialStability`], because the two describe different
         *     things and change independently. Stability is about our own maturity and is meant to be retired
         *     by promotion to `Stable`; an undocumented upstream is a permanent property of the vendor's API
         *     that our promotion does not change. Collapsing them would force an integration built on a
         *     reverse-engineered API to sit in `Beta` forever to keep the warning — or to reach `Stable` with
         *     the warning silently dropped. UniFi is the proof that both combinations are real: it is
         *     `Stable` and `Undocumented` today.
         * @enum {string}
         */
        UpstreamSupport: "Vendor" | "Undocumented";
        /** @enum {string} */
        UseCase: "homelab" | "internal_it" | "msp" | "other";
        User: components["schemas"]["UserBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        UserApiKey: components["schemas"]["UserApiKeyBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
        };
        UserApiKeyBase: {
            /**
             * Format: date-time
             * @description When this record stops being valid.
             */
            expires_at?: string | null;
            /** @description Whether the key may still be used. Disabled keys are rejected. */
            is_enabled?: boolean;
            /** @description The stored key. Returned redacted except on creation and rotation. */
            readonly key: string;
            /**
             * Format: date-time
             * @description When this key was last used to authenticate.
             */
            readonly last_used: string | null;
            /** @description Human-facing name for this key. */
            name: string;
            /** @description Network IDs this key has access to (hydrated from junction table) */
            network_ids?: string[];
            /**
             * Format: uuid
             * @description The organization that owns this record.
             */
            organization_id: string;
            /** @description Role the key is limited to, which cannot exceed the user's own. */
            permissions?: components["schemas"]["UserOrgPermissions"];
            /** @description Tags assigned to this entity. */
            tags: string[];
            /**
             * Format: uuid
             * @description User the key acts on behalf of.
             */
            user_id: string;
        };
        /**
         * @description Response for user API key creation/rotation
         *     Contains the full API key record plus the plaintext key (shown only once)
         */
        UserApiKeyResponse: {
            /** @description The stored key record. */
            api_key: components["schemas"]["UserApiKey"];
            /**
             * Format: password
             * @description The plaintext API key - only returned once during creation or rotation
             */
            readonly key: string;
        };
        UserBase: {
            /**
             * Format: email
             * @description The user's email address, also their login identifier.
             */
            email: string;
            /** @description Per-user email preferences */
            email_settings?: components["schemas"]["EmailSettings"];
            /** @description Whether the user has verified their email address */
            email_verified?: boolean;
            /** @description Whether the user has a password set — computed from password_hash, never stored in DB */
            readonly has_password?: boolean;
            /** @description The networks this entity applies to. */
            network_ids: string[];
            /**
             * Format: date-time
             * @description When the account was linked to the identity provider.
             */
            oidc_linked_at?: string | null;
            /** @description Slug of the identity provider this account signs in through, when linked. */
            oidc_provider?: string | null;
            /**
             * Format: uuid
             * @description The organization that owns this record.
             */
            organization_id: string;
            /** @description The user's role within the organization. */
            permissions: components["schemas"]["UserOrgPermissions"];
            /**
             * Format: date-time
             * @description When the user accepted the terms of service.
             */
            readonly terms_accepted_at?: string | null;
        };
        /** @enum {string} */
        UserOrgPermissions: "Owner" | "Admin" | "Member" | "Viewer";
        /**
         * @description 2D unsigned coordinate. Used for node positions and sizes.
         *     Element node sizes are computed by the frontend (elkjs); the backend
         *     sets `Uxy::default()` for element nodes.
         */
        Uxy: {
            /** @description Horizontal position. */
            x: number;
            /** @description Vertical position. */
            y: number;
        };
        VCenterVirtualization: {
            /** @description vCenter managed object ID of the guest. */
            vm_id?: string | null;
            /** @description Guest name as configured in vCenter. */
            vm_name?: string | null;
        };
        /** @description Request to verify email using token */
        VerifyEmailRequest: {
            /** @description Single-use token from the verification email. */
            token: string;
        };
        /**
         * @description Health status for daemon versions.
         *
         *     Lifecycle order: `Current` → `Outdated` → `Deprecated` → `Unsupported`, with
         *     `Unknown` for daemons whose version the server has no record of.
         * @enum {string}
         */
        VersionHealthStatus: "Current" | "Outdated" | "Deprecated" | "Unsupported" | "Unknown";
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
        Vlan: components["schemas"]["VlanBase"] & {
            /**
             * Format: date-time
             * @description When this record was first created.
             */
            readonly created_at: string;
            /**
             * Format: uuid
             * @description The discovery that first observed this entity.
             */
            readonly first_discovery_id?: string | null;
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            readonly id: string;
            /**
             * Format: uuid
             * @description The most recent discovery that observed this entity.
             */
            readonly last_discovery_id?: string | null;
            /**
             * Format: date-time
             * @description When a discovery last observed this entity.
             */
            readonly last_seen_at?: string;
            /**
             * Format: uuid
             * @description Stable identifier shared by every revision of the same entity across its history.
             */
            readonly lineage_id?: string | null;
            /**
             * Format: date-time
             * @description When this record was last modified.
             */
            readonly updated_at: string;
            /**
             * Format: date-time
             * @description Start of the interval this revision was current for (SCD2 history).
             */
            readonly valid_from?: string;
            /**
             * Format: date-time
             * @description End of the interval this revision was current for. `null` while it is the live revision.
             */
            readonly valid_to?: string | null;
        };
        VlanBase: {
            /** @description Free-text notes about the VLAN. */
            description?: string | null;
            /** @description Human-facing name for this VLAN. */
            name: string;
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /**
             * Format: uuid
             * @description The organization that owns this record.
             */
            organization_id: string;
            /** @description How this VLAN came to be known — discovered, imported, or created by hand. */
            source?: components["schemas"]["EntitySource"];
            /**
             * @description Subnets associated with this VLAN, derived from discovered interface
             *     native-VLAN data via the `subnet_vlans` junction. Hydrated by
             *     `VlanService` on read; it is not a column on `vlans`, so anything sent
             *     here on create/update is ignored by `to_params`.
             */
            subnet_ids?: string[];
            /**
             * Format: int32
             * @description The 802.1Q VLAN number (1-4094)
             */
            vlan_number: number;
        };
        VlanDiscoveryItem: {
            /** @description VLAN name as configured on the device. */
            name: string;
            /**
             * Format: int32
             * @description 802.1Q VLAN ID.
             */
            vlan_number: number;
        };
        /** @description Request body for daemon VLAN discovery upsert */
        VlanDiscoveryRequest: {
            /**
             * Format: uuid
             * @description The network this entity belongs to.
             */
            network_id: string;
            /** @description VLANs observed by the daemon. */
            vlans: components["schemas"]["VlanDiscoveryItem"][];
        };
        /** @description Response for discovery upsert */
        VlanDiscoveryResponse: {
            /** @description Mapping of vlan_number → VLAN entity UUID */
            vlans: components["schemas"]["VlanDiscoveryResponseItem"][];
        };
        VlanDiscoveryResponseItem: {
            /**
             * Format: uuid
             * @description Server-assigned unique identifier.
             */
            id: string;
            /**
             * Format: int32
             * @description 802.1Q VLAN ID.
             */
            vlan_number: number;
        };
        /** @enum {string} */
        VlanOrderField: "created_at" | "name" | "vlan_number" | "updated_at";
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
    cancel_subscription: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CancelSubscriptionRequest"];
            };
        };
        responses: {
            /** @description Cancellation initiated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_CancelSubscriptionResponse"];
                };
            };
            /** @description No active subscription or billing not enabled */
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
    apply_discount_save_offer: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Discount applied */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Discount not configured or billing not enabled */
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
    extend_trial: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Trial extended */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Ineligible or billing not enabled */
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
    finalize_payment_method: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["FinalizePaymentMethodRequest"];
            };
        };
        responses: {
            /** @description Payment method finalized */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Billing not enabled or SetupIntent invalid */
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
    pause_subscription: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["PauseSubscriptionRequest"];
            };
        };
        responses: {
            /** @description Subscription paused */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description Ineligible or billing not enabled */
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
    create_payment_method_setup_intent: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description SetupIntent client secret */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_SetupIntentResponse"];
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
    reactivate_subscription: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Subscription reactivated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description No pending cancellation or billing not enabled */
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
    resume_subscription: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Subscription resumed */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_String"];
                };
            };
            /** @description No paused subscription or billing not enabled */
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
    get_save_offer_coupon: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Save-offer coupon terms, or null when not configured */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Option_SaveOfferCoupon"];
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
                "application/json": components["schemas"]["DaemonStatus"];
            };
        };
        responses: {
            /** @description Work request processed - returns (Option<Value>, bool) */
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
                "application/json": components["schemas"]["LegacyCapabilities"];
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
                        /** @description The page of results. Empty when nothing matched the query. */
                        data: components["schemas"]["DaemonApiKey"][];
                        /** @description Human-readable failure message. Omitted on success. */
                        error?: string | null;
                        /** @description API and server version metadata, plus pagination counters. */
                        meta: components["schemas"]["PaginatedApiMeta"];
                        /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
        /** @description Array of Daemon API Key IDs to delete */
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
                    "text/csv": string;
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
        /** @description Array of User API Key IDs to delete */
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
                    "text/csv": string;
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
                ip_address_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
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
                        /** @description The page of results. Empty when nothing matched the query. */
                        data: components["schemas"]["Binding"][];
                        /** @description Human-readable failure message. Omitted on success. */
                        error?: string | null;
                        /** @description API and server version metadata, plus pagination counters. */
                        meta: components["schemas"]["PaginatedApiMeta"];
                        /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
            /** @description Referenced port or ip_address does not exist */
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
                ip_address_id?: string | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
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
                    "text/csv": string;
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
            /** @description Referenced port or ip_address does not exist */
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
    get_all_credentials: {
        parameters: {
            query?: {
                /** @description Filter by credential type (e.g. `SnmpV2c`, `DockerProxy`). */
                type?: null | components["schemas"]["CredentialTypeDiscriminants"];
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["CredentialOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["CredentialOrderField"];
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
            /** @description List of credentials */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Credential"];
                };
            };
        };
    };
    create_credential: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Credential"];
            };
        };
        responses: {
            /** @description Credential created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Credential"];
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
    bulk_create_credentials: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Credential"][];
            };
        };
        responses: {
            /** @description Credentials created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vec_Credential"];
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
    bulk_delete_credentials: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Credential IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Credentials deleted successfully */
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
    export_credentials_csv: {
        parameters: {
            query?: {
                /** @description Filter by credential type (e.g. `SnmpV2c`, `DockerProxy`). */
                type?: null | components["schemas"]["CredentialTypeDiscriminants"];
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["CredentialOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["CredentialOrderField"];
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
            /** @description CSV file containing Credentials */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": string;
                };
            };
        };
    };
    get_by_id_credential: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Credential ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Credential found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Credential"];
                };
            };
            /** @description Credential not found */
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
    update_credential: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Credential ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Credential"];
            };
        };
        responses: {
            /** @description Credential updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Credential"];
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
            /** @description Credential not found */
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
    delete_credential: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Credential ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Credential deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Credential not found */
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
            /** @description daemons deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description daemon has active sessions */
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
    email_install_command: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["EmailInstallCommandRequest"];
            };
        };
        responses: {
            /** @description Email sent */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Email service not configured */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description User session required */
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
                    "text/csv": string;
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
            /** @description Daemon is live and already has a bound key */
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
    test_daemon_reachability: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["TestReachabilityRequest"];
            };
        };
        responses: {
            /** @description Reachability test result */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_TestReachabilityResponse"];
                };
            };
            /** @description Invalid URL */
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
    update_daemon: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Daemon"];
            };
        };
        responses: {
            /** @description daemon updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Daemon"];
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
            /** @description daemon not found */
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
                /** @description daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description daemon deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description daemon not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description daemon has active sessions */
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
    get_daemon_install_command: {
        parameters: {
            query: {
                /** @description `install` (with the api-key placeholder) or `reconfigure` (credential-free). */
                purpose: components["schemas"]["InstallCommandKind"];
                /** @description Log verbosity the daemon should run at (e.g. `info`, `debug`). */
                log_level?: string | null;
                /** @description Path the daemon should write its log file to. */
                log_file?: string | null;
                /** @description How often the daemon reports in, in seconds. */
                heartbeat_interval?: number | null;
                /** @description Address and port the daemon should listen on, for server-polled mode. */
                bind_address?: string | null;
                /** @description Accept a self-signed certificate when connecting back to the server. */
                allow_self_signed_certs?: boolean | null;
                /** @description Continue scanning targets that present an untrusted certificate. */
                accept_invalid_scan_certs?: boolean | null;
                /** @description Comma-separated interface names. */
                interfaces?: string | null;
                /** @description Comma-separated credential/integration tokens (for the docker-compose env). */
                credential_refs?: string | null;
            };
            header?: never;
            path: {
                /** @description daemon ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Install command */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_InstallArtifacts"];
                };
            };
            /** @description daemon not found */
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
    get_all_dependencies: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["DependencyOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["DependencyOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of dependencies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Dependency"];
                };
            };
        };
    };
    create_dependency: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Dependency"];
            };
        };
        responses: {
            /** @description Dependency created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Dependency"];
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
    bulk_delete_dependencies: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Dependency IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Dependencies deleted */
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
    export_dependencies_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["DependencyOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["DependencyOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Dependencies */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": string;
                };
            };
        };
    };
    get_dependency_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Dependency ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Dependency found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Dependency"];
                };
            };
            /** @description Dependency not found */
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
    update_dependency: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Dependency ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Dependency"];
            };
        };
        responses: {
            /** @description Dependency updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Dependency"];
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
            /** @description Dependency not found */
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
    delete_dependency: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Dependency ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Dependency deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Dependency not found */
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
    get_all_discoveries: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by daemon ID */
                daemon_id?: string | null;
                /**
                 * @description `true` returns only completed runs (the history view), `false` only the
                 *     configurations that produce them. Omit for both.
                 */
                historical?: boolean | null;
                /**
                 * @description Free-text search across the discovery's name and the name of the daemon
                 *     that runs it.
                 */
                search?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["DiscoveryOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["DiscoveryOrderField"];
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
            /** @description List of discoveries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Discovery"];
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
            /** @description discoveries deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_BulkDeleteResponse"];
                };
            };
            /** @description discovery has active session */
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
    export_discoveries_csv: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by daemon ID */
                daemon_id?: string | null;
                /**
                 * @description `true` returns only completed runs (the history view), `false` only the
                 *     configurations that produce them. Omit for both.
                 */
                historical?: boolean | null;
                /**
                 * @description Free-text search across the discovery's name and the name of the daemon
                 *     that runs it.
                 */
                search?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["DiscoveryOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["DiscoveryOrderField"];
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
            /** @description CSV file containing Discoveries */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": string;
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
            /** @description A session is already running for this discovery */
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
                /** @description discovery ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description discovery deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description discovery not found */
            404: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description discovery has active session */
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
    get_all_hosts: {
        parameters: {
            query?: {
                /** @description Filter by network ID */
                network_id?: string | null;
                /** @description Filter by specific entity IDs (for selective loading) */
                ids?: string[] | null;
                /** @description Filter by tag IDs (returns hosts that have ANY of the specified tags) */
                tag_ids?: string[] | null;
                /**
                 * @description Free-text search. Case-insensitive substring match against the host's
                 *     name, hostname and description, and against its IP addresses and the
                 *     names of services running on it.
                 */
                search?: string | null;
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only hosts discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the host's own network's window.
                 */
                stale?: boolean | null;
                /**
                 * @description `false` returns hosts with empty `ip_addresses`/`ports`/`services`/
                 *     `interfaces`. The children dominate the payload, so callers that only need
                 *     host identity — name pickers, id→name lookups, counts — should pass
                 *     `false`. Defaults to `true`, so existing callers are unaffected.
                 */
                include_children?: boolean | null;
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
        /** @description Array of Host IDs to delete */
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
                /**
                 * @description Free-text search. Case-insensitive substring match against the host's
                 *     name, hostname and description, and against its IP addresses and the
                 *     names of services running on it.
                 */
                search?: string | null;
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only hosts discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the host's own network's window.
                 */
                stale?: boolean | null;
                /**
                 * @description `false` returns hosts with empty `ip_addresses`/`ports`/`services`/
                 *     `interfaces`. The children dominate the payload, so callers that only need
                 *     host identity — name pickers, id→name lookups, counts — should pass
                 *     `false`. Defaults to `true`, so existing callers are unaffected.
                 */
                include_children?: boolean | null;
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
                    "text/csv": string;
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
                /**
                 * @description Free-text search. Case-insensitive substring match against the host's
                 *     name, hostname and description, and against its IP addresses and the
                 *     names of services running on it.
                 */
                search?: string | null;
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only hosts discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the host's own network's window.
                 */
                stale?: boolean | null;
                /**
                 * @description `false` returns hosts with empty `ip_addresses`/`ports`/`services`/
                 *     `interfaces`. The children dominate the payload, so callers that only need
                 *     host identity — name pickers, id→name lookups, counts — should pass
                 *     `false`. Defaults to `true`, so existing callers are unaffected.
                 */
                include_children?: boolean | null;
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
                    "application/zip": string;
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
    rescan_host: {
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
            /** @description Rescan session started */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DiscoveryUpdatePayload"];
                };
            };
            /** @description Host cannot be rescanned (never scanned, daemon gone, daemon unreachable, or daemon too old) */
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
    list_interfaces: {
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
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
                        /** @description The page of results. Empty when nothing matched the query. */
                        data: components["schemas"]["Interface"][];
                        /** @description Human-readable failure message. Omitted on success. */
                        error?: string | null;
                        /** @description API and server version metadata, plus pagination counters. */
                        meta: components["schemas"]["PaginatedApiMeta"];
                        /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
                "application/json": components["schemas"]["Interface"];
            };
        };
        responses: {
            /** @description If entry created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Interface"];
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
    bulk_delete_interfaces: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Interface IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Interfaces deleted */
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
    export_interfaces_csv: {
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
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
                    "text/csv": string;
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
                "application/json": components["schemas"]["Interface"];
            };
        };
        responses: {
            /** @description If entry updated successfully */
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
            /** @description Interface deleted */
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
            /** @description Recipient named but the caller has no address to send from */
            400: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
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
    list_ip_addresses: {
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of IP Addresses */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The page of results. Empty when nothing matched the query. */
                        data: components["schemas"]["IPAddress"][];
                        /** @description Human-readable failure message. Omitted on success. */
                        error?: string | null;
                        /** @description API and server version metadata, plus pagination counters. */
                        meta: components["schemas"]["PaginatedApiMeta"];
                        /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
                        success: boolean;
                    };
                };
            };
        };
    };
    create_ip_address: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["IPAddress"];
            };
        };
        responses: {
            /** @description IP address created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_IPAddress"];
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
    bulk_delete_ip_addresses: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of IP Address IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description IP addresses deleted successfully */
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
    export_ip_addresses_csv: {
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing IP Addresses */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": string;
                };
            };
        };
    };
    get_ip_address_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description IP Address ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description IP Address found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_IPAddress"];
                };
            };
            /** @description IP Address not found */
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
    update_ip_address: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description IP address ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["IPAddress"];
            };
        };
        responses: {
            /** @description IP address updated successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_IPAddress"];
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
            /** @description IP address not found */
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
    delete_ip_address: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description IP address ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description IP address deleted successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description IP address not found */
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
    get_all_networks: {
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
            /** @description List of networks */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The page of results. Empty when nothing matched the query. */
                        data: (components["schemas"]["NetworkBase"] & {
                            /**
                             * Format: date-time
                             * @description When this record was first created.
                             */
                            readonly created_at: string;
                            /**
                             * Format: int64
                             * @description `stale_after_hours` with the server's default already applied.
                             *
                             *     Computed, never stored (excluded from `to_params`). Published so the
                             *     frontend derives staleness from the *same* number the digest uses rather
                             *     than re-declaring the default in TypeScript, where the two could drift
                             *     and a host could read stale in the app but current in the digest email.
                             */
                            readonly effective_stale_after_hours?: number;
                            /**
                             * Format: uuid
                             * @description Server-assigned unique identifier.
                             */
                            readonly id: string;
                            /**
                             * Format: date-time
                             * @description When this record was last modified.
                             */
                            readonly updated_at: string;
                        })[];
                        /** @description Human-readable failure message. Omitted on success. */
                        error?: string | null;
                        /** @description API and server version metadata, plus pagination counters. */
                        meta: components["schemas"]["PaginatedApiMeta"];
                        /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
        /** @description Array of Network IDs to delete */
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
                    "text/csv": string;
                };
            };
        };
    };
    get_by_id_network: {
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
    daemon_prompt_response: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["DaemonPromptResponseRequest"];
            };
        };
        responses: {
            /** @description Response recorded */
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
    update_profile: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ProfileUpdateRequest"];
            };
        };
        responses: {
            /** @description Profile updated */
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
    submit_referral_source: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["ReferralSourceRequest"];
            };
        };
        responses: {
            /** @description Referral source recorded */
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
    delete_organization: {
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
            /** @description Organization deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Cannot delete another organization */
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
            /** @description Demo data population started */
            202: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DemoPopulateStatus"];
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
            /** @description Population already in progress */
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
    populate_demo_status: {
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
            /** @description Demo populate status */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_DemoPopulateStatus"];
                };
            };
            /** @description No demo-populate task for this organization */
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
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
                        /** @description The page of results. Empty when nothing matched the query. */
                        data: components["schemas"]["Port"][];
                        /** @description Human-readable failure message. Omitted on success. */
                        error?: string | null;
                        /** @description API and server version metadata, plus pagination counters. */
                        meta: components["schemas"]["PaginatedApiMeta"];
                        /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
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
                    "text/csv": string;
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
                /**
                 * @description Free-text search. Case-insensitive substring match against the service's
                 *     name and definition, and against the name of the host it runs on.
                 */
                search?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Only services exposed on one of these port numbers, over either protocol. */
                ports?: number[] | null;
                /** @description Exclude services belonging to these categories. */
                exclude_categories?: components["schemas"]["ServiceCategory"][] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only services discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the service's own network's window.
                 */
                stale?: boolean | null;
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
                /**
                 * @description Free-text search. Case-insensitive substring match against the service's
                 *     name and definition, and against the name of the host it runs on.
                 */
                search?: string | null;
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["ServiceOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Only services exposed on one of these port numbers, over either protocol. */
                ports?: number[] | null;
                /** @description Exclude services belonging to these categories. */
                exclude_categories?: components["schemas"]["ServiceCategory"][] | null;
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only services discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the service's own network's window.
                 */
                stale?: boolean | null;
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
                    "text/csv": string;
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
                        /** @description The page of results. Empty when nothing matched the query. */
                        data: components["schemas"]["Share"][];
                        /** @description Human-readable failure message. Omitted on success. */
                        error?: string | null;
                        /** @description API and server version metadata, plus pagination counters. */
                        meta: components["schemas"]["PaginatedApiMeta"];
                        /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
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
                    "text/csv": string;
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
            /** @description Password verified; access token issued */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_ShareAccessTokenResponse"];
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
    list_snapshots: {
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
            /** @description List of Snapshots */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": {
                        /** @description The page of results. Empty when nothing matched the query. */
                        data: components["schemas"]["Snapshot"][];
                        /** @description Human-readable failure message. Omitted on success. */
                        error?: string | null;
                        /** @description API and server version metadata, plus pagination counters. */
                        meta: components["schemas"]["PaginatedApiMeta"];
                        /** @description `true` when the request succeeded. `false` responses carry `error` instead of `data`. */
                        success: boolean;
                    };
                };
            };
        };
    };
    create_snapshot: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["CreateSnapshotRequest"];
            };
        };
        responses: {
            /** @description Snapshot created */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Snapshot"];
                };
            };
            /** @description Snapshots not available on plan */
            402: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiErrorResponse"];
                };
            };
            /** @description Network is busy with discovery; retry shortly */
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
    get_snapshot_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Snapshot ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Snapshot found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Snapshot"];
                };
            };
            /** @description Snapshot not found */
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
    delete_snapshot: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Snapshot ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Snapshot deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Snapshot not found */
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only subnets discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the subnet's own network's window.
                 */
                stale?: boolean | null;
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
                /**
                 * @description `true` returns only subnets discovery hasn't observed within their
                 *     network's staleness window; `false` returns only those it has. Omit for
                 *     both. Evaluated per row against the subnet's own network's window.
                 */
                stale?: boolean | null;
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
                    "text/csv": string;
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
            /** @description CIDR change would orphan existing ip_addresses */
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
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
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
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
                    "text/csv": string;
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
    get_topology_data: {
        parameters: {
            query: {
                /** @description Network to read entities for. Required. */
                network_id: string;
                /**
                 * @description When set, returns the entity set as it was when this snapshot was taken.
                 *     When omitted, returns live entities.
                 */
                snapshot_id?: string | null;
                /**
                 * @description When `true`, records the `FirstTopologyRebuild` onboarding milestone (the user has
                 *     viewed their topology). Only the frontend's explicit on-tab view sets this — the
                 *     background topology-data query never does — so the milestone never fires from other
                 *     tabs. One-time per org (guarded below + subscriber dedup).
                 */
                mark_viewed?: boolean | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Topology entity bundle */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_TopologyData"];
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
            /** @description Snapshot not found */
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
                    "text/csv": string;
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
    export_confluence: {
        parameters: {
            query?: {
                /** @description View to export. Defaults to the default view when omitted. */
                view?: components["schemas"]["TopologyView"];
            };
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Confluence wiki markup export */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
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
    export_mermaid: {
        parameters: {
            query?: {
                /** @description View to export. Defaults to the default view when omitted. */
                view?: components["schemas"]["TopologyView"];
            };
            header?: never;
            path: {
                /** @description Topology ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Mermaid flowchart export */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/plain": string;
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
        /** @description Array of User IDs to delete */
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
                    "text/csv": string;
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
    get_all_vlans: {
        parameters: {
            query?: {
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["VlanOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["VlanOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description List of VLANs */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["PaginatedApiResponse_Vlan"];
                };
            };
        };
    };
    create_vlan: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Vlan"];
            };
        };
        responses: {
            /** @description VLAN created successfully */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vlan"];
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
            /** @description VLAN number already exists in this network */
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
    bulk_delete_vlans: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        /** @description Array of Vlan IDs to delete */
        requestBody: {
            content: {
                "application/json": string[];
            };
        };
        responses: {
            /** @description Vlans deleted */
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
    discovery_upsert_vlans: {
        parameters: {
            query?: never;
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["VlanDiscoveryRequest"];
            };
        };
        responses: {
            /** @description VLANs upserted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_VlanDiscoveryResponse"];
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
    export_vlans_csv: {
        parameters: {
            query?: {
                /** @description Primary ordering field (used for grouping). Always sorts ASC to keep groups together. */
                group_by?: null | components["schemas"]["VlanOrderField"];
                /** @description Secondary ordering field (sorting within groups or standalone sort). */
                order_by?: null | components["schemas"]["VlanOrderField"];
                /** @description Direction for order_by field (group_by always uses ASC). */
                order_direction?: null | components["schemas"]["OrderDirection"];
                /** @description Maximum number of results to return (1-1000, default: 50). Use 0 for no limit. */
                limit?: number | null;
                /** @description Number of results to skip. Default: 0. */
                offset?: number | null;
                /** @description Filter by network ID */
                network_id?: string | null;
                /**
                 * @description As-of timestamp (ISO 8601). When set, returns SCD2 state as of this
                 *     instant (snapshot view) instead of live state.
                 */
                at?: string | null;
            };
            header?: never;
            path?: never;
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description CSV file containing Vlans */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "text/csv": string;
                };
            };
        };
    };
    get_vlan_by_id: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Vlan ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Vlan found */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vlan"];
                };
            };
            /** @description Vlan not found */
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
    update_vlan: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Vlan ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody: {
            content: {
                "application/json": components["schemas"]["Vlan"];
            };
        };
        responses: {
            /** @description Vlan updated */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse_Vlan"];
                };
            };
            /** @description Vlan not found */
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
    delete_vlan: {
        parameters: {
            query?: never;
            header?: never;
            path: {
                /** @description Vlan ID */
                id: string;
            };
            cookie?: never;
        };
        requestBody?: never;
        responses: {
            /** @description Vlan deleted */
            200: {
                headers: {
                    [name: string]: unknown;
                };
                content: {
                    "application/json": components["schemas"]["ApiResponse"];
                };
            };
            /** @description Vlan not found */
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
