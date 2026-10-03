import logoIcon from "../../logo.ico";
import {
  isLanguage,
  languages,
  type Language,
  type MessageKey,
} from "../i18n";
import type { Dispatch, SetStateAction } from "react";

interface SystemInfo {
  distribution: string;
  family: string;
  architecture: string;
  package_managers: string[];
}

interface SettingsProps {
  onBack: () => void;
  systemInfo: SystemInfo | null;
  rememberLastSearch: boolean;
  setRememberLastSearch: (
    value: boolean
  ) => void;
  clearSavedSearch: () => void;
  language: Language;
  setLanguage: Dispatch<SetStateAction<Language>>;
  theme: "dark" | "light";
  setTheme: Dispatch<SetStateAction<"dark" | "light">>;
  t: (key: MessageKey) => string;
}

function Settings({
  onBack,
  systemInfo,
  rememberLastSearch,
  setRememberLastSearch,
  clearSavedSearch,
  language,
  setLanguage,
  theme,
  setTheme,
  t,
}: SettingsProps) {
  return (
    <div data-theme={theme} className="min-h-screen bg-[#09090b] text-zinc-100">
      <main className="min-h-screen overflow-auto">

        <header className="flex h-16 items-center justify-between border-b border-zinc-800/80 px-8">

          <button
            onClick={onBack}
            className="text-sm text-zinc-500 transition hover:text-white"
          >
            {t("back")}
          </button>

          <img
            src={logoIcon}
            alt="Logo de RepoFy"
            className="h-9 w-9 rounded-xl"
          />

        </header>

        <div className="mx-auto flex max-w-4xl flex-col items-center px-8 py-14">

            <div className="mb-10 w-full text-center">

              <div className="mb-4 flex justify-center">
                <img
                  src={logoIcon}
                  alt="Logo de RepoFy"
                  className="h-16 w-16 rounded-2xl"
                />
              </div>

              <h1 className="text-4xl font-bold tracking-tight text-white">
                {t("settingsTitle")}
              </h1>

              <p className="mt-3 text-base text-zinc-500">
                {t("settingsDescription")}
              </p>

            </div>

            {/* EXPERIENCIA */}
            <section className="mb-8 w-full">

              <h2 className="mb-4 text-xl font-semibold text-white">
                {t("experience")}
              </h2>

              <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6">
                <div className="flex flex-col gap-5 lg:flex-row lg:items-center lg:justify-between">

                  <div>

                    <h3 className="font-medium text-white">
                      {t("rememberSearch")}
                    </h3>

                    <p className="mt-1 text-sm text-zinc-500">
                      {t("rememberSearchDescription")}
                    </p>

                  </div>

                  <button
                    type="button"
                    onClick={() =>
                      setRememberLastSearch(
                        !rememberLastSearch
                      )
                    }
                    className={`inline-flex items-center gap-3 rounded-xl border px-4 py-3 text-sm font-medium transition ${
                      rememberLastSearch
                        ? "border-blue-500 bg-blue-500/10 text-blue-300"
                        : "border-zinc-800 bg-zinc-950 text-zinc-400 hover:border-zinc-700 hover:text-white"
                    }`}
                  >
                    <span
                      className={`h-2.5 w-2.5 rounded-full ${
                        rememberLastSearch
                          ? "bg-blue-400"
                          : "bg-zinc-600"
                      }`}
                    />

                    {rememberLastSearch
                      ? t("enabled")
                      : t("disabled")}
                  </button>

                </div>

                <div className="mt-5 rounded-xl border border-zinc-800 bg-zinc-950 p-4">
                  <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                    <div>
                      <h3 className="font-medium text-white">
                        {t("clearSavedSearch")}
                      </h3>

                      <p className="mt-1 text-sm text-zinc-500">
                        {t("clearSavedSearchDescription")}
                      </p>
                    </div>

                    <button
                      type="button"
                      onClick={clearSavedSearch}
                      className="rounded-lg border border-zinc-700 px-4 py-2.5 text-sm font-medium text-zinc-300 transition hover:border-zinc-600 hover:bg-zinc-900 hover:text-white"
                    >
                      {t("deleteSearch")}
                    </button>
                  </div>
                </div>
              </div>

            </section>

            {/* APARIENCIA */}
            <section className="mb-8 w-full">

              <h2 className="mb-4 text-xl font-semibold text-white">
                {t("appearance")}
              </h2>

              <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6">

                <h3 className="font-medium text-white">
                  {t("currentTheme")}
                </h3>

                <p className="mt-1 text-sm text-zinc-500">
                  {t("themeDescription")}
                </p>

                <div className="mt-5 grid gap-3 sm:grid-cols-2">
                  <button
                    type="button"
                    onClick={() => setTheme("dark")}
                    aria-pressed={theme === "dark"}
                    className={`rounded-xl border p-4 text-left transition ${
                      theme === "dark"
                        ? "border-blue-500 bg-blue-500/10"
                        : "border-zinc-800 bg-zinc-950 hover:border-zinc-700"
                    }`}
                  >
                    <div className="text-lg">
                      🌙
                    </div>

                    <div className="mt-2 font-medium text-white">
                      {t("dark")}
                    </div>

                    <div className="mt-1 text-xs text-zinc-400">
                      {theme === "dark" ? t("active") : ""}
                    </div>
                  </button>

                  <button
                    type="button"
                    onClick={() => setTheme("light")}
                    aria-pressed={theme === "light"}
                    className={`rounded-xl border p-4 text-left transition ${
                      theme === "light"
                        ? "border-blue-500 bg-blue-500/10"
                        : "border-zinc-800 bg-zinc-950 hover:border-zinc-700"
                    }`}
                  >
                    <div className="text-lg">
                      ☀️
                    </div>

                    <div className="mt-2 font-medium text-white">
                      {t("light")}
                    </div>

                    <div className="mt-1 text-xs text-zinc-500">
                      {theme === "light" ? t("active") : ""}
                    </div>
                  </button>
                </div>

              </div>

            </section>

            {/* IDIOMA */}
            <section className="mb-8 w-full">
              <h2 className="mb-4 text-xl font-semibold text-white">
                {t("language")}
              </h2>

              <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6">
                <label
                  htmlFor="repofy-language"
                  className="mb-2 block font-medium text-white"
                >
                  {t("language")}
                </label>
                <p className="mb-4 text-sm text-zinc-500">
                  {t("languageDescription")}
                </p>
                <select
                  id="repofy-language"
                  value={language}
                  onChange={(event) => {
                    if (isLanguage(event.target.value)) {
                      setLanguage(event.target.value);
                    }
                  }}
                  className="w-full max-w-sm rounded-lg border border-zinc-700 bg-zinc-950 px-3 py-2.5 text-sm text-zinc-100 outline-none focus:border-blue-500"
                >
                  {Object.entries(languages).map(([code, name]) => (
                    <option key={code} value={code}>
                      {name}
                    </option>
                  ))}
                </select>
              </div>
            </section>

            {/* SISTEMA */}
            <section className="mb-8 w-full">

              <h2 className="mb-4 text-xl font-semibold text-white">
                {t("system")}
              </h2>

              <div className="grid gap-3 md:grid-cols-2">
                <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">
                  <p className="text-xs font-medium uppercase tracking-wide text-zinc-600">
                    {t("distribution")}
                  </p>

                  <p className="mt-2 text-lg font-semibold text-white">
                    {systemInfo?.distribution ??
                      t("detecting")}
                  </p>

                  <p className="mt-1 text-sm text-zinc-500">
                    {t("family")}{" "}
                    {systemInfo?.family ??
                      "Linux"}
                  </p>
                </div>

                <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">
                  <p className="text-xs font-medium uppercase tracking-wide text-zinc-600">
                    {t("architecture")}
                  </p>

                  <p className="mt-2 text-lg font-semibold text-white">
                    {systemInfo?.architecture ??
                      t("detecting")}
                  </p>
                </div>
              </div>

              <div className="mt-3 rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">
                <h3 className="font-medium text-white">
                  {t("detectedManagers")}
                </h3>

                <p className="mt-1 text-sm text-zinc-500">
                  {t("managersDescription")}
                </p>

                <div className="mt-4 flex flex-wrap gap-2">
                  {systemInfo?.package_managers
                    .length ? (
                    systemInfo.package_managers.map(
                      (manager) => (
                        <div
                          key={manager}
                          className="flex items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2"
                        >
                          <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />

                          <span className="text-sm text-zinc-300">
                            {manager}
                          </span>
                        </div>
                      )
                    )
                  ) : (
                    <p className="text-sm text-zinc-500">
                      {t("detectingManagers")}
                    </p>
                  )}
                </div>
              </div>
            </section>

            {/* ACERCA DE */}
            <section className="w-full">

              <h2 className="mb-4 text-xl font-semibold text-white">
                {t("about")}
              </h2>

              <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6">

                <div className="flex items-center gap-4">

                  <img
                    src={logoIcon}
                    alt="Logo de RepoFy"
                    className="h-12 w-12 rounded-xl"
                  />

                  <div>

                    <h3 className="font-semibold text-white">
                      RepoFy
                    </h3>

                    <p className="text-sm text-zinc-500">
                      {t("softwareCenter")}
                    </p>

                  </div>

                </div>

                <div className="mt-6 border-t border-zinc-800 pt-5">

                  <p className="text-sm text-zinc-500">
                    {t("aboutDescription")}
                  </p>

                </div>

              </div>

            </section>

        </div>
      </main>
    </div>
  );
}

export default Settings;