// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	const __APP_VERSION__: string;
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}
}

declare module 'freemail' {
	export function isFree(email: string): boolean;
	export function disposable(email: string): boolean;
}

declare module '$lib/data/billing-plans.json' {
	interface BillingPlanFixture {
		id: string;
		name: string;
		description: string;
		category: string | null;
		icon: string;
		color: string;
		metadata: {
			base_cents: number;
			rate: 'Month' | 'Year';
			trial_days: number;
			seat_cents: number | null;
			site_cents: number | null;
			included_seats: number | null;
			included_sites: number | null;
			features: Record<string, boolean>;
			is_commercial: boolean;
			hosting: string;
			custom_price: string | null;
		};
	}
	const data: BillingPlanFixture[];
	export default data;
}

declare module '$lib/data/billing-plans-all.json' {
	interface BillingPlanAllFixture {
		id: string;
		name: string;
		description: string;
		category: string | null;
		icon: string;
		color: string;
		metadata: {
			features: Record<string, boolean>;
			is_commercial: boolean;
			hosting: string;
			custom_price: string | null;
		};
	}
	const data: BillingPlanAllFixture[];
	export default data;
}

declare module '$lib/data/features.json' {
	interface FeatureFixture {
		id: string;
		name: string;
		description: string;
		category: string;
		icon: string;
		color: string;
		metadata: {
			is_coming_soon: boolean;
		};
	}
	const data: FeatureFixture[];
	export default data;
}

export {};
