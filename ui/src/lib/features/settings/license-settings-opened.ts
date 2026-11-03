/**
 * The licensed plan Settings last opened on the License tab for, per org, so a reload doesn't
 * reopen it. Kept per browser: a new browser opens it once more.
 */

const KEY_PREFIX = 'license_settings_opened:';

export function licenseSettingsOpenedFor(orgId: string): string | null {
	try {
		return localStorage.getItem(KEY_PREFIX + orgId);
	} catch {
		return null;
	}
}

export function markLicenseSettingsOpened(orgId: string, planType: string | null): void {
	try {
		if (planType == null) localStorage.removeItem(KEY_PREFIX + orgId);
		else localStorage.setItem(KEY_PREFIX + orgId, planType);
	} catch {
		// Storage unavailable: Settings opens again on the next load.
	}
}
