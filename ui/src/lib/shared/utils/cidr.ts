/**
 * Range arithmetic on CIDRs, for the places the UI has to reason about one range against another.
 *
 * `ipaddr.js` is already a dependency and already used for the address validators; this only gives
 * its `parseCIDR`/`match` pair the shape the callers want, so nobody hand-rolls prefix maths.
 */

// Default import, then destructure. `ipaddr.js` is CommonJS, so Vite's SSR transform cannot bind
// named exports from it and a `import { parseCIDR }` form fails at run time rather than at build.
// `shared/components/forms/validators.ts` takes the same shape against the same package.
import pkg from 'ipaddr.js';

const { parseCIDR, isValidCIDR } = pkg;

/**
 * Whether `outer` covers every address in `inner`. A range contains itself.
 *
 * Both halves are needed: `match` alone only says the inner network address falls inside `outer`,
 * which is also true when `inner` is the *wider* of the two and merely starts in the same place —
 * `10.20.30.0/23` "matches" `10.20.30.0/24` in both directions. The prefix comparison is what makes
 * the answer directional.
 *
 * Returns `false` rather than throwing for anything unparseable or for a v4/v6 pair, since callers
 * are asking a yes/no question about data they did not author.
 */
export function cidrContains(outer: string, inner: string): boolean {
	if (!isValidCIDR(outer) || !isValidCIDR(inner)) return false;
	try {
		const [outerAddr, outerPrefix] = parseCIDR(outer);
		const [innerAddr, innerPrefix] = parseCIDR(inner);
		if (outerAddr.kind() !== innerAddr.kind()) return false;
		return outerPrefix <= innerPrefix && innerAddr.match([outerAddr, outerPrefix]);
	} catch {
		return false;
	}
}

/**
 * Orders two CIDRs by family, then network address, then prefix length, widest first.
 *
 * That order is the tree order: a range sorts directly before every range it contains, because it
 * starts at or before them and, where it starts at the same address, is the wider of the two. Text
 * order is not: it puts `10.0.16.0/20` ahead of `10.0.2.0/24`.
 *
 * Unparseable values sort after every valid one, and among themselves as text.
 */
export function compareCidr(a: string, b: string): number {
	const pa = isValidCIDR(a) ? parseCIDR(a) : null;
	const pb = isValidCIDR(b) ? parseCIDR(b) : null;
	if (!pa || !pb) {
		if (pa) return -1;
		if (pb) return 1;
		return a.localeCompare(b);
	}

	const [addrA, prefixA] = pa;
	const [addrB, prefixB] = pb;
	if (addrA.kind() !== addrB.kind()) return addrA.kind() === 'ipv4' ? -1 : 1;

	const bytesA = addrA.toByteArray();
	const bytesB = addrB.toByteArray();
	for (let i = 0; i < bytesA.length; i++) {
		if (bytesA[i] !== bytesB[i]) return bytesA[i] - bytesB[i];
	}
	return prefixA - prefixB;
}
