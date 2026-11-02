<script lang="ts">
  import { t } from "$lib/i18n";

  const APP_VERSION = "0.9.6";

</script>

<section class="project-header">
  <img src="/loop.png" alt="Loop" class="project-logo" width="64" height="64" />
  <p class="project-description">{$t('about.description')}</p>
  <p class="project-description">{$t('about.upstream_attribution')}</p>
</section>

<section class="card">
  <h5 class="card-title">{$t('about.features_title')}</h5>
  <div class="features-list">
    <p class="feature-item">YouTube · Instagram · TikTok · Twitter/X · Reddit · Twitch · Pinterest · Vimeo · Bluesky</p>
    <p class="feature-item">{$t('about.feature_progress')}</p>
    <p class="feature-item">{$t('about.feature_mascot')}</p>
    <p class="feature-item">{$t('about.feature_themes')}</p>
    <p class="feature-item">{$t('about.feature_i18n')}</p>
    <p class="feature-item">{$t('about.feature_tech')}</p>
  </div>
</section>

<p class="version">{$t('about.version')} {APP_VERSION}</p>

<style>
  .project-header {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: calc(var(--padding) / 2);
    text-align: center;
  }

  .project-logo {
    border-radius: calc(var(--border-radius) + 4px);
    pointer-events: none;
  }

  .project-description {
    font-size: var(--text-base);
    font-weight: 400;
    line-height: 1.8;
    color: var(--gray);
    max-width: 320px;
  }

  .card {
    width: 100%;
    background: var(--button);
    box-shadow: var(--button-box-shadow);
    border-radius: var(--border-radius);
    padding: calc(var(--padding) + 4px);
    display: flex;
    flex-direction: column;
    gap: calc(var(--padding) / 2);
  }

  .card-title {
    color: var(--secondary);
  }

  .features-list {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .feature-item {
    font-size: 13px;
    font-weight: 400;
    line-height: 1.6;
    color: var(--gray);
  }

  .feature-item::before {
    content: "• ";
    color: var(--accent);
  }

  .version {
    font-size: var(--text-sm);
    font-weight: 500;
    color: var(--gray);
    text-align: center;
  }
</style>
