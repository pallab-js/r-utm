export type ThemePreference = 'dark' | 'light' | 'system';

export function normalizeTheme(value: string | null | undefined): ThemePreference {
  if (value === 'light' || value === 'system' || value === 'dark') return value;
  return 'dark';
}

export function resolveTheme(theme: string | null | undefined, media?: MediaQueryList | null): 'dark' | 'light' {
  const pref = normalizeTheme(theme);
  if (pref === 'system') {
    const m = media ?? (typeof window !== 'undefined' ? window.matchMedia('(prefers-color-scheme: dark)') : null);
    return m?.matches ? 'dark' : 'light';
  }
  return pref;
}

export function applyTheme(theme: string | null | undefined, media?: MediaQueryList | null): 'dark' | 'light' {
  if (typeof document === 'undefined') return 'dark';
  const resolved = resolveTheme(theme, media);
  const root = document.documentElement;
  root.dataset.theme = resolved;
  root.classList.toggle('dark', resolved === 'dark');
  root.style.colorScheme = resolved;
  return resolved;
}

export function watchSystemTheme(onChange: () => void): () => void {
  if (typeof window === 'undefined') return () => {};
  const media = window.matchMedia('(prefers-color-scheme: dark)');
  const handler = () => onChange();
  if (typeof media.addEventListener === 'function') {
    media.addEventListener('change', handler);
    return () => media.removeEventListener('change', handler);
  }
  media.addListener(handler);
  return () => media.removeListener(handler);
}
