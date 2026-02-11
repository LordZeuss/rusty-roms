<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { writable, get } from "svelte/store";
  import { listen } from "@tauri-apps/api/event";
  import { onMount } from "svelte";
  import { browser } from "$app/environment";
  import "../app.css";

  type Game = {
    id: number;
    name: string;
    dl_link: string;
    console: string;
    size: string;
    is_downloaded: boolean;
  };

  let searchTerm = "";

  const games = writable<Game[]>([]);
  const error = writable<string>("");

  // status check: null = unknown/loading, true = up, false = down
  const serverUp = writable<boolean | null>(null);

  // progressById[id] = "12.34%"
  const progressById = writable<Record<number, string>>({});
  const downloadingIds = writable<Set<number>>(new Set());

  // --- search-as-you-type (SSR-safe) ---
  let mounted = false;
  let searchTimer: ReturnType<typeof setTimeout> | null = null;
  let hasSearched = false;

  // --- settings: download dir ---
  let downloadDir = ""; // current saved value from backend
  let downloadDirInput = ""; // what user edits in UI
  let savingDir = false;

  // --- update library progress ---
  type StartupPayload = { percent: number; message: string };
  let startupRunning = false;
  let startupPercent = 0;
  let startupMessage = "";

  async function refreshServerStatus() {
    try {
      const ok = await invoke<boolean>("network_check");
      serverUp.set(ok);
    } catch (e) {
      console.error("Status check failed:", e);
      serverUp.set(false);
    }
  }

  async function search(term: string) {
    const t = term.trim();

    if (!t) {
      games.set([]);
      error.set("");
      hasSearched = false;
      return;
    }

    hasSearched = true;

    try {
      const results = await invoke("search_games", { search: t });
      games.set(results as Game[]);
      error.set("");
    } catch (e) {
      error.set(String(e));
      games.set([]);
    }
  }

  // Debounced search whenever searchTerm changes (only in browser, after mount)
  $: if (browser && mounted) {
    if (searchTimer) clearTimeout(searchTimer);

    const t = searchTerm.trim();

    if (t.length === 0) {
      games.set([]);
      error.set("");
      hasSearched = false;
    } else {
      searchTimer = setTimeout(() => {
        search(searchTerm);
      }, 250);
    }
  }

  async function downloadGame(game: Game) {
    const id = game.id;

    // Block downloads if server is down
    if (get(serverUp) === false) {
      console.warn("Server is down; blocking download.");
      error.set("Server is down. Please try again later.");
      return;
    }

    // Prevent double-click while downloading
    if (get(downloadingIds).has(id)) return;

    downloadingIds.update((s) => new Set(s).add(id));
    progressById.update((m) => ({ ...m, [id]: "0.00%" }));

    try {
      const result = await invoke<string>("download_file", {
        url: game.dl_link,
        fileName: game.name,
        id,
      });

      console.log(result);
      progressById.update((m) => ({ ...m, [id]: "100.00%" }));
    } catch (e) {
      console.error("Download failed:", e);

      progressById.update((m) => {
        const { [id]: _, ...rest } = m;
        return rest;
      });
    } finally {
      downloadingIds.update((s) => {
        const copy = new Set(s);
        copy.delete(id);
        return copy;
      });

      // optional: remove the progress after a moment
      setTimeout(() => {
        progressById.update((m) => {
          const { [id]: _, ...rest } = m;
          return rest;
        });
      }, 1500);
    }
  }

  // ---------- drawer: download dir helpers ----------
  async function loadDownloadDir() {
    try {
      const dir = await invoke<string>("get_download_dir");
      downloadDir = dir;
      downloadDirInput = dir;
    } catch (e) {
      console.error("Failed to load download dir:", e);
      error.set(`Failed to load download dir: ${String(e)}`);
    }
  }

  async function pickDir() {
    try {
      // returns string | null
      const picked = await invoke<string | null>("pick_download_dir");
      if (picked) {
        downloadDirInput = picked;
      }
    } catch (e) {
      console.error("Pick folder failed:", e);
      error.set(`Pick folder failed: ${String(e)}`);
    }
  }

  async function saveDir() {
    const path = downloadDirInput.trim();
    if (!path) {
      error.set("Path cannot be empty.");
      return;
    }

    savingDir = true;
    try {
      await invoke("set_download_dir", { path });
      downloadDir = path;
      error.set("");
    } catch (e) {
      console.error("Save dir failed:", e);
      error.set(`Save dir failed: ${String(e)}`);
    } finally {
      savingDir = false;
    }
  }

  async function resetDirToDefault() {
    savingDir = true;
    try {
      // your backend: clears saved value so default is used
      await invoke("clear_download_dir");
      await loadDownloadDir();
      error.set("");
    } catch (e) {
      console.error("Reset dir failed:", e);
      error.set(`Reset dir failed: ${String(e)}`);
    } finally {
      savingDir = false;
    }
  }

  // ---------- drawer: update library ----------
  async function runUpdateLibrary() {
    startupRunning = true;
    startupPercent = 0;
    startupMessage = "Starting…";
    error.set("");

    try {
      await invoke("run_startup_tasks");
      // Optional: re-run current search after update
      if (searchTerm.trim()) {
        await search(searchTerm);
      }
    } catch (e) {
      console.error("Update library failed:", e);
      startupMessage = `Failed: ${String(e)}`;
      error.set(`Update library failed: ${String(e)}`);
    } finally {
      startupRunning = false;
    }
  }

  type ProgressPayload = { id: number; progress: string };
  type CompletePayload = { id: number };

  onMount(() => {
    mounted = true;

    // initial server check + poll
    refreshServerStatus();
    const interval = setInterval(refreshServerStatus, 15000);

    // load settings for drawer
    loadDownloadDir();

    const unlistenProgress = listen<ProgressPayload>(
      "download-progress",
      (event) => {
        const { id, progress } = event.payload;
        progressById.update((m) => ({ ...m, [id]: progress }));
      },
    );

    const unlistenComplete = listen<CompletePayload>(
      "download-complete",
      (event) => {
        const { id } = event.payload;
        games.update((list) =>
          list.map((g) => (g.id === id ? { ...g, is_downloaded: true } : g)),
        );
      },
    );

    const unlistenStartup = listen<StartupPayload>(
      "startup-progress",
      (event) => {
        startupPercent = event.payload.percent;
        startupMessage = event.payload.message;
      },
    );

    return () => {
      clearInterval(interval);
      unlistenProgress.then((f) => f());
      unlistenComplete.then((f) => f());
      unlistenStartup.then((f) => f());
    };
  });

  function openDrawer() {
    const el = document.getElementById(
      "settings-drawer",
    ) as HTMLInputElement | null;
    if (el) el.checked = true;
  }
</script>

<div class="drawer drawer-end">
  <input id="settings-drawer" type="checkbox" class="drawer-toggle" />

  <!-- Page content -->
  <div class="drawer-content pb-24">
    <div class="pt-10">
      <div class="flex items-center justify-center gap-3 mx-auto">
        <!-- Invisible spacer to keep the search centered -->
        <button
          class="btn btn-ghost btn-square invisible"
          aria-hidden="true"
          tabindex="-1"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="h-5 w-5"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
          >
            <path
              stroke-linecap="round"
              stroke-linejoin="round"
              stroke-width="2"
              d="M11.983 2.25a1.5 1.5 0 011.5 1.5v.33a7.44 7.44 0 012.01.83l.234-.234a1.5 1.5 0 112.121 2.121l-.234.234c.36.62.64 1.3.83 2.01h.33a1.5 1.5 0 010 3h-.33a7.44 7.44 0 01-.83 2.01l.234.234a1.5 1.5 0 11-2.121 2.121l-.234-.234a7.44 7.44 0 01-2.01.83v.33a1.5 1.5 0 01-3 0v-.33a7.44 7.44 0 01-2.01-.83l-.234.234a1.5 1.5 0 11-2.121-2.121l.234-.234a7.44 7.44 0 01-.83-2.01h-.33a1.5 1.5 0 010-3h.33c.19-.71.47-1.39.83-2.01l-.234-.234a1.5 1.5 0 112.121-2.121l.234.234c.62-.36 1.3-.64 2.01-.83v-.33a1.5 1.5 0 011.5-1.5z"
            />
            <circle cx="12" cy="12" r="3" />
          </svg>
        </button>

        <!-- Search input -->
        <label class="input">
          <svg
            class="h-[1em] opacity-50"
            xmlns="http://www.w3.org/2000/svg"
            viewBox="0 0 24 24"
          >
            <g
              stroke-linejoin="round"
              stroke-linecap="round"
              stroke-width="2.5"
              fill="none"
              stroke="currentColor"
            >
              <circle cx="11" cy="11" r="8"></circle>
              <path d="m21 21-4.3-4.3"></path>
            </g>
          </svg>
          <input
            type="search"
            bind:value={searchTerm}
            placeholder="Search for Rom"
          />
        </label>
      </div>

      <!-- Server status line -->
      <div class="flex justify-center mt-3">
        {#if $serverUp === null}
          <div class="inline-flex items-center gap-2 text-gray-400">
            <span class="loading loading-spinner loading-xs"></span>
            Checking server…
          </div>
        {:else if $serverUp}
          <div class="inline-flex items-center gap-2 text-success">
            <div class="inline-grid *:[grid-area:1/1]">
              <div class="status status-success animate-ping"></div>
              <div class="status status-success"></div>
            </div>
            <div class="badge badge-success">Online</div>
          </div>
        {:else}
          <div class="inline-flex items-center gap-2 text-error">
            <div class="inline-grid *:[grid-area:1/1]">
              <div class="status status-error animate-ping"></div>
              <div class="status status-error"></div>
            </div>
            <div class="badge badge-error">Offline. Downloads Disabled.</div>
          </div>
        {/if}
      </div>
    </div>

    {#if $error}
      <p style="color:red" class="text-center mt-4">Error: {$error}</p>
    {/if}

    {#if hasSearched && !$error && $games.length === 0}
      <p class="text-center text-gray-400 mt-6">No results found</p>
    {/if}

    <div class="max-w-6xl mx-auto px-4 p-5">
      <div
        class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-4 lg:gap-13"
      >
        {#each $games as game}
          <div class="card flex flex-col bg-zinc-700 w-96 shadow-sm">
            <div class="card-body">
              <h3 class="card-title">{game.name}</h3>

              <div class="card-actions mt-auto justify-end mx-auto">
                <button
                  on:click={() => downloadGame(game)}
                  class={`btn btn-xs ${game.is_downloaded ? "btn-error" : "btn-success"}`}
                  disabled={$downloadingIds.has(game.id) || $serverUp === false}
                  title={$serverUp === false ? "Server is down" : ""}
                >
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    class="h-4 w-4"
                    fill="none"
                    viewBox="0 0 24 24"
                    stroke="currentColor"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="2"
                      d="M4 16v2a2 2 0 002 2h12a2 2 0 002-2v-2M7 10l5 5 5-5M12 15V3"
                    />
                  </svg>

                  {#if $downloadingIds.has(game.id)}
                    {$progressById[game.id] ?? "0.00%"}
                  {:else if game.is_downloaded}
                    Redownload
                  {:else}
                    Download
                  {/if}
                </button>

                <div class="badge bg-blue-600">{game.console}</div>
                <div class="badge bg-red-600">{game.size}</div>
              </div>
            </div>
          </div>
        {/each}
      </div>
    </div>
  </div>

  <!-- Drawer panel -->
  <div class="drawer-side">
    <label
      for="settings-drawer"
      aria-label="close sidebar"
      class="drawer-overlay"
    ></label>

    <div class="bg-base-200 min-h-full w-80 p-4 flex flex-col gap-4">
      <div class="text-lg font-semibold">Settings</div>

      <!-- Download directory -->
      <div class="card bg-base-100 shadow">
        <div class="card-body">
          <h3 class="card-title text-base">Download Location</h3>

          <div class="text-xs opacity-70">
            Current:
            <div class="mt-1 font-mono break-words">{downloadDir}</div>
          </div>

          <label class="form-control mt-3">
            <div class="label">
              <span class="label-text text-sm">Path</span>
            </div>
            <input
              class="input input-bordered w-full"
              bind:value={downloadDirInput}
              placeholder="/home/user/Downloads/Roms"
            />
          </label>

          <div class="flex gap-2 mt-3">
            <button
              class="btn btn-outline btn-sm"
              on:click={pickDir}
              disabled={savingDir || startupRunning}
            >
              Pick…
            </button>

            <button
              class="btn btn-primary btn-sm"
              on:click={saveDir}
              disabled={savingDir || startupRunning}
            >
              {#if savingDir}Saving…{:else}Save{/if}
            </button>

            <button
              class="btn btn-ghost btn-sm"
              on:click={resetDirToDefault}
              disabled={savingDir || startupRunning}
            >
              Reset
            </button>
          </div>

          <p class="text-xs opacity-60 mt-2">
            Reset clears your saved setting and uses the app default.
          </p>
        </div>
      </div>

      <!-- Update library -->
      <div class="card bg-base-100 shadow">
        <div class="card-body">
          <h3 class="card-title text-base">Update Library</h3>
          <p class="text-xs opacity-70">
            Rebuild DB and scrape the latest ROM list.
          </p>

          <button
            class="btn btn-primary mt-2"
            on:click={runUpdateLibrary}
            disabled={startupRunning}
          >
            {#if startupRunning}
              Updating… {startupPercent}%
            {:else}
              Update Library
            {/if}
          </button>

          {#if startupRunning}
            <progress
              class="progress progress-primary w-full mt-3"
              value={startupPercent}
              max="100"
            ></progress>
            <div class="text-xs opacity-70 mt-2">{startupMessage}</div>
          {:else if startupMessage}
            <div class="text-xs opacity-70 mt-2">{startupMessage}</div>
          {/if}
        </div>
      </div>
    </div>
  </div>
</div>

<!-- Dock -->
<div
  class="dock fixed bottom-0 left-0 right-0 z-50 bg-base-200/80 backdrop-blur border-t border-base-300"
>
  <button>
    <svg
      class="size-[1.2em]"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
    >
      <g fill="currentColor" stroke-linejoin="miter" stroke-linecap="butt">
        <polyline
          points="1 11 12 2 23 11"
          fill="none"
          stroke="currentColor"
          stroke-miterlimit="10"
          stroke-width="2"
        ></polyline>
        <path
          d="m5,13v7c0,1.105.895,2,2,2h10c1.105,0,2-.895,2-2v-7"
          fill="none"
          stroke="currentColor"
          stroke-linecap="square"
          stroke-miterlimit="10"
          stroke-width="2"
        ></path>
        <line
          x1="12"
          y1="22"
          x2="12"
          y2="18"
          fill="none"
          stroke="currentColor"
          stroke-linecap="square"
          stroke-miterlimit="10"
          stroke-width="2"
        ></line>
      </g>
    </svg>
    <span class="dock-label">Home</span>
  </button>

  <!-- Settings opens the drawer (no navigation) -->
  <button on:click={openDrawer}>
    <svg
      class="size-[1.2em]"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
    >
      <g fill="currentColor" stroke-linejoin="miter" stroke-linecap="butt">
        <circle
          cx="12"
          cy="12"
          r="3"
          fill="none"
          stroke="currentColor"
          stroke-linecap="square"
          stroke-miterlimit="10"
          stroke-width="2"
        ></circle>
        <path
          d="m22,13.25v-2.5l-2.318-.966c-.167-.581-.395-1.135-.682-1.654l.954-2.318-1.768-1.768-2.318.954c-.518-.287-1.073-.515-1.654-.682l-.966-2.318h-2.5l-.966,2.318c-.581.167-1.135.395-1.654.682l-2.318-.954-1.768,1.768.954,2.318c-.287.518-.515,1.073-.682,1.654l-2.318.966v2.5l2.318.966c.167.581.395,1.135.682,1.654l-.954,2.318,1.768,1.768,2.318-.954c.518.287,1.073.515,1.654.682l.966,2.318h2.5l.966-2.318c.581-.167,1.135-.395,1.654-.682l2.318.954,1.768-1.768-.954-2.318c.287-.518.515,1.073.682-1.654l2.318-.966Z"
          fill="none"
          stroke="currentColor"
          stroke-linecap="square"
          stroke-miterlimit="10"
          stroke-width="2"
        ></path>
      </g>
    </svg>
    <span class="dock-label">Settings</span>
  </button>
</div>
