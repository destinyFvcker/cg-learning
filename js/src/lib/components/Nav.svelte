<script lang="ts">
    import { page } from "$app/state";

    export type NavItem = {
        href: string;
        label: string;
        exact?: boolean;
        external?: boolean;
        children?: NavItem[];
    };

    export const defaultNavItems: NavItem[] = [
        { href: "/", label: "Overview", exact: true },
        { href: "/basics", label: "Basics" },
        {
            href: "/shader-data-pass",
            label: "Shader Data Pass",
            children: [
                { href: "/shader-data-pass/inter-stage", label: "Inter-stage" },
            ],
        },
    ];

    export type NavProps = {
        items?: NavItem[];
        brand?: string;
        eyebrow?: string;
        description?: string;
        class?: string;
    };

    let {
        items = defaultNavItems,
        brand = "WebGPU Learning",
        eyebrow = "Learning path",
        description = "Start with the fundamentals, then follow the data through the shader pipeline.",
        class: className = "",
    }: NavProps = $props();

    const pathname = $derived(page.url.pathname);

    function isActive(item: NavItem) {
        if (item.external) return false;
        if (item.exact || item.href === "/") return pathname === item.href;
        return pathname === item.href || pathname.startsWith(`${item.href}/`);
    }

    function hasActiveChild(item: NavItem): boolean {
        return (
            item.children?.some(
                (child) => isActive(child) || hasActiveChild(child),
            ) ?? false
        );
    }
</script>

<nav class={`nav-shell ${className}`} aria-label="Main navigation">
    <div class="nav-inner">
        <div class="nav-intro">
            <p class="nav-eyebrow">{eyebrow}</p>
            <a class="nav-brand" href="/">{brand}</a>
            <p class="nav-description">{description}</p>
        </div>

        {#snippet renderItems(navItems: NavItem[], depth = 0)}
            <ul
                class:nav-list={depth === 0}
                class:nav-children={depth > 0}
                aria-label={depth === 0 ? "Learning sections" : undefined}
            >
                {#each navItems as item, index (item.href)}
                    {@const current = isActive(item)}
                    {@const branch = current || hasActiveChild(item)}
                    <li class:branch>
                        <a
                            class:active={current}
                            class="nav-link"
                            href={item.href}
                            aria-current={current ? "page" : undefined}
                            target={item.external ? "_blank" : undefined}
                            rel={item.external ? "noreferrer" : undefined}
                        >
                            <span class="nav-index"
                                >{depth === 0
                                    ? String(index + 1).padStart(2, "0")
                                    : "↳"}</span
                            >
                            <span class="nav-label">{item.label}</span>
                            {#if item.children?.length}
                                <span class="nav-count"
                                    >{item.children.length}</span
                                >
                            {/if}
                        </a>

                        {#if item.children?.length}
                            {@render renderItems(item.children, depth + 1)}
                        {/if}
                    </li>
                {/each}
            </ul>
        {/snippet}

        {@render renderItems(items)}
    </div>
</nav>

<style>
    .nav-shell {
        position: fixed;
        top: 1.5rem;
        right: 1.5rem;
        z-index: 50;
        width: min(calc(100% - 3rem), 22rem);
        max-height: calc(100dvh - 3rem);
        overflow-y: auto;
        border-radius: 1rem;
        box-shadow: 0 1rem 2.5rem
            color-mix(in oklab, var(--foreground) 8%, transparent);
    }

    .nav-inner {
        display: grid;
        grid-template-columns: minmax(0, 1fr);
		gap: 1rem;
		padding: 1rem 1.5rem;
        border: 1px solid var(--border);
        border-radius: 1rem;
        background:
            radial-gradient(
                circle at 100% 0%,
                color-mix(in oklab, var(--primary) 9%, transparent),
                transparent 40%
            ),
            var(--card);
    }

    .nav-intro {
        display: flex;
        align-items: flex-start;
        flex-direction: column;
        justify-content: center;
    }

    .nav-eyebrow {
		margin: 0 0 0.35rem;
        color: var(--muted-foreground);
        font-size: 0.7rem;
        font-weight: 700;
        letter-spacing: 0.16em;
        text-transform: uppercase;
    }

    .nav-brand {
        color: var(--foreground);
        font-size: clamp(1.35rem, 2vw, 1.75rem);
        font-weight: 750;
        letter-spacing: -0.04em;
        line-height: 1.1;
        text-decoration: none;
    }

    .nav-description {
        max-width: 20rem;
		margin: 0.5rem 0 0;
        color: var(--muted-foreground);
        font-size: 0.875rem;
		line-height: 1.5;
    }

    .nav-list,
    .nav-children {
        margin: 0;
        padding: 0;
        list-style: none;
    }

    .nav-list {
        display: grid;
        align-content: center;
		gap: 0.25rem;
    }

    .nav-children {
        margin: 0.2rem 0 0.35rem 1.1rem;
        padding: 0.15rem 0 0.15rem 1rem;
        border-left: 1px solid var(--border);
    }

    .nav-link {
        position: relative;
        display: grid;
        grid-template-columns: 2rem 1fr auto;
        align-items: center;
        gap: 0.65rem;
		min-height: 2.4rem;
		padding: 0.35rem 0.7rem;
        border: 1px solid transparent;
        border-radius: 0.65rem;
        color: var(--muted-foreground);
        font-size: 0.9rem;
        font-weight: 550;
        text-decoration: none;
        transition:
            color 150ms ease,
            background-color 150ms ease,
            border-color 150ms ease,
            transform 150ms ease;
    }

    .nav-link:hover,
    .nav-link:focus-visible {
        border-color: var(--border);
        color: var(--foreground);
        background: var(--muted);
        outline: none;
        transform: translateX(2px);
    }

    .nav-link.active {
        border-color: color-mix(in oklab, var(--primary) 30%, var(--border));
        color: var(--primary-foreground);
        background: var(--primary);
        box-shadow: 0 0.4rem 1rem
            color-mix(in oklab, var(--primary) 18%, transparent);
    }

    .branch > .nav-link:not(.active) {
        color: var(--foreground);
    }

    .nav-index {
        color: color-mix(in oklab, var(--muted-foreground) 70%, transparent);
        font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
        font-size: 0.72rem;
        font-variant-numeric: tabular-nums;
    }

    .nav-link.active .nav-index {
        color: color-mix(in oklab, var(--primary-foreground) 72%, transparent);
    }

    .nav-label {
        overflow: hidden;
        text-overflow: ellipsis;
        white-space: nowrap;
    }

    .nav-count {
        display: inline-grid;
        place-items: center;
        min-width: 1.3rem;
        height: 1.3rem;
        padding: 0 0.25rem;
        border-radius: 999px;
        color: var(--muted-foreground);
        background: var(--muted);
        font-size: 0.68rem;
    }

    .nav-link.active .nav-count {
        color: var(--primary);
        background: var(--primary-foreground);
    }

    .nav-children .nav-link {
        grid-template-columns: 1.25rem 1fr auto;
		min-height: 2.1rem;
        font-size: 0.82rem;
    }

    @media (max-width: 42rem) {
        .nav-shell {
            top: 0.5rem;
            right: 0.5rem;
            width: min(calc(100% - 1rem), 22rem);
            max-height: calc(100dvh - 1rem);
        }

        .nav-inner {
            grid-template-columns: 1fr;
            gap: 1rem;
			padding: 0.85rem 1.1rem;
        }
    }
</style>
