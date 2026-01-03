import { computePosition, flip, shift, offset, arrow } from '@floating-ui/dom';

export interface TooltipOptions {
	content: string;
	placement?: 'top' | 'bottom' | 'left' | 'right';
	/** Additional CSS class for tooltip variants (e.g., 'error-tooltip', 'paste-tooltip') */
	variant?: string;
	/** If true, content is treated as HTML (use with caution) */
	html?: boolean;
}

/**
 * Portal-based tooltip action using Floating UI.
 * Renders tooltip to document.body to avoid clipping from editor overflow.
 */
export function tooltip(node: HTMLElement, options: TooltipOptions) {
	let tooltipEl: HTMLElement | null = null;
	let arrowEl: HTMLElement | null = null;
	let cleanup: (() => void) | null = null;

	function escapeHtml(text: string): string {
		const div = document.createElement('div');
		div.textContent = text;
		return div.innerHTML;
	}

	function show() {
		if (!options.content) return;

		// Create tooltip container (uses existing chip-tooltip styling)
		tooltipEl = document.createElement('div');
		tooltipEl.className = options.variant ? `chip-tooltip ${options.variant}` : 'chip-tooltip';
		// Override hidden visibility since we're managing show/hide manually
		// Apply zoom from CSS variable to match content zoom level
		const zoom = getComputedStyle(document.documentElement).getPropertyValue('--content-zoom') || '1';
		tooltipEl.style.cssText = `position: fixed; z-index: 9999; pointer-events: none; opacity: 1; visibility: visible; zoom: ${zoom};`;

		// Create content
		const contentEl = document.createElement('div');
		contentEl.className = 'chip-tooltip-content';
		contentEl.innerHTML = options.html ? options.content : escapeHtml(options.content);
		tooltipEl.appendChild(contentEl);

		// Create arrow
		arrowEl = document.createElement('div');
		arrowEl.className = 'chip-tooltip-arrow';
		tooltipEl.appendChild(arrowEl);

		document.body.appendChild(tooltipEl);

		// Position with Floating UI
		updatePosition();
	}

	async function updatePosition() {
		if (!tooltipEl || !arrowEl) return;

		const { x, y, placement, middlewareData } = await computePosition(node, tooltipEl, {
			placement: options.placement ?? 'top',
			strategy: 'fixed',
			middleware: [
				offset(8),
				flip({ padding: { top: 50, bottom: 8, left: 8, right: 8 } }), // Account for header
				shift({ padding: 8 }),
				arrow({ element: arrowEl }),
			],
		});

		Object.assign(tooltipEl.style, {
			left: `${x}px`,
			top: `${y}px`,
		});

		// Position arrow
		if (middlewareData.arrow) {
			const { x: arrowX } = middlewareData.arrow;
			const staticSide = placement.includes('top') ? 'bottom' : 'top';

			Object.assign(arrowEl.style, {
				left: arrowX != null ? `${arrowX}px` : '',
				[staticSide]: '-4px',
			});
		}
	}

	function hide() {
		tooltipEl?.remove();
		tooltipEl = null;
		arrowEl = null;
	}

	node.addEventListener('mouseenter', show);
	node.addEventListener('mouseleave', hide);

	return {
		update(newOptions: TooltipOptions) {
			options = newOptions;
			if (tooltipEl) {
				hide();
				show();
			}
		},
		destroy() {
			hide();
			node.removeEventListener('mouseenter', show);
			node.removeEventListener('mouseleave', hide);
		},
	};
}
