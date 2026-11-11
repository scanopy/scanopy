import { defineEnvVars } from '@sveltejs/kit/env';

// All three are optional: an unset variable reads as '', and the API client then uses the page's
// own origin.
export const variables = defineEnvVars({
	PUBLIC_SERVER_HOSTNAME: { public: true, schema: (input) => input ?? '' },
	PUBLIC_SERVER_PROTOCOL: { public: true, schema: (input) => input ?? '' },
	PUBLIC_SERVER_PORT: { public: true, schema: (input) => input ?? '' }
});
