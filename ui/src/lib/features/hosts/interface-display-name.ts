import { hosts_unnamedInterface } from '$lib/paraglide/messages';
import type { Interface } from './types/base';

/**
 * What to call an interface. The only function on the frontend that answers that question.
 *
 * `display_name` is `Interface::display_name` resolved on the server — `if_alias`, else
 * `if_descr`, else the MAC it was identified by (a PROFINET DCP identify has neither name field,
 * only a MAC), else the literal "Interface". Reading it here rather than walking those rungs in
 * TypeScript is the whole point: the ladder is backend logic, and a second copy of it would be
 * free to disagree with the first — which is exactly how this interface used to render as the
 * literal string "Interface null" (`if_descr || \`Interface ${if_index}\`` interpolating two
 * absent fields, one of three separate reimplementations that all disagreed with each other and
 * with the backend).
 *
 * Only present on interfaces nested under a host response (`HostResponse.interfaces`) — absent on
 * the standalone `/interfaces` CRUD responses, which return `Interface` without this computation.
 * The fallback below covers that gap defensively, the same way `hostDisplayName` does for a host.
 */
export function interfaceDisplayName(iface: Pick<Interface, 'display_name'>): string {
	return iface.display_name?.trim() || hosts_unnamedInterface();
}
