import { redirect } from "@sveltejs/kit";
import { isUnavailableRoute } from "$lib/personal-build-policy";
import { loadTranslations, defaultLocale } from "$lib/i18n";
import { loadSettings } from "$lib/stores/settings-store.svelte";

export const ssr = false;

export const load = async ({ url }) => {
  if (isUnavailableRoute(url.pathname)) redirect(307, "/");
  let lang = defaultLocale;
  try {
    const settings = await loadSettings();
    lang = settings.appearance?.language || defaultLocale;
  } catch {
    // First run or settings unavailable — use defaultLocale
  }
  try {
    await loadTranslations(lang, url.pathname);
  } catch (e) {
    // O inglês estático em `$lib/i18n` segura a UI; só registra o motivo.
    console.warn(`[i18n] could not load "${lang}":`, e);
  }
  return {};
};
