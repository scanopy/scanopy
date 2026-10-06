/**
 * Grouping the host list by virtualization tree: every host under the host at the top of its
 * chain (a Proxmox node above its VMs above their containers), each host that runs others heading
 * a collapsible section of them.
 *
 * The tree itself comes from the server, which orders the rows across pages and sends each host's
 * root, parent and depth. This module only reads those fields: the group key has to match the
 * server's group counts exactly, and the header has to name the root even when it sits on another
 * page.
 */
import type { TreeConfig } from '$lib/shared/components/data/types';
import { hostDisplayName } from './host-display-name';
import type { Host } from './types/base';

type TreeHost = Pick<
	Host,
	| 'id'
	| 'display_name'
	| 'virtualization_parent_host_id'
	| 'virtualization_root_host_id'
	| 'virtualization_depth'
>;

/** The key the server groups a host under: its tree's root id, `''` for a host in no tree. */
export function virtualizationGroupKey(host: TreeHost): string {
	return host.virtualization_root_host_id ?? '';
}

/** The roots these hosts group under that aren't among them, so their names must be fetched. */
export function missingRootIds(hosts: TreeHost[]): string[] {
	const present = new Set(hosts.map((host) => host.id));
	const missing = new Set<string>();
	for (const host of hosts) {
		const root = host.virtualization_root_host_id;
		if (root && !present.has(root)) missing.add(root);
	}
	return [...missing];
}

/**
 * The header a host groups under: its root's title, `notVirtualized` for a host in no tree, and
 * `unknownRoot` while the root's name hasn't arrived.
 */
export function virtualizationGroupLabel(
	host: TreeHost,
	roots: ReadonlyMap<string, Pick<Host, 'display_name'>>,
	labels: { notVirtualized: string; unknownRoot: string }
): string {
	const root = host.virtualization_root_host_id;
	if (!root) return labels.notVirtualized;
	const rootHost = roots.get(root);
	return rootHost ? hostDisplayName(rootHost) : labels.unknownRoot;
}

/** The tree the host list draws when grouped this way. */
export const virtualizationTree: TreeConfig<TreeHost> = {
	key: (host) => host.id,
	parentKey: (host) => host.virtualization_parent_host_id ?? null,
	depth: (host) => host.virtualization_depth ?? 0
};
