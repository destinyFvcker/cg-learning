<script lang="ts">
    import "./layout.css";
    import favicon from "$lib/assets/webgpu-notext.svg";
    import Nav from "$lib/components/Nav.svelte";

    let { children } = $props();
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>
<Nav />
{@render children()}
