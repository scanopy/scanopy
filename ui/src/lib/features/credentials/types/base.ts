import type { components } from '$lib/api/schema';
import { credentialTypes, entities } from '$lib/shared/stores/metadata';
import { utcTimeZoneSentinel, uuidv4Sentinel } from '$lib/shared/utils/formatting';

export type Credential = components['schemas']['Credential'];
export type CredentialBase = components['schemas']['CredentialBase'];
export type CredentialType = components['schemas']['CredentialType'];
export type CredentialOrderField = components['schemas']['CredentialOrderField'];
/** Which integrations run on a daemon, and against which hosts. */
export type IntegrationTarget = components['schemas']['IntegrationTarget'];
/** Release maturity of a credential type's integration. Derived from the backend enum. */
export type CredentialStability = components['schemas']['CredentialStability'];
/** Whether the vendor publishes the API this credential type talks to. Derived from the backend
 *  enum, and independent of `CredentialStability` — an integration can be stable and still ride
 *  an endpoint the vendor never documented. */
export type UpstreamSupport = components['schemas']['UpstreamSupport'];

// Re-export SNMP types still used by other features (Interface display, etc.)
export type Interface = components['schemas']['Interface'];
export type IfAdminStatus = components['schemas']['IfAdminStatus'];
export type IfOperStatus = components['schemas']['IfOperStatus'];

import type { Color } from '$lib/shared/utils/styling';
import type { TagProps } from '$lib/shared/components/data/types';
import {
	common_beta,
	common_testing,
	common_unknown,
	credentials_betaTooltip,
	credentials_unofficialApi,
	credentials_unofficialApiTooltip,
	credentials_targetNetworkTooltip,
	credentials_targetDaemonHostTooltip,
	credentials_targetHostTooltip,
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
 * Defaults to SNMPv2c type.
 */
export function createDefaultCredential(organization_id: string): Credential {
	return {
		name: '',
		description: null,
		credential_type: {
			type: 'SnmpV2c',
			community: { mode: 'Inline' as const, value: '' }
		},
		daemon_os: null,
		organization_id,
		tags: [],
		assigned_network_ids: [],
		host_assignments: [],
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
		case 'SnmpV2c':
			return '161/udp';
		case 'DockerProxy':
		case 'PodmanProxy':
			return `Port ${ct.port ?? 2376}`;
		default:
			return '';
	}
}

/**
 * Get the associated service name for display in credential lists and popovers.
 * Returns the service name from the associated ServiceDefinition (e.g. "SNMP", "Docker").
 */
export function getCredentialDescription(credential: Credential): string {
	const ct = credential.credential_type;
	return credentialTypes.getDescription(ct.type);
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
 * arbitrary one: a network scope reads as a network, a daemon-host scope as a daemon, and a
 * remote-host scope as a host. That way a scope means the same thing here as the thing it points
 * at does everywhere else.
 *
 * Carries no label, so the chip renders as the icon alone. A type commonly has two or three of
 * these, and spelled out they crowd out the tags that vary — beta and unofficial-API — which are
 * the ones a reader has to notice. The name lives in `title` instead, which both explains the
 * scope on hover and keeps it searchable in the type dropdown.
 */
export function getTargetTagProps(target: string): TagProps {
	if (target === 'Network') {
		return {
			color: entities.getColorHelper('Network').color,
			icon: entities.getIconComponent('Network'),
			title: credentials_targetNetworkTooltip()
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
 * Tag marking a credential type whose integration is still in beta, or `null` for stable
 * types. Beta is presentation only — the type stays selectable and usable; it is flagged so
 * users know the integration is unvalidated and its fields may still change.
 *
 * Lives here rather than in the template so the i18n lookup sits next to `getTargetTagProps`,
 * the other producer of credential-type tags.
 */
export function getStabilityTagProps(stability: CredentialStability | undefined): TagProps | null {
	if (stability !== 'Beta') return null;
	return {
		label: common_beta(),
		color: 'Amber' as Color,
		title: credentials_betaTooltip()
	};
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
 * A credential type that applies *only* to the daemon's own host (e.g. the Docker/Podman
 * socket). Derived from the target scheme — this replaces the former `is_local_auto` flag.
 * Such types target only the daemon host (a `<uuid>@127.0.0.1` token) — no remote/network scope.
 */
export function isDaemonHostOnly(targets: string[] | undefined): boolean {
	return targets?.length === 1 && targets[0] === 'DaemonHost';
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
