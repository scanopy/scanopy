import pkg from 'ipaddr.js';
import { validate } from 'email-validator';

const { isValid, isValidCIDR, parse, parseCIDR } = pkg;

// ============================================================================
// TanStack Form Validators
// These return `string | undefined` - undefined means valid, string is error
// ============================================================================

/** Form field value type - validators should accept any field value */
export type FormValue = string | number | boolean | null | undefined;

/** Validator function type */
export type Validator = (value: FormValue) => string | undefined;

/** Required field validator */
export function required(value: FormValue): string | undefined {
	if (value === null || value === undefined) return 'This field is required';
	if (typeof value === 'string') return !value.trim() ? 'This field is required' : undefined;
	return undefined;
}

/** Email format validator */
export function email(value: FormValue): string | undefined {
	if (!value || typeof value !== 'string') return undefined;
	return !validate(value) ? 'Please enter a valid email address' : undefined;
}

/** Maximum length validator */
export function max(length: number): Validator {
	return (value: FormValue) => {
		if (!value || typeof value !== 'string') return undefined;
		return value.length > length ? `Must be less than ${length} characters` : undefined;
	};
}

/** Minimum length validator */
export function min(length: number): Validator {
	return (value: FormValue) => {
		if (!value || typeof value !== 'string') return undefined;
		return value.length < length ? `Must be at least ${length} characters` : undefined;
	};
}

/** CIDR notation validator */
export function cidrNotation(value: FormValue): string | undefined {
	if (!value || typeof value !== 'string') return undefined;
	return !isValidCIDR(value) ? 'Invalid CIDR notation (e.g., 192.168.1.0/24)' : undefined;
}

/** IP address validator */
export function ipAddressFormat(value: FormValue): string | undefined {
	if (!value || typeof value !== 'string') return undefined;
	return !isValid(value) ? 'Invalid IP address format' : undefined;
}

/** IP address within CIDR range validator */
export function ipAddressInCidrFormat(cidr: string): Validator {
	return (value: FormValue) => {
		if (!value || typeof value !== 'string') return undefined;
		if (!isValidCIDR(cidr)) return `${cidr} is not valid CIDR notation`;
		if (!isValid(value)) return 'Invalid IP address format';
		if (!parse(value).match(parseCIDR(cidr))) {
			return `IP must be within ${cidr}`;
		}
		return undefined;
	};
}

/** MAC address validator */
export function macAddress(value: FormValue): string | undefined {
	if (!value || typeof value !== 'string') return undefined;
	const macRegex = /^([0-9A-Fa-f]{2}[:-]){5}([0-9A-Fa-f]{2})$/;
	return !macRegex.test(value) ? 'Invalid MAC address format' : undefined;
}

/** MAC address format validator (alias for macAddress) */
export const macFormat = macAddress;

/** Hostname validator */
export function hostnameFormat(value: FormValue): string | undefined {
	if (!value || typeof value !== 'string') return undefined;
	const hostnameRegex =
		/^([a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?\.)*[a-zA-Z0-9]([a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?$/;
	return !hostnameRegex.test(value.trim()) ? 'Please enter a valid hostname' : undefined;
}

/** Port range validator (1-65535) */
export function port(value: FormValue): string | undefined {
	if (value === null || value === undefined || value === '') return undefined;
	const portNum =
		typeof value === 'string' ? parseInt(value) : typeof value === 'number' ? value : NaN;
	return !Number.isInteger(portNum) || portNum < 1 || portNum > 65535
		? 'Port must be between 1 and 65535'
		: undefined;
}

/** Port range validation (alias for port) */
export const portRangeValidation = port;

/** URL format validator */
export function url(value: FormValue): string | undefined {
	if (!value || typeof value !== 'string') return undefined;
	try {
		const parsed = new URL(value);
		// Require hostname to have at least one dot (e.g., example.com) or be localhost
		const hostname = parsed.hostname;
		if (hostname !== 'localhost' && !hostname.includes('.')) {
			return 'Please enter a valid URL with a domain (e.g., https://example.com)';
		}
		return undefined;
	} catch {
		return 'Please enter a valid URL';
	}
}

/** URL format validator that rejects URLs with explicit ports */
export function urlWithoutPort(value: FormValue): string | undefined {
	if (!value || typeof value !== 'string') return undefined;
	try {
		const parsed = new URL(value);
		// Require hostname to have at least one dot (e.g., example.com) or be localhost
		const hostname = parsed.hostname;
		if (hostname !== 'localhost' && !hostname.includes('.')) {
			return 'Please enter a valid URL with a domain (e.g., https://example.com)';
		}
		// Reject URLs with explicit ports
		if (parsed.port) {
			return 'URL should not include a port. Use the Port field below.';
		}
		return undefined;
	} catch {
		return 'Please enter a valid URL';
	}
}

/** Numeric value validator */
export function numeric(value: FormValue): string | undefined {
	if (!value) return undefined;
	return isNaN(Number(value)) ? 'Must be a number' : undefined;
}

/** Password complexity validator */
export function password(value: FormValue): string | undefined {
	if (!value || typeof value !== 'string') return undefined;
	const hasUppercase = /[A-Z]/.test(value);
	const hasLowercase = /[a-z]/.test(value);
	const hasNumber = /[0-9]/.test(value);
	const longEnough = value.length >= 10;
	if (!longEnough) return 'Password must be at least 10 characters';
	if (!hasUppercase || !hasLowercase || !hasNumber) {
		return 'Password must contain uppercase, lowercase, and number';
	}
	return undefined;
}

/** Confirm password match validator */
export function confirmPasswordMatch(getPassword: () => string): Validator {
	return (value: FormValue) => {
		const passwordValue = getPassword();
		if (!passwordValue && !value) return undefined;
		if (passwordValue && !value) return 'Please confirm your password';
		if (typeof value !== 'string') return undefined;
		if (value !== passwordValue) return 'Passwords do not match';
		return undefined;
	};
}

/** Pattern matcher validator */
export function pattern(regex: RegExp, message: string): Validator {
	return (value: FormValue) => {
		if (!value || typeof value !== 'string') return undefined;
		return !regex.test(value) ? message : undefined;
	};
}

/** PEM certificate format validator */
export function pemCertificate(value: FormValue): string | undefined {
	if (!value || typeof value !== 'string') return undefined;
	const trimmed = value.trim();
	if (!trimmed) return undefined;
	if (
		!trimmed.startsWith('-----BEGIN CERTIFICATE-----') ||
		!trimmed.endsWith('-----END CERTIFICATE-----')
	) {
		return 'Invalid PEM certificate format. Must start with "-----BEGIN CERTIFICATE-----" and end with "-----END CERTIFICATE-----"';
	}
	return undefined;
}

/** PEM private key format validator */
export function pemPrivateKey(value: FormValue): string | undefined {
	if (!value || typeof value !== 'string') return undefined;
	const trimmed = value.trim();
	if (!trimmed) return undefined;
	const validStarts = [
		'-----BEGIN PRIVATE KEY-----',
		'-----BEGIN RSA PRIVATE KEY-----',
		'-----BEGIN EC PRIVATE KEY-----'
	];
	const validEnds = [
		'-----END PRIVATE KEY-----',
		'-----END RSA PRIVATE KEY-----',
		'-----END EC PRIVATE KEY-----'
	];
	if (
		!validStarts.some((s) => trimmed.startsWith(s)) ||
		!validEnds.some((e) => trimmed.endsWith(e))
	) {
		return 'Invalid PEM private key format. Must have a valid PRIVATE KEY PEM envelope';
	}
	return undefined;
}

/** Array minimum length validator */
export function minArrayLength(
	length: number,
	message?: string
): (value: unknown[] | null | undefined) => string | undefined {
	return (value: unknown[] | null | undefined) => {
		if (!value || !Array.isArray(value)) return message ?? `At least ${length} item(s) required`;
		return value.length < length ? (message ?? `At least ${length} item(s) required`) : undefined;
	};
}
