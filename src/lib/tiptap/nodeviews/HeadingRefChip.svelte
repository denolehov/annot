<script lang="ts">
	interface Props {
		level: number;
		title: string;
		line: number;
	}

	let { level, title, line }: Props = $props();
</script>

<span class="heading-ref-chip" title="Section at line {line}">
	<span class="heading-level">H{level}</span>
	<span class="heading-title">{title}</span>
</span>

<style>
	.heading-ref-chip {
		display: inline-flex;
		align-items: baseline;
		gap: calc(4px * var(--content-zoom, 1));
	}

	.heading-level {
		font-size: var(--fs-10);
		font-weight: 600;
		opacity: 0.7;
		text-transform: uppercase;
	}

	.heading-title {
		font-size: var(--fs-12);
		font-weight: 500;
	}
</style>
