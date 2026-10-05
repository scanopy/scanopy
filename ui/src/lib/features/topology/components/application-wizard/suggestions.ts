/**
 * Suggested application group names based on organization use case.
 */
export const USE_CASE_SUGGESTIONS: Record<string, string[]> = {
	homelab: [
		'Media',
		'Home Automation',
		'Monitoring',
		'Site Infrastructure',
		'Development',
		'Gaming',
		'Productivity',
		'Backup',
		'Security/Cameras',
		'DNS/Ad Blocking',
		'VPN',
		'File Storage',
		'Other'
	],
	internal_it: [
		'Production',
		'Staging',
		'Internal Tools',
		'Monitoring',
		'CI/CD',
		'Shared Services',
		'Authentication',
		'Database',
		'API Gateway',
		'Logging',
		'Security',
		'Backup',
		'Other'
	],
	msp: [
		'Shared Infrastructure',
		'Monitoring',
		'Backup',
		'Security',
		'VPN',
		'DNS',
		'Logging',
		'Authentication',
		'File Storage',
		'Communication',
		'Other'
	]
};

export const DEFAULT_SUGGESTIONS = [
	'Web Services',
	'Database',
	'Monitoring',
	'Storage',
	'Infrastructure',
	'Authentication',
	'Logging',
	'Backup',
	'Other'
];

export function getSuggestions(useCase: string | null | undefined): string[] {
	if (useCase && useCase in USE_CASE_SUGGESTIONS) {
		return USE_CASE_SUGGESTIONS[useCase];
	}
	return DEFAULT_SUGGESTIONS;
}
