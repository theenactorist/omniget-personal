<script lang="ts">
  import { page } from "$app/state";
  import { t } from "$lib/i18n";
  import NavIcon from "$components/shell/NavIcon.svelte";
  import type { NavItem } from "$lib/nav-config";

  interface Props {
    primaryNav?: NavItem[];
    appNav?: NavItem[];
    badgeLabel?: string;
    badgeCount?: number;
  }

  let {
    primaryNav = [],
    appNav = [],
    badgeLabel = "",
    badgeCount = 0,
  }: Props = $props();

  function isActive(href: string): boolean {
    if (href === "/") return page.url.pathname === "/";
    return page.url.pathname.startsWith(href);
  }

  function itemTitle(item: NavItem): string {
    return item.label || (item.labelKey ? $t(item.labelKey) : "");
  }
</script>

{#snippet navLink(item: NavItem)}
  {@const title = itemTitle(item)}
  {@const active = isActive(item.href)}
  <a
    href={item.href}
    class="mac-nav-item"
    class:active
    title={title}
    aria-current={active ? "page" : undefined}
  >
    <NavIcon icon={item.icon} iconSvg={item.iconSvg} size={18} {active} />
    <span class="mac-nav-label">{title}</span>
    {#if item.badge === "downloads" && badgeCount > 0}
      <span class="mac-nav-badge live">{badgeLabel}</span>
    {/if}
  </a>
{/snippet}

<aside class="mac-source-list" aria-label={$t("nav.section_primary")}>
  <div class="mac-source-list-drag" data-tauri-drag-region></div>

  <nav class="mac-nav-section" aria-label={$t("nav.section_primary")}>
    {#each primaryNav as item (item.href)}
      {@render navLink(item)}
    {/each}
  </nav>

  <nav class="mac-nav-section" aria-label={$t("nav.section_app")}>
    <div class="mac-nav-section-header">{$t("nav.section_app")}</div>
    {#each appNav as item (item.href)}
      {@render navLink(item)}
    {/each}
  </nav>

</aside>
