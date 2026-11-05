import { derived, writable } from "svelte/store";

export interface LocaleMeta {
  name: string;
  nativeName: string;
  dir?: "ltr" | "rtl";
}

export interface LocaleFile {
  _meta: LocaleMeta;
  [key: string]: unknown;
}

const modules = import.meta.glob("../locales/*.json", {
  eager: true,
}) as Record<string, LocaleFile | { default: LocaleFile }>;

const catalogs: Record<string, LocaleFile> = {};

for (const [path, module] of Object.entries(modules)) {
  const code = path.split("/").pop()!.replace(/\.json$/, "");
  const data =
    (module as { default?: LocaleFile }).default ?? (module as LocaleFile);
  if (data?._meta) {
    catalogs[code] = data;
  }
}

const fallbackCode = "en";

function initialLocale(): string {
  try {
    if (typeof localStorage === "undefined") return fallbackCode;
    const raw = localStorage.getItem("gambiit.settings.v1");
    if (raw) {
      const value = JSON.parse(raw) as { language?: unknown };
      if (
        typeof value.language === "string" &&
        catalogs[value.language]
      ) {
        return value.language;
      }
    }
  } catch {
    // Ignore malformed local storage and fall back to English.
  }
  return fallbackCode;
}

export const locale = writable<string>(initialLocale());

export const availableLocales = Object.entries(catalogs)
  .map(([code, catalog]) => ({ code, ...catalog._meta }))
  .sort((left, right) => left.nativeName.localeCompare(right.nativeName));

export function localeMeta(code: string): LocaleMeta {
  return (
    catalogs[code]?._meta ??
    catalogs[fallbackCode]?._meta ?? {
      name: fallbackCode,
      nativeName: fallbackCode,
    }
  );
}

export function setLocale(code: string): void {
  if (catalogs[code]) {
    locale.set(code);
  }
}

function lookup(catalog: LocaleFile | undefined, key: string): unknown {
  if (!catalog) return undefined;
  return key.split(".").reduce<unknown>((value, part) => {
    if (value && typeof value === "object") {
      return (value as Record<string, unknown>)[part];
    }
    return undefined;
  }, catalog);
}

export function translate(
  code: string,
  key: string,
  params?: Record<string, string | number>,
): string {
  const primary = lookup(catalogs[code], key);
  const fallback = lookup(catalogs[fallbackCode], key);
  let text =
    typeof primary === "string"
      ? primary
      : typeof fallback === "string"
        ? fallback
        : key;

  if (params) {
    for (const [name, value] of Object.entries(params)) {
      text = text.replaceAll(`{${name}}`, String(value));
    }
  }
  return text;
}

export const i18n = derived(
  locale,
  ($locale) =>
    (key: string, params?: Record<string, string | number>) =>
      translate($locale, key, params),
);
