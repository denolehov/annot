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
		gap: 4px;
	}

	.heading-level {
		font-size: 10px;
		font-weight: 600;
		opacity: 0.7;
		text-transform: uppercase;
	}

	.heading-title {
		font-size: 12px;
		font-weight: 500;
	}
</style>
