import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import Settings from "./components/Settings";
import logoIcon from "../logo.ico";

interface SystemInfo {
  distribution: string;
  family: string;
  architecture: string;
  package_managers: string[];
}

interface Package {
  name: string;
  version: string;
  description: string;
  repository: string;
  manager: string;
}

type InstallationStatus = "checking" | "installed" | "available";
const STORAGE_KEYS = {
  lastSearch: "repofy.last-search",
  rememberLastSearch:
    "repofy.remember-last-search",
} as const;

function App() {
  const [showSettings, setShowSettings] = useState(false);
  const [rememberLastSearch, setRememberLastSearch] =
    useState(() => {
      if (typeof window === "undefined") {
        return true;
      }

      const savedPreference =
        window.localStorage.getItem(
          STORAGE_KEYS.rememberLastSearch
        );

      return savedPreference !== "false";
    });
  const [systemInfo, setSystemInfo] =
    useState<SystemInfo | null>(null);

  const [searchQuery, setSearchQuery] = useState(
    () => {
      if (typeof window === "undefined") {
        return "";
      }

      const shouldRemember =
        window.localStorage.getItem(
          STORAGE_KEYS.rememberLastSearch
        ) !== "false";

      if (!shouldRemember) {
        return "";
      }

      return (
        window.localStorage.getItem(
          STORAGE_KEYS.lastSearch
        ) ?? ""
      );
    }
  );
  const [searching, setSearching] = useState(false);
  const [searchError, setSearchError] = useState("");
  const [packages, setPackages] = useState<Package[]>([]);

  /*
   * ESTADO DE INSTALACIÓN DE LOS RESULTADOS
   *
   * Ejemplo:
   *
   * {
   *   steam: "installed",
   *   neofetch: "available",
   *   firefox: "installed"
   * }
   */
  const [installationStatuses, setInstallationStatuses] =
    useState<Record<string, InstallationStatus>>({});

  /*
   * PAQUETE SELECCIONADO
   */
  const [selectedPackage, setSelectedPackage] =
    useState<Package | null>(null);

  const [packageInstalled, setPackageInstalled] =
    useState(false);

  const [checkingInstalled, setCheckingInstalled] =
    useState(false);

  const [installing, setInstalling] =
    useState(false);

  const [removing, setRemoving] =
    useState(false);

  const [packageActionError, setPackageActionError] =
    useState("");

  const [packageActionMessage, setPackageActionMessage] =
    useState("");

  /*
   * INFORMACIÓN DEL SISTEMA
   */
  useEffect(() => {
    async function loadSystemInfo() {
      try {
        const info = await invoke<SystemInfo>(
          "get_system_info"
        );

        setSystemInfo(info);
      } catch (error) {
        console.error(
          "Error obteniendo información del sistema:",
          error
        );
      }
    }

    loadSystemInfo();
  }, []);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    window.localStorage.setItem(
      STORAGE_KEYS.rememberLastSearch,
      String(rememberLastSearch)
    );

    if (!rememberLastSearch) {
      window.localStorage.removeItem(
        STORAGE_KEYS.lastSearch
      );
    }
  }, [rememberLastSearch]);

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    if (rememberLastSearch) {
      window.localStorage.setItem(
        STORAGE_KEYS.lastSearch,
        searchQuery
      );
      return;
    }

    window.localStorage.removeItem(
      STORAGE_KEYS.lastSearch
    );
  }, [rememberLastSearch, searchQuery]);

  /*
   * COMPROBAR SI UN PAQUETE ESTÁ INSTALADO
   */
  async function checkPackageInstalled(
    packageName: string
  ): Promise<boolean> {
    try {
      const installed = await invoke<boolean>(
        "is_package_installed",
        {
          packageName,
        }
      );

      return installed;
    } catch (error) {
      console.error(
        `Error comprobando ${packageName}:`,
        error
      );

      return false;
    }
  }

  /*
   * COMPROBAR ESTADO DE TODOS LOS RESULTADOS
   *
   * Las comprobaciones se realizan en paralelo para
   * que una búsqueda con muchos resultados no sea lenta.
   */
  async function checkPackagesInstallationStatus(
    packageList: Package[]
  ) {
    if (packageList.length === 0) {
      setInstallationStatuses({});
      return;
    }

    /*
     * Primero mostramos "Comprobando..." para todos.
     */
    const initialStatuses: Record<
      string,
      InstallationStatus
    > = {};

    packageList.forEach((pkg) => {
      initialStatuses[pkg.name] = "checking";
    });

    setInstallationStatuses(initialStatuses);

    /*
     * Comprobamos todos los paquetes simultáneamente.
     */
    const results = await Promise.all(
      packageList.map(async (pkg) => {
        const installed =
          await checkPackageInstalled(pkg.name);

        return {
          name: pkg.name,
          installed,
        };
      })
    );

    /*
     * Construimos el estado final.
     */
    const finalStatuses: Record<
      string,
      InstallationStatus
    > = {};

    results.forEach((result) => {
      finalStatuses[result.name] =
        result.installed
          ? "installed"
          : "available";
    });

    setInstallationStatuses(finalStatuses);
  }

  /*
   * BUSCAR PAQUETES
   */
  async function searchPackages(
    queryOverride?: string
  ) {
    const query = (
      queryOverride ?? searchQuery
    ).trim();

    if (!query) {
      setPackages([]);
      setSearchError("");
      setInstallationStatuses({});
      return;
    }

    setSearching(true);
    setSearchError("");

    /*
     * Limpiamos los estados de la búsqueda anterior.
     */
    setInstallationStatuses({});

    try {
      const results = await invoke<Package[]>(
        "search_pacman",
        {
          query,
        }
      );

      const normalizedQuery =
        query.toLowerCase();

      /*
       * Ordenamos por relevancia.
       *
       * 0 = coincidencia exacta
       * 1 = empieza por la búsqueda
       * 2 = contiene la búsqueda
       * 3 = resto
       */
      const sortedResults = [...results].sort(
        (a, b) => {
          const aName =
            a.name.toLowerCase();

          const bName =
            b.name.toLowerCase();

          const getScore = (name: string) => {
            if (name === normalizedQuery) {
              return 0;
            }

            if (
              name.startsWith(normalizedQuery)
            ) {
              return 1;
            }

            if (
              name.includes(normalizedQuery)
            ) {
              return 2;
            }

            return 3;
          };

          return (
            getScore(aName) -
            getScore(bName)
          );
        }
      );

      /*
       * Eliminamos posibles duplicados.
       *
       * Esto evita que el mismo paquete aparezca
       * varias veces si pacman devuelve entradas
       * repetidas.
       */
      const uniqueResults: Package[] = [];

      const seenPackages =
        new Set<string>();

      sortedResults.forEach((pkg) => {
        const key =
          `${pkg.manager}:${pkg.name}`;

        if (!seenPackages.has(key)) {
          seenPackages.add(key);
          uniqueResults.push(pkg);
        }
      });

      setPackages(uniqueResults);

      /*
       * Ya tenemos los resultados.
       *
       * Dejamos de mostrar el spinner de búsqueda
       * y comenzamos a comprobar instalaciones.
       */
      setSearching(false);

      await checkPackagesInstallationStatus(
        uniqueResults
      );
    } catch (error) {
      console.error(
        "Error buscando paquetes:",
        error
      );

      setPackages([]);

      setInstallationStatuses({});

      setSearchError(
        typeof error === "string"
          ? error
          : "No se pudieron buscar los paquetes."
      );

      setSearching(false);
    }
  }

  /*
   * ABRIR DETALLES
   */
  async function openPackageDetails(
    pkg: Package
  ) {
    setSelectedPackage(pkg);

    setPackageInstalled(false);
    setCheckingInstalled(true);

    setPackageActionError("");
    setPackageActionMessage("");

    const installed =
      await checkPackageInstalled(
        pkg.name
      );

    setPackageInstalled(installed);
    setCheckingInstalled(false);

    /*
     * Actualizamos también el estado de la
     * tarjeta de resultados.
     */
    setInstallationStatuses(
      (previous) => ({
        ...previous,
        [pkg.name]: installed
          ? "installed"
          : "available",
      })
    );
  }

  /*
   * INSTALAR
   */
  async function installSelectedPackage() {
    if (
      !selectedPackage ||
      installing ||
      removing
    ) {
      return;
    }

    setInstalling(true);

    setPackageActionError("");
    setPackageActionMessage("");

    try {
      const message =
        await invoke<string>(
          "install_package",
          {
            packageName:
              selectedPackage.name,
          }
        );

      setPackageActionMessage(message);

      /*
       * Comprobamos el estado real después
       * de la instalación.
       */
      const installed =
        await checkPackageInstalled(
          selectedPackage.name
        );

      setPackageInstalled(installed);

      /*
       * Actualizamos la tarjeta de resultados.
       */
      setInstallationStatuses(
        (previous) => ({
          ...previous,
          [selectedPackage.name]:
            installed
              ? "installed"
              : "available",
        })
      );
    } catch (error) {
      console.error(
        "Error instalando paquete:",
        error
      );

      setPackageActionError(
        typeof error === "string"
          ? error
          : "No se pudo instalar el paquete."
      );
    } finally {
      setInstalling(false);
    }
  }

  /*
   * DESINSTALAR
   */
  async function removeSelectedPackage() {
    if (
      !selectedPackage ||
      installing ||
      removing
    ) {
      return;
    }

    setRemoving(true);

    setPackageActionError("");
    setPackageActionMessage("");

    try {
      const message =
        await invoke<string>(
          "remove_package",
          {
            packageName:
              selectedPackage.name,
          }
        );

      setPackageActionMessage(message);

      /*
       * Comprobamos el estado real después
       * de desinstalar.
       */
      const installed =
        await checkPackageInstalled(
          selectedPackage.name
        );

      setPackageInstalled(installed);

      /*
       * Actualizamos también los resultados.
       */
      setInstallationStatuses(
        (previous) => ({
          ...previous,
          [selectedPackage.name]:
            installed
              ? "installed"
              : "available",
        })
      );
    } catch (error) {
      console.error(
        "Error desinstalando paquete:",
        error
      );

      setPackageActionError(
        typeof error === "string"
          ? error
          : "No se pudo desinstalar el paquete."
      );
    } finally {
      setRemoving(false);
    }
  }

  /*
   * VOLVER A RESULTADOS
   */
  function goBackToResults() {
    setSelectedPackage(null);

    setPackageActionError("");
    setPackageActionMessage("");
  }

  function clearSavedSearch() {
    setSearchQuery("");
    setPackages([]);
    setSearchError("");
    setInstallationStatuses({});

    if (typeof window !== "undefined") {
      window.localStorage.removeItem(
        STORAGE_KEYS.lastSearch
      );
    }
  }

  function openHome() {
    setShowSettings(false);
    setSelectedPackage(null);
  }

  function openSettings() {
    setSelectedPackage(null);
    setShowSettings(true);
  }

  /*
   * OBTENER ESTADO DE UN PAQUETE
   */
  function getInstallationStatus(
    packageName: string
  ): InstallationStatus | undefined {
    return installationStatuses[
      packageName
    ];
  }

  if (showSettings) {
    return (
      <Settings
        onBack={openHome}
        systemInfo={systemInfo}
        rememberLastSearch={
          rememberLastSearch
        }
        setRememberLastSearch={
          setRememberLastSearch
        }
        clearSavedSearch={clearSavedSearch}
      />
    );
  }

  /*
   * VISTA DE DETALLES
   */
  if (selectedPackage) {
    return (
      <div className="min-h-screen bg-[#09090b] text-zinc-100">
        <main className="min-h-screen overflow-auto">

          <header className="flex h-16 items-center justify-between border-b border-zinc-800/80 px-8">

            <button
              onClick={goBackToResults}
              className="text-sm text-zinc-500 transition hover:text-white"
            >
              ← Volver a resultados
            </button>

            <button
              onClick={openSettings}
              className="flex h-9 w-9 items-center justify-center rounded-lg text-zinc-500 transition hover:bg-zinc-800 hover:text-white"
              title="Ajustes"
            >
              ⚙
            </button>

          </header>

          <div className="mx-auto flex max-w-5xl flex-col items-center px-8 py-14">

              {/* HEADER */}
              <section className="mb-10 w-full text-center">

                <div className="flex items-start gap-5">

                  <div className="flex h-20 w-20 shrink-0 items-center justify-center rounded-2xl border border-zinc-800 bg-zinc-900 text-3xl font-semibold text-zinc-400">
                    {selectedPackage.name
                      .charAt(0)
                      .toUpperCase()}
                  </div>

                  <div className="min-w-0 text-left">

                    <div className="flex flex-wrap items-center gap-3">

                      <h1 className="text-4xl font-bold tracking-tight text-white">
                        {selectedPackage.name}
                      </h1>

                      {checkingInstalled && (
                        <span className="flex items-center gap-2 rounded-full bg-zinc-800 px-3 py-1 text-xs font-semibold text-zinc-400">

                          <span className="h-2.5 w-2.5 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-500" />

                          Comprobando...

                        </span>
                      )}

                      {!checkingInstalled &&
                        packageInstalled && (
                          <span className="rounded-full bg-emerald-500/10 px-3 py-1 text-xs font-semibold text-emerald-400">
                            ✓ Instalado
                          </span>
                        )}

                      {!checkingInstalled &&
                        !packageInstalled && (
                          <span className="rounded-full bg-zinc-800 px-3 py-1 text-xs font-semibold text-zinc-500">
                            Disponible
                          </span>
                        )}

                      <span className="rounded-full bg-blue-500/10 px-3 py-1 text-xs font-semibold text-blue-400">
                        {selectedPackage.repository}
                      </span>

                    </div>

                    <p className="mt-3 max-w-2xl text-base leading-7 text-zinc-500">
                      {selectedPackage.description ||
                        "Sin descripción disponible."}
                    </p>

                  </div>

                </div>

              </section>

              {/* INFORMACIÓN */}
              <section className="mb-8 w-full">

                <h2 className="mb-4 text-xl font-semibold text-white">
                  Información
                </h2>

                <div className="grid gap-3 md:grid-cols-2">

                  <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">

                    <p className="text-xs font-medium uppercase tracking-wide text-zinc-600">
                      Versión
                    </p>

                    <p className="mt-2 text-lg font-semibold text-white">
                      {selectedPackage.version}
                    </p>

                  </div>

                  <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">

                    <p className="text-xs font-medium uppercase tracking-wide text-zinc-600">
                      Repositorio
                    </p>

                    <p className="mt-2 text-lg font-semibold text-white">
                      {selectedPackage.repository}
                    </p>

                  </div>

                  <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">

                    <p className="text-xs font-medium uppercase tracking-wide text-zinc-600">
                      Gestor de paquetes
                    </p>

                    <p className="mt-2 text-lg font-semibold text-white">
                      {selectedPackage.manager}
                    </p>

                  </div>

                  <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">

                    <p className="text-xs font-medium uppercase tracking-wide text-zinc-600">
                      Arquitectura
                    </p>

                    <p className="mt-2 text-lg font-semibold text-white">
                      {systemInfo?.architecture ??
                        "Desconocida"}
                    </p>

                  </div>

                </div>

              </section>

              {/* INSTALACIÓN */}
              <section className="w-full">

                <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6">

                  <h2 className="text-xl font-semibold text-white">

                    {checkingInstalled
                      ? "Comprobando paquete"
                      : packageInstalled
                      ? "Paquete instalado"
                      : "Instalación"}

                  </h2>

                  <p className="mt-2 text-sm leading-6 text-zinc-500">

                    {checkingInstalled
                      ? "RepoFy está comprobando el estado real del paquete."
                      : packageInstalled
                      ? "Este paquete ya está instalado en tu sistema."
                      : `RepoFy ha detectado que este paquete está disponible mediante ${selectedPackage.manager}.`}

                  </p>

                  <div className="mt-5 rounded-xl border border-zinc-800 bg-zinc-950 p-4">

                    <code className="text-sm text-zinc-300">

                      {packageInstalled
                        ? `pacman -Q ${selectedPackage.name}`
                        : `sudo pacman -S ${selectedPackage.name}`}

                    </code>

                  </div>

                  {checkingInstalled && (
                    <div className="mt-5 flex items-center gap-3 text-sm text-zinc-500">

                      <div className="h-4 w-4 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-500" />

                      Comprobando estado...

                    </div>
                  )}

                  {packageActionError && (
                    <div className="mt-5 rounded-xl border border-red-900/50 bg-red-950/20 p-4">

                      <p className="text-sm font-medium text-red-300">
                        No se pudo realizar la operación
                      </p>

                      <p className="mt-1 text-xs leading-5 text-red-400/80">
                        {packageActionError}
                      </p>

                    </div>
                  )}

                  {packageActionMessage &&
                    !packageActionError && (
                      <div className="mt-5 rounded-xl border border-emerald-900/50 bg-emerald-950/20 p-4">

                        <p className="text-sm font-medium text-emerald-300">
                          {packageActionMessage}
                        </p>

                      </div>
                    )}

                  <div className="mt-5 flex flex-wrap gap-3">

                    {!packageInstalled &&
                      !checkingInstalled && (
                        <button
                          onClick={
                            installSelectedPackage
                          }
                          disabled={
                            installing ||
                            removing
                          }
                          className="rounded-lg bg-blue-600 px-5 py-2.5 text-sm font-medium text-white transition hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50"
                        >
                          {installing
                            ? "Instalando..."
                            : "Instalar"}
                        </button>
                      )}

                    {packageInstalled &&
                      !checkingInstalled && (
                        <>
                          <button
                            disabled
                            className="cursor-default rounded-lg bg-emerald-600/20 px-5 py-2.5 text-sm font-medium text-emerald-400"
                          >
                            ✓ Instalado
                          </button>

                          <button
                            onClick={
                              removeSelectedPackage
                            }
                            disabled={
                              installing ||
                              removing
                            }
                            className="rounded-lg border border-red-900/60 px-5 py-2.5 text-sm font-medium text-red-400 transition hover:bg-red-950/40 hover:text-red-300 disabled:cursor-not-allowed disabled:opacity-50"
                          >
                            {removing
                              ? "Desinstalando..."
                              : "Desinstalar"}
                          </button>
                        </>
                      )}

                  </div>

                  <p className="mt-3 text-xs text-zinc-600">

                    {packageInstalled
                      ? "Puedes desinstalar este paquete desde RepoFy."
                      : "RepoFy utilizará pkexec para solicitar permisos de administrador."}

                  </p>

                </div>

              </section>

          </div>
        </main>
      </div>
    );
  }

  /*
   * VISTA PRINCIPAL
   */
  return (
    <div className="min-h-screen bg-[#09090b] text-zinc-100">
      <main className="min-h-screen overflow-auto">

        {/* TOP BAR */}
        <header className="flex h-16 items-center justify-between border-b border-zinc-800/80 px-8">

          <div className="flex items-center gap-3 text-xs text-zinc-600">
            <img
              src={logoIcon}
              alt="Logo de RepoFy"
              className="h-9 w-9 rounded-xl"
            />
            <span>
              RepoFy
            </span>
          </div>

          <button
            onClick={openSettings}
            className="flex h-9 w-9 items-center justify-center rounded-lg text-zinc-500 transition hover:bg-zinc-800 hover:text-white"
            title="Ajustes"
          >
            ⚙
          </button>

        </header>

        <div className="mx-auto flex max-w-6xl flex-col items-center px-8 py-14">

            {/* HERO */}
            <section className="mb-12 flex w-full flex-col items-center text-center">

              <div className="mb-4 inline-flex items-center gap-2 rounded-full border border-blue-500/20 bg-blue-500/5 px-3 py-1.5 text-xs font-medium text-blue-400">

                <span className="h-1.5 w-1.5 rounded-full bg-blue-400" />

                Software para Linux

              </div>

              <h1 className="max-w-2xl text-4xl font-bold tracking-tight text-white md:text-5xl">

                Encuentra el software
                <br />
                que necesitas.

              </h1>

              <p className="mt-5 max-w-2xl text-base leading-7 text-zinc-500">

                Busca aplicaciones y paquetes disponibles
                en los repositorios de tu sistema.

              </p>

              {/* SEARCH */}
              <div className="mt-8 w-full max-w-5xl">

                <form
                  onSubmit={(event) => {
                    event.preventDefault();
                    searchPackages();
                  }}
                >

                  <div className="group flex items-center rounded-2xl border border-zinc-800 bg-zinc-900/80 px-4 shadow-2xl shadow-black/20 transition focus-within:border-blue-500/50 focus-within:ring-4 focus-within:ring-blue-500/5">

                    <span className="mr-3 text-xl text-zinc-500">
                      ⌕
                    </span>

                    <input
                      id="repofy-search"
                      type="text"
                      value={searchQuery}
                      onChange={(event) => {
                        setSearchQuery(
                          event.target.value
                        );

                        setSearchError("");
                      }}
                      placeholder="Buscar una aplicación o paquete..."
                      className="h-16 flex-1 bg-transparent text-base text-white outline-none placeholder:text-zinc-600"
                    />

                    <button
                      type="submit"
                      disabled={searching}
                      className="hidden rounded-lg bg-blue-600 px-4 py-2 text-sm font-medium text-white transition hover:bg-blue-500 disabled:cursor-not-allowed disabled:opacity-50 sm:block"
                    >
                      {searching
                        ? "Buscando..."
                        : "Buscar"}
                    </button>

                  </div>

                </form>

                <div className="mt-3 flex items-center gap-2 px-2 text-xs text-zinc-600">

                  <span>
                    Ejemplos:
                  </span>

                  <button
                    onClick={() => {
                      setSearchQuery(
                        "firefox"
                      );
                    }}
                    className="transition hover:text-zinc-400"
                  >
                    Firefox
                  </button>

                  <span>
                    ·
                  </span>

                  <button
                    onClick={() => {
                      setSearchQuery(
                        "steam"
                      );
                    }}
                    className="transition hover:text-zinc-400"
                  >
                    Steam
                  </button>

                  <span>
                    ·
                  </span>

                  <button
                    onClick={() => {
                      setSearchQuery(
                        "vlc"
                      );
                    }}
                    className="transition hover:text-zinc-400"
                  >
                    VLC
                  </button>

                </div>

              </div>

            </section>

            {/* RESULTADOS */}
            {(searching ||
              searchError ||
              packages.length > 0 ||
              searchQuery.trim()) && (

              <section className="mb-14 w-full max-w-4xl">

                <div className="mb-5 flex items-end justify-between">

                  <div>

                    <h2 className="text-xl font-semibold text-white">
                      Resultados
                    </h2>

                    <p className="mt-1 text-sm text-zinc-600">

                      {searching
                        ? "Buscando en los repositorios..."
                        : `${packages.length} resultado${
                            packages.length ===
                            1
                              ? ""
                              : "s"
                          } encontrado${
                            packages.length ===
                            1
                              ? ""
                              : "s"
                          }`}

                    </p>

                  </div>

                  {packages.length > 0 &&
                    !searching && (
                      <span className="text-xs text-zinc-600">
                        Ordenados por relevancia
                      </span>
                    )}

                </div>

                {/* BUSCANDO */}
                {searching && (
                  <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-8 text-center">

                    <div className="mx-auto mb-3 h-6 w-6 animate-spin rounded-full border-2 border-zinc-700 border-t-blue-500" />

                    <p className="text-sm text-zinc-400">
                      Buscando paquetes...
                    </p>

                  </div>
                )}

                {/* ERROR */}
                {searchError &&
                  !searching && (
                    <div className="rounded-2xl border border-red-900/50 bg-red-950/20 p-5 text-sm text-red-300">
                      {searchError}
                    </div>
                  )}

                {/* RESULTADOS */}
                {!searching &&
                  !searchError &&
                  packages.length > 0 && (

                    <div className="grid gap-3">

                      {packages.map(
                        (pkg, index) => {

                          const isExact =
                            pkg.name
                              .toLowerCase() ===
                            searchQuery
                              .trim()
                              .toLowerCase();

                          const status =
                            getInstallationStatus(
                              pkg.name
                            );

                          return (
                            <div
                              key={`${pkg.repository}-${pkg.name}-${pkg.version}-${index}`}
                              className="group rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5 transition duration-200 hover:border-zinc-700 hover:bg-zinc-900"
                            >

                              <div className="flex items-start gap-4">

                                {/* ICONO */}
                                <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-xl border border-zinc-800 bg-zinc-950 text-lg font-semibold text-zinc-400">
                                  {pkg.name
                                    .charAt(0)
                                    .toUpperCase()}
                                </div>

                                {/* INFO */}
                                <div className="min-w-0 flex-1">

                                  <div className="flex flex-wrap items-center gap-2">

                                    <h3 className="font-semibold text-white">
                                      {pkg.name}
                                    </h3>

                                    {isExact && (
                                      <span className="rounded-full bg-blue-500/10 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide text-blue-400">
                                        Coincidencia exacta
                                      </span>
                                    )}

                                    {/* ESTADO */}
                                    {status ===
                                      "checking" && (
                                      <span className="flex items-center gap-1.5 rounded-full bg-zinc-800 px-2 py-1 text-[10px] font-semibold text-zinc-500">

                                        <span className="h-2.5 w-2.5 animate-spin rounded-full border-2 border-zinc-600 border-t-blue-500" />

                                        Comprobando

                                      </span>
                                    )}

                                    {status ===
                                      "installed" && (
                                      <span className="rounded-full bg-emerald-500/10 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide text-emerald-400">
                                        ✓ Instalado
                                      </span>
                                    )}

                                    {status ===
                                      "available" && (
                                      <span className="rounded-full bg-zinc-800 px-2 py-1 text-[10px] font-semibold uppercase tracking-wide text-zinc-500">
                                        Disponible
                                      </span>
                                    )}

                                  </div>

                                  <p className="mt-1 text-sm text-zinc-500">
                                    {pkg.description ||
                                      "Sin descripción disponible."}
                                  </p>

                                  <div className="mt-4 flex flex-wrap items-center gap-2">

                                    <span className="inline-flex items-center rounded-md border border-zinc-800 bg-zinc-950 px-2.5 py-1 text-xs text-zinc-400">

                                      <span className="mr-1.5 text-zinc-600">
                                        Versión
                                      </span>

                                      {pkg.version}

                                    </span>

                                    <span className="inline-flex items-center rounded-md border border-blue-500/20 bg-blue-500/10 px-2.5 py-1 text-xs font-medium text-blue-400">
                                      {pkg.repository}
                                    </span>

                                    <span className="inline-flex items-center rounded-md border border-zinc-800 bg-zinc-950 px-2.5 py-1 text-xs text-zinc-500">
                                      {pkg.manager}
                                    </span>

                                  </div>

                                </div>

                                {/* DETALLES */}
                                <button
                                  onClick={() =>
                                    openPackageDetails(
                                      pkg
                                    )
                                  }
                                  className="hidden shrink-0 rounded-lg border border-zinc-700 px-3 py-2 text-xs font-medium text-zinc-400 transition hover:border-zinc-600 hover:bg-zinc-800 hover:text-white sm:block"
                                >
                                  Ver detalles
                                </button>

                              </div>

                            </div>
                          );
                        }
                      )}

                    </div>
                  )}

                {/* SIN RESULTADOS */}
                {!searching &&
                  !searchError &&
                  searchQuery.trim() &&
                  packages.length === 0 && (

                    <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-10 text-center">

                      <div className="mx-auto mb-4 flex h-12 w-12 items-center justify-center rounded-xl bg-zinc-800 text-xl text-zinc-500">
                        ?
                      </div>

                      <h3 className="font-medium text-white">
                        No encontramos resultados
                      </h3>

                      <p className="mt-2 text-sm text-zinc-600">

                        No se encontraron paquetes para{" "}

                        <span className="text-zinc-400">
                          {searchQuery}
                        </span>

                        .

                      </p>

                    </div>
                  )}

              </section>
            )}

            {/* SISTEMA */}
            <section className="w-full max-w-4xl">

              <div className="mb-5">

                <h2 className="text-xl font-semibold text-white">
                  Tu sistema
                </h2>

                <p className="mt-1 text-sm text-zinc-600">
                  Información detectada automáticamente por RepoFy
                </p>

              </div>

              <div className="grid gap-3 md:grid-cols-2">

                {/* DISTRIBUCIÓN */}
                <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">

                  <div className="flex items-center justify-between">

                    <div>

                      <p className="text-xs font-medium uppercase tracking-wide text-zinc-600">
                        Distribución
                      </p>

                      <p className="mt-2 text-lg font-semibold text-white">
                        {systemInfo?.distribution ??
                          "Detectando..."}
                      </p>

                      <p className="mt-1 text-sm text-zinc-500">
                        Familia{" "}
                        {systemInfo?.family ??
                          "Linux"}
                      </p>

                    </div>

                    <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-zinc-800 text-lg">
                      🐧
                    </div>

                  </div>

                </div>

                {/* ARQUITECTURA */}
                <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">

                  <div className="flex items-center justify-between">

                    <div>

                      <p className="text-xs font-medium uppercase tracking-wide text-zinc-600">
                        Arquitectura
                      </p>

                      <p className="mt-2 text-lg font-semibold text-white">
                        {systemInfo?.architecture ??
                          "Detectando..."}
                      </p>

                      <p className="mt-1 text-sm text-zinc-500">

                        {systemInfo?.architecture ===
                        "x86_64"
                          ? "Sistema de 64 bits"
                          : "Arquitectura detectada"}

                      </p>

                    </div>

                    <div className="flex h-11 w-11 items-center justify-center rounded-xl bg-zinc-800 text-lg">
                      ⚙
                    </div>

                  </div>

                </div>

              </div>

            </section>

            {/* PACKAGE MANAGERS */}
            <section className="mt-4 w-full max-w-4xl">

              <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">

                <div className="mb-5">

                  <h2 className="font-semibold text-white">
                    Gestores disponibles
                  </h2>

                  <p className="mt-1 text-sm text-zinc-600">
                    Herramientas disponibles para instalar software
                  </p>

                </div>

                <div className="flex flex-wrap gap-2">

                  {systemInfo?.package_managers.map(
                    (manager) => (

                      <div
                        key={manager}
                        className="flex items-center gap-2 rounded-lg border border-zinc-800 bg-zinc-950 px-3 py-2"
                      >

                        <span className="h-1.5 w-1.5 rounded-full bg-emerald-400" />

                        <span className="text-sm text-zinc-400">
                          {manager}
                        </span>

                        <span className="text-[10px] uppercase text-emerald-500">
                          Disponible
                        </span>

                      </div>

                    )
                  )}

                </div>

              </div>

            </section>

        </div>
      </main>
    </div>
  );
}

export default App;