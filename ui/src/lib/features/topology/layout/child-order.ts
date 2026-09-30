import type { ElkNode } from 'elkjs';

/**
 * Put a container's children in `rank` order and keep them there through box packing.
 *
 * ELK's box algorithm places children by `elk.priority`, highest first, and only then by size, so a
 * descending priority per position fixes the reading order: left to right, top to bottom.
 * Children missing from `rank` go last.
 */
export function pinChildOrder(children: ElkNode[], rank: Map<string, number>): void {
	const rankOf = (child: ElkNode) => rank.get(child.id) ?? Number.MAX_SAFE_INTEGER;
	children.sort((a, b) => rankOf(a) - rankOf(b));
	children.forEach((child, index) => {
		child.layoutOptions = {
			...child.layoutOptions,
			'elk.priority': String(children.length - index)
		};
	});
}
