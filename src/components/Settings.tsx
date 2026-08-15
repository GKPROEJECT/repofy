import logoIcon from "../../logo.ico";

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
}

function Settings({
  onBack,
  systemInfo,
  rememberLastSearch,
  setRememberLastSearch,
  clearSavedSearch,
}: SettingsProps) {
  return (
    <div className="min-h-screen bg-[#09090b] text-zinc-100">
      <main className="min-h-screen overflow-auto">

        <header className="flex h-16 items-center justify-between border-b border-zinc-800/80 px-8">

          <button
            onClick={onBack}
            className="text-sm text-zinc-500 transition hover:text-white"
          >
            ← Volver
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
                Ajustes
              </h1>

              <p className="mt-3 text-base text-zinc-500">
                Personaliza RepoFy según tus preferencias.
              </p>

            </div>

            {/* EXPERIENCIA */}
            <section className="mb-8 w-full">

              <h2 className="mb-4 text-xl font-semibold text-white">
                Experiencia
              </h2>

              <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6">
                <div className="flex flex-col gap-5 lg:flex-row lg:items-center lg:justify-between">

                  <div>

                    <h3 className="font-medium text-white">
                      Recordar la última búsqueda
                    </h3>

                    <p className="mt-1 text-sm text-zinc-500">
                      Guarda el texto de búsqueda para retomarlo la próxima vez que abras RepoFy.
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
                      ? "Activado"
                      : "Desactivado"}
                  </button>

                </div>

                <div className="mt-5 rounded-xl border border-zinc-800 bg-zinc-950 p-4">
                  <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
                    <div>
                      <h3 className="font-medium text-white">
                        Limpiar búsqueda guardada
                      </h3>

                      <p className="mt-1 text-sm text-zinc-500">
                        Elimina el texto recordado y deja la pantalla de inicio limpia.
                      </p>
                    </div>

                    <button
                      type="button"
                      onClick={clearSavedSearch}
                      className="rounded-lg border border-zinc-700 px-4 py-2.5 text-sm font-medium text-zinc-300 transition hover:border-zinc-600 hover:bg-zinc-900 hover:text-white"
                    >
                      Borrar búsqueda
                    </button>
                  </div>
                </div>
              </div>

            </section>

            {/* APARIENCIA */}
            <section className="mb-8 w-full">

              <h2 className="mb-4 text-xl font-semibold text-white">
                Apariencia
              </h2>

              <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-6">

                <h3 className="font-medium text-white">
                  Tema actual
                </h3>

                <p className="mt-1 text-sm text-zinc-500">
                  La interfaz de RepoFy utiliza actualmente el tema oscuro.
                </p>

                <div className="mt-5 grid gap-3 sm:grid-cols-2">
                  <div className="rounded-xl border border-blue-500 bg-blue-500/10 p-4">
                    <div className="text-lg">
                      🌙
                    </div>

                    <div className="mt-2 font-medium text-white">
                      Oscuro
                    </div>

                    <div className="mt-1 text-xs text-zinc-400">
                      Activo
                    </div>
                  </div>

                  <div className="rounded-xl border border-zinc-800 bg-zinc-950 p-4 opacity-70">
                    <div className="text-lg">
                      ☀️
                    </div>

                    <div className="mt-2 font-medium text-white">
                      Claro
                    </div>

                    <div className="mt-1 text-xs text-zinc-500">
                      Disponible en una futura mejora
                    </div>
                  </div>
                </div>

              </div>

            </section>

            {/* SISTEMA */}
            <section className="mb-8 w-full">

              <h2 className="mb-4 text-xl font-semibold text-white">
                Sistema
              </h2>

              <div className="grid gap-3 md:grid-cols-2">
                <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">
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

                <div className="rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">
                  <p className="text-xs font-medium uppercase tracking-wide text-zinc-600">
                    Arquitectura
                  </p>

                  <p className="mt-2 text-lg font-semibold text-white">
                    {systemInfo?.architecture ??
                      "Detectando..."}
                  </p>
                </div>
              </div>

              <div className="mt-3 rounded-2xl border border-zinc-800 bg-zinc-900/60 p-5">
                <h3 className="font-medium text-white">
                  Gestores de paquetes detectados
                </h3>

                <p className="mt-1 text-sm text-zinc-500">
                  RepoFy usará estos gestores para encontrar e instalar software.
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
                      Detectando gestores disponibles...
                    </p>
                  )}
                </div>
              </div>
            </section>

            {/* ACERCA DE */}
            <section className="w-full">

              <h2 className="mb-4 text-xl font-semibold text-white">
                Acerca de RepoFy
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
                      Software Center para Linux
                    </p>

                  </div>

                </div>

                <div className="mt-6 border-t border-zinc-800 pt-5">

                  <p className="text-sm text-zinc-500">
                    Una forma sencilla de descubrir e instalar software en Linux.
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