import type { components } from '$lib/api/schema';
import {
	billingPlans,
	credentialTypes,
	entities,
	type FeatureId
} from '$lib/shared/stores/metadata';
import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

export type Credential = components['schemas']['Credential'];
export type CredentialBase = components['schemas']['CredentialBase'];
export type CredentialType = components['schemas']['CredentialType'];
export type CredentialOrderField = components['schemas']['CredentialOrderField'];
/** Which integrations run on a daemon, and against which hosts. */
export type IntegrationTarget = components['schemas']['IntegrationTarget'];
/** Whether the vendor publishes the API this credential type talks to. Derived from the backend
 *  enum, and independent of `Stability` — an integration can be stable and still ride
 *  an endpoint the vendor never documented. */
export type UpstreamSupport = components['schemas']['UpstreamSupport'];

// Re-export SNMP types still used by other features (Interface display, etc.)
export type Interface = components['schemas']['Interface'];
export type IfAdminStatus = components['schemas']['IfAdminStatus'];
export type IfOperStatus = components['schemas']['IfOperStatus'];

import type { Color } from '$lib/shared/utils/styling';
import type { TagProps } from '$lib/shared/components/data/types';
import {
	common_testing,
	common_unknown,
	credentials_unofficialApi,
	credentials_unofficialApiTooltip,
	credentials_targetSiteTooltip,
	credentials_targetDaemonHostTooltip,
	credentials_targetHostTooltip,
	credentials_targetLoginTooltip,
	snmp_adminStatusDown,
	snmp_adminStatusUp,
	snmp_operStatusDormant,
	snmp_operStatusDown,
	snmp_operStatusLowerLayerDown,
	snmp_operStatusNotPresent,
	snmp_operStatusUp
} from '$lib/paraglide/messages';

/**
 * Create a default credential with the given organization ID.
 * Defaults to the SNMP community type.
 */
export function createDefaultCredential(organization_id: string): Credential {
	return {
		name: '',
		description: null,
		credential_type: {
			type: 'SnmpCommunity',
			community: { mode: 'Inline' as const, value: '' }
		},
		daemon_os: null,
		login_credential_id: null,
		organization_id,
		tags: [],
		assigned_site_ids: [],
		host_assignments: [],
		used_by: [],
		is_default: false,
		// Read-only: the server sets it to Manual on create.
		source: { type: 'Manual' },
		id: uuidv4Sentinel,
		created_at: utcTimeZoneSentinel,
		updated_at: utcTimeZoneSentinel
	};
}

/**
 * Get the type discriminant from a credential's credential_type.
 */
export function getCredentialTypeId(credential: Credential): string {
	return credential.credential_type.type;
}

/**
 * Get a summary of non-secret fields for display on cards.
 */
export function getCredentialSummary(credential: Credential): string {
	const ct = credential.credential_type;
	switch (ct.type) {
		case 'SnmpCommunity':
			return '161/udp';
		case 'DockerProxy':
		case 'PodmanProxy':
			return `Port ${ct.port ?? 2376}`;
		default:
			return '';
	}
}

/**
 * Description shown under a credential in pickers and popovers: the one the user wrote, or the
 * credential type's description when there is none.
 */
export function getCredentialDescription(credential: Credential): string {
	return (
		credential.description?.trim() ||
		credentialTypes.getDescription(credential.credential_type.type)
	);
}

/**
 * Get human-readable labels for SNMP admin status
 */
export function getAdminStatusLabels(): Record<IfAdminStatus, string> {
	return {
		Up: snmp_adminStatusUp(),
		Down: snmp_adminStatusDown(),
		Testing: common_testing()
	};
}

/**
 * Get human-readable labels for SNMP operational status
 */
export function getOperStatusLabels(): Record<IfOperStatus, string> {
	return {
		Up: snmp_operStatusUp(),
		Down: snmp_operStatusDown(),
		Testing: common_testing(),
		Unknown: common_unknown(),
		Dormant: snmp_operStatusDormant(),
		NotPresent: snmp_operStatusNotPresent(),
		LowerLayerDown: snmp_operStatusLowerLayerDown()
	};
}

/**
 * Single source of truth for target display properties (color, label, tooltip).
 * Targets are the unified replacement for scope models.
 *
 * A scope chip, drawn as the entity it actually reaches.
 *
 * Each scope names a real entity, so it borrows that entity's colour *and* icon rather than an
 * arbitrary one: a site scope reads as a site, a daemon-host scope as a daemon, and a
 * remote-host scope as a host. That way a scope means the same thing here as the thing it points
 * at does everywhere else.
 *
 * Carries no label, so the chip renders as the icon alone. A type commonly has two or three of
 * these, and spelled out they crowd out the tags that vary — beta and unofficial-API — which are
 * the ones a reader has to notice. The name lives in `title` instead, which both explains the
 * scope on hover and keeps it searchable in the type dropdown.
 */
export function getTargetTagProps(target: string): TagProps {
	if (target === 'Site') {
		return {
			color: entities.getColorHelper('Site').color,
			icon: entities.getIconComponent('Site'),
			title: credentials_targetSiteTooltip()
		};
	}
	if (target === 'DaemonHost') {
		return {
			color: entities.getColorHelper('Daemon').color,
			icon: entities.getIconComponent('Daemon'),
			title: credentials_targetDaemonHostTooltip()
		};
	}
	return {
		color: entities.getColorHelper('Host').color,
		icon: entities.getIconComponent('Host'),
		title: credentials_targetHostTooltip()
	};
}

/**
 * The scope chips for a credential type: one per target, or for an SSH login, which has no
 * targets of its own, one chip saying it signs in for the credentials that use it.
 */
export function getScopeTagProps(typeId: string): TagProps[] {
	const metadata = credentialTypes.getMetadata(typeId);
	if (metadata?.role === 'Login') {
		return [
			{
				color: 'Gray',
				icon: credentialTypes.getIconComponent(typeId),
				title: credentials_targetLoginTooltip()
			}
		];
	}
	return (metadata?.targets ?? []).map((target: string) => getTargetTagProps(target));
}

/**
 * Tag marking a credential type that talks to an API its vendor does not publish, or `null` for
 * vendor-supported ones.
 *
 * Separate from `getStabilityTagProps` because the two say different things and change
 * independently: beta is about how far *we* have validated the integration and goes away when it
 * is promoted, while an undocumented upstream is a permanent property of the vendor's API. A
 * credential type can carry both tags, one, or neither.
 */
export function getUpstreamSupportTagProps(
	upstreamSupport: UpstreamSupport | undefined
): TagProps | null {
	if (upstreamSupport !== 'Undocumented') return null;
	return {
		label: credentials_unofficialApi(),
		// Gray rather than the amber Beta uses: this is a standing property of the vendor's API,
		// not a warning about our own maturity, and the two can appear side by side.
		color: 'Gray' as Color,
		title: credentials_unofficialApiTooltip()
	};
}

/**
 * The plan feature a new credential of `typeId` needs and an organization on `planType` lacks,
 * or `null` when it can create one. Only creating is gated: existing credentials of the type keep
 * working on any plan. An unknown plan gates nothing here; the server refuses the create anyway.
 */
export function credentialTypeUpgradeFeature(
	typeId: string,
	planType: string | null | undefined
): FeatureId | null {
	const required = credentialTypes.getMetadata(typeId)?.required_feature;
	if (!required || !planType) return null;
	return billingPlans.getMetadata(planType)?.features?.[required] ? null : required;
}

/**
 * A credential type that applies *only* to the daemon's own host (e.g. the Docker/Podman
 * socket). Derived from the target scheme — this replaces the former `is_local_auto` flag.
 * Such types target only the daemon host (a `<uuid>@127.0.0.1` token) — no remote/site scope.
 */
export function isDaemonHostOnly(targets: string[] | undefined): boolean {
	return targets?.length === 1 && targets[0] === 'DaemonHost';
}

/** An SSH login: other credentials run over it, and it is never assigned to sites or hosts. */
export function isLoginType(typeId: string): boolean {
	return credentialTypes.getMetadata(typeId)?.role === 'Login';
}

/** A type whose credentials name the SSH login they run over. */
export function requiresLogin(typeId: string): boolean {
	return credentialTypes.getMetadata(typeId)?.requires_login === true;
}

/**
 * Human-readable labels for SNMP admin status
 * @deprecated Use getAdminStatusLabels() instead for proper i18n support
 */
export const ADMIN_STATUS_LABELS: Record<IfAdminStatus, string> = {
	Up: 'Admin Up',
	Down: 'Admin Down',
	Testing: 'Testing'
};

/**
 * Human-readable labels for SNMP operational status
 * @deprecated Use getOperStatusLabels() instead for proper i18n support
 */
export const OPER_STATUS_LABELS: Record<IfOperStatus, string> = {
	Up: 'Up',
	Down: 'Down',
	Testing: 'Testing',
	Unknown: 'Unknown',
	Dormant: 'Dormant',
	NotPresent: 'Not Present',
	LowerLayerDown: 'Lower Layer Down'
};
