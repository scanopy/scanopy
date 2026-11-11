import { hosts_unnamedHost } from '#lib/paraglide/messages.js';
import type { Host } from './types/base';

/**
 * What to call a host. The only function on the frontend that answers that question.
 *
 * `display_name` is `Host::display_name` resolved on the server — `name`, else `hostname`, else
 * sysName, else chassis id, else the host's first address. Reading it here rather than walking
 * those rungs in TypeScript is the whole point: the ladder is backend logic, and a second copy of
 * it would be free to disagree with the first.
 *
 * A host whose `name` is blank is not an edge case — controller-imported devices and LLDP far ends
 * arrive that way by construction — and `name` renders as the empty string for all of them. So the
 * rule is: nothing displays `host.name`.
 *
 * Takes only `display_name`, so a `Host` and a `HostResponse` are both acceptable, and takes a host
 * rather than an optional one on purpose: a host that could not be *found* is
 * `common_unknownEntity(...)`, not a host without a name, and squashing the two here would hide
 * broken lookups behind a plausible-looking label.
 */
export function hostDisplayName(host: Pick<Host, 'display_name'>): string {
	return host.display_name?.trim() || hosts_unnamedHost();
}
