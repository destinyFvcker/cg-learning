<script lang="ts">
    import { afterNavigate } from "$app/navigation";
    import { page } from "$app/state";
    import * as Popover from "$lib/components/ui/popover/index.js";
    import CompassIcon from "@lucide/svelte/icons/compass";
    import ChevronUpIcon from "@lucide/svelte/icons/chevron-up";

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

    let open = $state(false);
    const pathname = $derived(page.url.pathname);

    function closeNavigation() {
        open = false;
    }

    afterNavigate(closeNavigation);

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

<Popover.Root bind:open>
    <div
        class={[
            "fixed right-[max(1rem,env(safe-area-inset-right))] bottom-[max(1rem,env(safe-area-inset-bottom))] z-50 sm:right-[max(1.5rem,env(safe-area-inset-right))] sm:bottom-[max(1.5rem,env(safe-area-inset-bottom))]",
            className,
        ]}
    >
        <Popover.Trigger
            class="inline-flex h-11 cursor-pointer items-center gap-2 rounded-full border border-border bg-card px-4 text-sm font-medium text-foreground shadow-lg shadow-foreground/10 transition-colors hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 focus-visible:ring-offset-background data-open:bg-muted"
            aria-label={open ? "Close navigation" : "Open navigation"}
        >
            <CompassIcon class="size-4" aria-hidden="true" />
            <span>Navigation</span>
            <ChevronUpIcon
                class={["size-4 transition-transform duration-150 motion-reduce:transition-none", open && "rotate-180"]}
                aria-hidden="true"
            />
        </Popover.Trigger>
    </div>

    <Popover.Content
        side="top"
        align="end"
        sideOffset={12}
        collisionPadding={16}
        trapFocus={false}
        aria-label="Learning navigation"
        class="max-h-[min(calc(100dvh-6rem),var(--bits-popover-content-available-height))] w-[min(calc(100vw-2rem),22rem)] overflow-y-auto overscroll-contain rounded-[1rem] border border-border bg-card bg-[radial-gradient(circle_at_100%_0%,color-mix(in_oklab,var(--primary)_9%,transparent),transparent_40%)] p-0 text-base leading-[1.5] shadow-[0_1rem_2.5rem_color-mix(in_oklab,var(--foreground)_12%,transparent)] ring-0 motion-reduce:animate-none"
    >
        <nav
            class="grid grid-cols-1 gap-4 px-[1.1rem] py-[0.85rem] sm:px-6 sm:py-4"
            aria-label="Main navigation"
        >
            <div class="flex flex-col items-start justify-center">
                <p
                    class="m-0 mb-[0.35rem] text-[0.7rem] font-bold tracking-[0.16em] text-muted-foreground uppercase"
                >
                    {eyebrow}
                </p>
                <a
                    class="text-[clamp(1.35rem,2vw,1.75rem)] leading-[1.1] font-[750] tracking-[-0.04em] text-foreground no-underline"
                    onclick={closeNavigation}
                    href="/">{brand}</a
                >
                <p class="m-0 mt-2 max-w-80 text-sm leading-[1.5] text-muted-foreground">
                    {description}
                </p>
            </div>

            {#snippet renderItems(navItems: NavItem[], depth = 0)}
                <ul
                    class={[
                        "list-none",
                        depth === 0
                            ? "m-0 grid content-center gap-1 p-0"
                            : "mt-[0.2rem] mr-0 mb-[0.35rem] ml-[1.1rem] border-l border-border py-[0.15rem] pr-0 pl-4",
                    ]}
                    aria-label={depth === 0 ? "Learning sections" : undefined}
                >
                    {#each navItems as item, index (item.href)}
                        {@const current = isActive(item)}
                        {@const branch = current || hasActiveChild(item)}
                        <li>
                            <a
                                class={[
                                    "relative grid items-center gap-[0.65rem] rounded-[0.65rem] border px-[0.7rem] py-[0.35rem] font-[550] no-underline transition-[color,background-color,border-color,translate] duration-150 ease-[ease] hover:translate-x-0.5 hover:outline-none focus-visible:translate-x-0.5 focus-visible:outline-none",
                                    depth === 0
                                        ? "min-h-[2.4rem] grid-cols-[2rem_1fr_auto] text-[0.9rem]"
                                        : "min-h-[2.1rem] grid-cols-[1.25rem_1fr_auto] text-[0.82rem]",
                                    current
                                        ? "border-[color-mix(in_oklab,var(--primary)_30%,var(--border))] bg-primary text-primary-foreground shadow-[0_0.4rem_1rem_color-mix(in_oklab,var(--primary)_18%,transparent)]"
                                        : [
                                              "border-transparent hover:border-border hover:bg-muted hover:text-foreground focus-visible:border-border focus-visible:bg-muted focus-visible:text-foreground",
                                              branch ? "text-foreground" : "text-muted-foreground",
                                          ],
                                ]}
                                onclick={closeNavigation}
                                href={item.href}
                                aria-current={current ? "page" : undefined}
                                target={item.external ? "_blank" : undefined}
                                rel={item.external ? "noreferrer" : undefined}
                            >
                                <span
                                    class={[
                                        "font-[family-name:ui-monospace,SFMono-Regular,Menlo,monospace] text-[0.72rem] tabular-nums",
                                        current ? "text-primary-foreground/72" : "text-muted-foreground/70",
                                    ]}
                                    >{depth === 0
                                        ? String(index + 1).padStart(2, "0")
                                        : "↳"}</span
                                >
                                <span class="truncate">{item.label}</span>
                                {#if item.children?.length}
                                    <span
                                        class={[
                                            "inline-grid h-[1.3rem] min-w-[1.3rem] place-items-center rounded-[999px] px-1 py-0 text-[0.68rem]",
                                            current ? "bg-primary-foreground text-primary" : "bg-muted text-muted-foreground",
                                        ]}
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
        </nav>
    </Popover.Content>
</Popover.Root>
