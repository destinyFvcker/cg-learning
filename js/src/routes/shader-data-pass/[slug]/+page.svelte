<script lang="ts">
    import type { PageProps } from "./$types";
    import InterStage from "./inter-stage.svelte";

    let { data }: PageProps = $props();
</script>

{#if data.slug == "inter-stage"}
    <InterStage />
{/if}
