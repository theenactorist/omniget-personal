<script lang="ts">
  import { t } from "$lib/i18n";
  import { downloadLocation } from "$lib/download-location";

  let { path, onReveal }: {
    path?: string | null;
    onReveal: (path: string) => void | Promise<void>;
  } = $props();

  let location = $derived(downloadLocation(path));
</script>

<div class="saved-location">
  {#if location}
    {@const savedPath = location.path}
    <div class="saved-location-details">
      <span class="saved-location-label">{$t("downloads.saved_to")}</span>
      <span class="saved-location-name" title={location.path} dir="auto">{location.name}</span>
      {#if location.parent}
        <span class="saved-location-parent" dir="auto">{location.parent}</span>
      {/if}
    </div>
    <button
      class="btn btn-secondary saved-location-reveal"
      type="button"
      title={location.path}
      aria-label={`${$t("downloads.open_folder")}: ${location.path}`}
      onclick={() => onReveal(savedPath)}
    >
      <svg viewBox="0 0 24 24" width="16" height="16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z" />
      </svg>
      {$t("downloads.open_folder")}
    </button>
  {:else}
    <span class="saved-location-label">{$t("downloads.saved_location_unavailable")}</span>
  {/if}
</div>

<style>
  .saved-location {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-3);
    width: 100%;
    min-width: 0;
    padding-block: var(--space-1);
  }

  .saved-location-details {
    display: flex;
    flex-direction: column;
    flex: 1 1 16rem;
    gap: 2px;
    min-width: 0;
  }

  .saved-location-label {
    font-size: var(--text-sm);
    line-height: var(--leading-sm);
    color: var(--text-muted);
  }

  .saved-location-name,
  .saved-location-parent {
    overflow-wrap: anywhere;
    white-space: pre-wrap;
    user-select: text;
    -webkit-user-select: text;
  }

  .saved-location-name {
    font-size: var(--text-sm);
    line-height: var(--leading-sm);
    font-weight: 500;
    color: var(--text);
  }

  .saved-location-parent {
    font-size: var(--text-xs);
    line-height: var(--leading-sm);
    color: var(--text-muted);
  }

  .saved-location-reveal {
    flex-shrink: 0;
    max-width: 100%;
    height: auto;
    min-height: var(--control-h);
    padding-block: var(--space-1);
    line-height: var(--leading-sm);
    white-space: normal;
  }
</style>
