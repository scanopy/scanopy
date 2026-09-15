/**
 * Copy text by selecting it in an off-screen field. `navigator.clipboard` is gated to secure
 * contexts and Scanopy supports plain-HTTP self-hosts, so this is the path those pages use.
 */
export function copyViaSelection(text: string): boolean {
	const field = document.createElement('textarea');
	field.value = text;
	// Off-screen rather than `display: none`: a hidden field cannot be selected.
	field.setAttribute('readonly', '');
	field.style.position = 'fixed';
	field.style.opacity = '0';
	document.body.appendChild(field);
	try {
		field.select();
		return document.execCommand('copy');
	} finally {
		field.remove();
	}
}
