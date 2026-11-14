<script lang="ts">
  import '../app.css';
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { applyTheme, normalizeTheme } from '$lib/theme';
  import { watchSystemTheme } from '$lib/theme';

  onMount(() => {
    let media: MediaQueryList | null = null;
    let currentTheme = 'dark';
    let unwatchSystem = () => {};
    const applyCurrent = () => applyTheme(currentTheme, media);

    void (async () => {
      try {
        const settings = await invoke<{ theme?: string }>('get_default_settings');
        currentTheme = normalizeTheme(settings?.theme);
      } catch {
        currentTheme = 'dark';
      }
      media = window.matchMedia('(prefers-color-scheme: dark)');
      applyCurrent();
      unwatchSystem = watchSystemTheme(() => applyCurrent());
    })();

    const onThemeChanged = (e: Event) => {
      const detail = (e as CustomEvent<{ theme?: string }>).detail;
      currentTheme = normalizeTheme(detail?.theme ?? currentTheme);
      applyCurrent();
    };
    window.addEventListener('vibevisor-theme-changed', onThemeChanged as EventListener);

    return () => {
      unwatchSystem();
      window.removeEventListener('vibevisor-theme-changed', onThemeChanged as EventListener);
    };
  });
</script>

<slot />
