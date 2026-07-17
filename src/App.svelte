<script lang="ts">
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, tick } from "svelte";
  import {
    ArrowLeft,
    Box,
    Bug,
    Check,
    ClipboardList,
    Copy,
    DownloadCloud,
    Heart,
    Loader2,
    PackageSearch,
    Play,
    Search,
    Settings,
    Star,
    Terminal,
    Trash2,
    Wrench,
    X,
  } from "lucide-svelte";

  type CommandMatch = {
    plugin_id: string;
    plugin_name: string;
    command_id: string;
    title: string;
    keyword: string;
    score: number;
  };

  type PluginView = {
    id: string;
    name: string;
    version: string;
    description: string;
    icon: string;
    runtime: string;
    entry: string;
    bundled: boolean;
    permissions: string[];
    commands: number;
  };

  type MarketplaceEntry = {
    id: string;
    name: string;
    version: string;
    description: string;
    icon: string;
    runtime: string;
    entry: string;
    bundled: boolean;
    download_url: string;
    sha256?: string | null;
    permissions: string[];
  };

  type RpcResult =
    | { type: "text"; text: string }
    | { type: "markdown"; markdown: string }
    | { type: "list"; items: Array<{ title: string; subtitle?: string; action?: RpcAction }> }
    | { type: "error"; message: string };

  type RpcAction =
    | { type: "copy"; value: string }
    | { type: "open_url"; url: string }
    | { type: "shell"; command: string };

  type ClipboardItem = {
    id: string;
    kind: "text" | "image" | "file";
    text: string;
    preview: string;
    copied_at: string;
    favorite?: boolean;
    image_path?: string;
    width?: number;
    height?: number;
    file_paths?: string[];
  };

  type ClipboardFilter = "all" | "text" | "image" | "file" | "favorite";

  type View = "launcher" | "finder" | "clipboard" | "installed" | "dev" | "settings";

  const CLIPBOARD_APP_ID = "dev.vvicat.system-clipboard";
  const params = new URLSearchParams(window.location.search);
  const isClipboardWindow = params.get("window") === "clipboard";
  const categories = [
    { view: "finder" as const, label: "插件市场" },
  ];

  let view: View = isClipboardWindow ? "clipboard" : params.get("view") === "finder" ? "finder" : "launcher";
  let query = "";
  let selectedIndex = 0;
  let selectedMarketId = "";
  let selectedPluginId = "";
  let commands: CommandMatch[] = [];
  let plugins: PluginView[] = [];
  let market: MarketplaceEntry[] = [];
  let clipboardItems: ClipboardItem[] = [];
  let result: RpcResult | null = null;
  let resultPluginId = "";
  let loading = false;
  let error = "";
  let clipboardStatus = "";
  let clipboardFilter: ClipboardFilter = "all";
  let searchInput: HTMLInputElement;
  let clipboardBoard: HTMLElement;
  let unlistenShowLauncher: (() => void) | undefined;
  let unlistenOpenSettings: (() => void) | undefined;
  let unlistenOpenClipboard: (() => void) | undefined;

  $: installedIds = new Set(plugins.map((plugin) => plugin.id));
  $: filteredMarket = market.filter((item) => matchText(item, query));
  $: filteredPlugins = plugins.filter((plugin) => matchText(plugin, query));
  $: filteredClipboardItems = clipboardItems.filter(
    (item) =>
      (clipboardFilter === "favorite" ? item.favorite : clipboardFilter === "all" || item.kind === clipboardFilter) &&
      matchClipboardText(item, query)
  );
  $: marketDetail = filteredMarket.find((item) => item.id === selectedMarketId) ?? filteredMarket[0];
  $: isClipboardApp = marketDetail?.id === CLIPBOARD_APP_ID;
  $: pluginDetail = filteredPlugins.find((plugin) => plugin.id === selectedPluginId) ?? filteredPlugins[0];
  $: launcherItems = query ? commands : [];
  $: recentItems = commands.slice(0, 8);
  $: visibleCount =
    view === "clipboard"
      ? filteredClipboardItems.length
      : view === "launcher"
        ? Math.max(launcherItems.length, recentItems.length)
        : filteredMarket.length;
  $: if (view === "clipboard") clampClipboardSelection();
  $: if (view === "clipboard" && filteredClipboardItems.length) void scrollSelectedClipboardItemIntoView();

  async function refreshAll() {
    error = "";
    try {
      plugins = await invoke("list_plugins");
      market = await invoke("load_marketplace");
      clipboardItems = await invoke("list_clipboard_history");
      await searchCommands();
      selectedMarketId ||= market[0]?.id ?? "";
      selectedPluginId ||= plugins[0]?.id ?? "";
    } catch (err) {
      error = String(err);
    }
  }

  async function searchCommands() {
    commands = await invoke("search_plugin_commands", { query });
    selectedIndex = 0;
  }

  async function onInput() {
    selectedIndex = 0;
    result = null;
    error = "";
    clipboardStatus = "";
    if (view === "launcher") {
      await searchCommands();
    }
  }

  async function run(command: CommandMatch) {
    if (command.plugin_id === CLIPBOARD_APP_ID) {
      await invoke("open_clipboard_window");
      return;
    }
    loading = true;
    error = "";
    result = null;
    try {
      result = await invoke("execute_plugin_command", {
        request: {
          plugin_id: command.plugin_id,
          command_id: command.command_id,
          query,
          context: { source: "launcher" },
        },
      });
      resultPluginId = command.plugin_id;
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  async function executeAction(action?: RpcAction) {
    if (!action || !resultPluginId) return;
    error = "";
    try {
      await invoke("execute_plugin_action", {
        request: {
          plugin_id: resultPluginId,
          action,
        },
      });
      if (action.type === "copy") {
        result = { type: "text", text: "已复制到剪切板" };
      }
    } catch (err) {
      error = String(err);
    }
  }

  async function install(entry: MarketplaceEntry) {
    if (entry.id === CLIPBOARD_APP_ID) return;
    const approved = window.confirm(
      `安装 ${entry.name}？\n\n版本：${entry.version}\n权限：${entry.permissions.join("、") || "无"}`
    );
    if (!approved) return;
    loading = true;
    error = "";
    try {
      await invoke("install_marketplace_plugin", { request: { entry, approved } });
      await refreshAll();
      view = "installed";
      query = "";
      selectedPluginId = entry.id;
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  async function openFeature(nextView: View = "finder") {
    view = nextView;
    query = "";
    result = null;
    resultPluginId = "";
    error = "";
    clipboardStatus = "";
    if (nextView === "clipboard") await loadClipboardHistory();
    await invoke("set_launcher_view", { view: "feature" });
    await tick();
    resetViewport();
    searchInput?.focus();
  }

  async function backToLauncher() {
    await invoke("set_launcher_view", { view: "launcher" });
    await resetLauncherState();
  }

  async function resetLauncherState() {
    view = "launcher";
    query = "";
    result = null;
    resultPluginId = "";
    error = "";
    await searchCommands();
    await tick();
    resetViewport();
    searchInput?.focus();
  }

  function handleShowLauncher() {
    void resetLauncherState();
  }

  async function openSettings() {
    view = "settings";
    query = "";
    result = null;
    resultPluginId = "";
    error = "";
    clipboardStatus = "";
    await tick();
    resetViewport();
    searchInput?.focus();
  }

  async function loadClipboardHistory() {
    try {
      clipboardItems = await invoke("list_clipboard_history");
    } catch (err) {
      error = String(err);
    }
  }

  async function copyClipboardItem(item: ClipboardItem, closeAfterCopy = false) {
    error = "";
    clipboardStatus = "";
    try {
      await invoke(closeAfterCopy ? "paste_clipboard_item" : "copy_clipboard_item", { request: { id: item.id } });
      clipboardStatus = "已复制";
      if (!closeAfterCopy) await loadClipboardHistory();
    } catch (err) {
      error = String(err);
    }
  }

  async function deleteClipboardItem(item: ClipboardItem) {
    error = "";
    clipboardStatus = "";
    try {
      await invoke("delete_clipboard_item", { id: item.id });
      await loadClipboardHistory();
    } catch (err) {
      error = String(err);
    }
  }

  async function toggleClipboardFavorite(item: ClipboardItem) {
    error = "";
    clipboardStatus = "";
    try {
      await invoke("toggle_clipboard_favorite", { id: item.id });
      await loadClipboardHistory();
    } catch (err) {
      error = String(err);
    }
  }

  async function clearClipboardHistory() {
    if (!clipboardItems.length) return;
    const approved = window.confirm("清空全部剪贴板历史？");
    if (!approved) return;
    error = "";
    clipboardStatus = "";
    try {
      await invoke("clear_clipboard_history");
      await loadClipboardHistory();
    } catch (err) {
      error = String(err);
    }
  }

  function handleOpenSettings() {
    void openSettings();
  }

  async function openClipboardPanel() {
    view = "clipboard";
    query = "";
    result = null;
    resultPluginId = "";
    error = "";
    clipboardStatus = "";
    clipboardFilter = "all";
    selectedIndex = 0;
    await loadClipboardHistory();
    selectedIndex = 0;
    await tick();
    resetViewport();
    searchInput?.focus();
  }

  function handleOpenClipboard() {
    void openClipboardPanel();
  }

  function resetViewport() {
    window.scrollTo(0, 0);
    document.documentElement.scrollTop = 0;
    document.body.scrollTop = 0;
  }

  async function hideLauncher() {
    try {
      await invoke("hide_launcher");
    } catch {
      // 浏览器预览环境没有 Tauri 窗口。
    }
  }

  async function hideCurrentWindow() {
    try {
      await invoke("hide_window", { label: isClipboardWindow ? "clipboard" : "main" });
    } catch {
      // 浏览器预览环境没有 Tauri 窗口。
    }
  }

  async function activateSelected() {
    if (view === "clipboard") {
      const item = filteredClipboardItems[selectedIndex];
      if (item) await copyClipboardItem(item, true);
      return;
    }
    if (view !== "launcher") {
      if (marketDetail && !installedIds.has(marketDetail.id)) await install(marketDetail);
      return;
    }
    const list = query ? launcherItems : recentItems;
    const command = list[selectedIndex];
    if (command) await run(command);
  }

  async function handleKeydown(event: KeyboardEvent) {
    if (
      event.key === "ArrowDown" ||
      event.key === "ArrowRight" ||
      (event.key === "Tab" && !event.shiftKey)
    ) {
      event.preventDefault();
      selectedIndex = Math.min(selectedIndex + 1, Math.max(visibleCount - 1, 0));
    } else if (event.key === "ArrowUp" || event.key === "ArrowLeft" || (event.key === "Tab" && event.shiftKey)) {
      event.preventDefault();
      selectedIndex = Math.max(selectedIndex - 1, 0);
    } else if (event.key === "Enter") {
      event.preventDefault();
      await activateSelected();
    } else if (event.key === "Escape") {
      event.preventDefault();
      if (isClipboardWindow) await hideCurrentWindow();
      else if (view === "launcher" && !result && !error) await hideLauncher();
      else if (view === "launcher") {
        result = null;
        error = "";
      } else {
        await backToLauncher();
      }
    }
  }

  function setClipboardFilter(filter: ClipboardFilter) {
    clipboardFilter = filter;
    selectedIndex = 0;
  }

  function clampClipboardSelection() {
    if (!filteredClipboardItems.length) {
      if (selectedIndex !== 0) selectedIndex = 0;
      return;
    }
    if (selectedIndex < 0) selectedIndex = 0;
    else if (selectedIndex > filteredClipboardItems.length - 1) selectedIndex = filteredClipboardItems.length - 1;
  }

  async function scrollSelectedClipboardItemIntoView() {
    await tick();
    const selected = clipboardBoard?.querySelector<HTMLElement>("[data-selected='true']");
    selected?.scrollIntoView({ block: "nearest", inline: "nearest" });
  }

  function matchText(item: MarketplaceEntry | PluginView, value: string) {
    const q = value.trim().toLowerCase();
    if (!q) return true;
    return [item.id, item.name, item.description, item.version, item.runtime, item.entry, item.permissions.join(" ")]
      .join(" ")
      .toLowerCase()
      .includes(q);
  }

  function matchClipboardText(item: ClipboardItem, value: string) {
    const q = value.trim().toLowerCase();
    if (!q) return true;
    return [item.preview, item.text, item.copied_at].join(" ").toLowerCase().includes(q);
  }

  function clipboardEmptyText() {
    if (query) return "没有匹配记录";
    if (clipboardFilter === "image") return "暂无图片记录";
    if (clipboardFilter === "file") return "暂无文件记录";
    if (clipboardFilter === "favorite") return "暂无收藏记录";
    if (clipboardFilter === "text") return "暂无文本记录";
    return "暂无剪贴板历史";
  }

  function clipboardImageSrc(item: ClipboardItem) {
    return item.image_path ? convertFileSrc(item.image_path) : "";
  }

  function clipboardItemLabel(item: ClipboardItem) {
    if (item.kind === "image") return "图片";
    if (item.kind === "file") return item.file_paths?.length && item.file_paths.length > 1 ? "多文件" : "文件";
    return "纯文本";
  }

  function clipboardItemMeta(item: ClipboardItem) {
    if (item.kind === "image") return `${item.width || 0} x ${item.height || 0}`;
    if (item.kind === "file") return `${item.file_paths?.length || 1} 个文件`;
    return `${item.text.length} 个字符`;
  }

  function commandDisplayName(command: CommandMatch) {
    return command.title || command.keyword;
  }

  function commandSubtitle(command: CommandMatch) {
    return command.plugin_name;
  }

  function launcherCardName(command: CommandMatch) {
    return command.plugin_name || command.title || command.keyword;
  }

  function formatClipboardTime(value: string) {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "";
    return date.toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  }

  function initials(value = "VT") {
    return value
      .split(/[\s.-]+/)
      .filter(Boolean)
      .slice(0, 2)
      .map((part) => part[0]?.toUpperCase())
      .join("");
  }

  function featureTitle() {
    if (view === "installed") return "已安装";
    if (view === "dev") return "开发者";
    if (view === "settings") return "设置";
    if (view === "clipboard") return "剪贴板";
    return query ? "搜索结果" : "插件市场";
  }

  refreshAll();
  tick().then(() => searchInput?.focus());
  window.addEventListener("vvitools-show-launcher", handleShowLauncher);
  window.addEventListener("vvitools-open-settings", handleOpenSettings);
  window.addEventListener("vvitools-open-clipboard", handleOpenClipboard);
  listen("show-launcher", handleShowLauncher)
    .then((unlisten) => {
      unlistenShowLauncher = unlisten;
    })
    .catch(() => {
      // 浏览器预览环境没有 Tauri 事件总线。
    });
  listen("open-settings", handleOpenSettings)
    .then((unlisten) => {
      unlistenOpenSettings = unlisten;
    })
    .catch(() => {
      // 浏览器预览环境没有 Tauri 事件总线。
    });
  listen("open-clipboard", handleOpenClipboard)
    .then((unlisten) => {
      unlistenOpenClipboard = unlisten;
    })
    .catch(() => {
      // 浏览器预览环境没有 Tauri 事件总线。
    });
  onDestroy(() => {
    window.removeEventListener("vvitools-show-launcher", handleShowLauncher);
    window.removeEventListener("vvitools-open-settings", handleOpenSettings);
    window.removeEventListener("vvitools-open-clipboard", handleOpenClipboard);
    unlistenShowLauncher?.();
    unlistenOpenSettings?.();
    unlistenOpenClipboard?.();
  });
</script>

{#if isClipboardWindow}
  <main class="rubick-window clipboard-window" data-tauri-drag-region>
    <header class="copycat-topbar">
      <strong>剪贴板</strong>
      <div class="copycat-search">
        <Search size={18} />
        <input
          bind:this={searchInput}
          bind:value={query}
          on:input={onInput}
          on:keydown={handleKeydown}
          placeholder="搜索"
          spellcheck="false"
        />
        {#if query}
          <button class="clear" on:click={() => ((query = ""), onInput())} type="button"><X size={15} /></button>
        {/if}
      </div>
      <div class="copycat-filters">
        <button class:active={clipboardFilter === "all"} on:click={() => setClipboardFilter("all")} type="button">
          {#if clipboardFilter === "all"}<Check size={15} />{/if}全部
        </button>
        <button class:active={clipboardFilter === "text"} on:click={() => setClipboardFilter("text")} type="button">
          {#if clipboardFilter === "text"}<Check size={15} />{/if}文本
        </button>
        <button class:active={clipboardFilter === "image"} on:click={() => setClipboardFilter("image")} type="button">
          {#if clipboardFilter === "image"}<Check size={15} />{/if}图片
        </button>
        <button class:active={clipboardFilter === "file"} on:click={() => setClipboardFilter("file")} type="button">
          {#if clipboardFilter === "file"}<Check size={15} />{/if}文件
        </button>
        <button class:active={clipboardFilter === "favorite"} on:click={() => setClipboardFilter("favorite")} type="button">
          {#if clipboardFilter === "favorite"}<Check size={15} />{/if}收藏
        </button>
      </div>
      <div class="copycat-hotkey">
        <span>↯</span>
        <strong>系统快捷键</strong>
        <kbd>⌥ V</kbd>
      </div>
    </header>
    <section class="copycat-board" bind:this={clipboardBoard}>
      {#if error}
        <div class="launcher-status error"><Terminal size={18} />{error}</div>
      {:else if filteredClipboardItems.length}
        {#each filteredClipboardItems as item, index}
          <article
            class:image-card={item.kind === "image"}
            class:file-card={item.kind === "file"}
            class:selected={selectedIndex === index}
            class="copycat-card text-card"
            data-selected={selectedIndex === index}
          >
            <button
              class="copycat-card-main"
              on:click={() => copyClipboardItem(item, true)}
              on:mouseenter={() => (selectedIndex = index)}
              type="button"
            >
              <span class="copycat-card-head">
                <span class="copycat-card-type">{clipboardItemLabel(item)}</span>
                <small>{formatClipboardTime(item.copied_at)}</small>
              </span>
              {#if item.kind === "image"}
                <div class="copycat-image-preview">
                  <img src={clipboardImageSrc(item)} alt={item.preview || "剪贴板图片"} />
                </div>
              {:else if item.kind === "file"}
                <p class="copycat-file-name">{item.preview || item.text}</p>
              {:else}
                <p>{item.preview || item.text}</p>
              {/if}
            </button>
            <footer>
              <span>
                {clipboardItemMeta(item)}
              </span>
              <div>
                <button
                  class:active={item.favorite}
                  aria-label={item.favorite ? "取消收藏" : "收藏"}
                  on:click={() => toggleClipboardFavorite(item)}
                  type="button"
                >
                  <Heart size={15} fill={item.favorite ? "currentColor" : "none"} />
                </button>
                <button aria-label="复制" on:click={() => copyClipboardItem(item, true)} type="button">
                  <Copy size={16} />
                </button>
                <button aria-label="删除" on:click={() => deleteClipboardItem(item)} type="button">
                  <Trash2 size={16} />
                </button>
              </div>
            </footer>
          </article>
        {/each}
      {:else}
        <div class="copycat-empty">
          <ClipboardList size={46} />
          <strong>{clipboardEmptyText()}</strong>
        </div>
      {/if}
    </section>
  </main>
{:else if view === "launcher"}
  <main class="rubick-window search-window" data-tauri-drag-region>
    <div class="main-search">
      <button class="rubick-logo" aria-label="打开插件市场" on:click={() => openFeature("finder")} type="button">
        <PackageSearch size={26} />
      </button>
      <input
        id="search"
        bind:this={searchInput}
        bind:value={query}
        on:input={onInput}
        on:keydown={handleKeydown}
        placeholder={loading ? "更新检测中..." : "你好，VviTools！请输入插件关键词"}
        spellcheck="false"
      />
    </div>

    {#if error}
      <div class="launcher-status error"><Terminal size={18} />{error}</div>
    {:else if result}
      <section class="result-panel">
        {#if result.type === "text"}
          <pre>{result.text}</pre>
        {:else if result.type === "markdown"}
          <pre>{result.markdown}</pre>
        {:else if result.type === "list"}
          {#each result.items as item}
            <button
              class:clickable={!!item.action}
              class="result-item"
              disabled={!item.action}
              on:click={() => executeAction(item.action)}
              type="button"
            >
              <strong>{item.title}</strong>
              {#if item.subtitle}<small>{item.subtitle}</small>{/if}
            </button>
          {/each}
        {:else}
          <div class="launcher-status error"><Terminal size={18} />{result.message}</div>
        {/if}
      </section>
    {:else if query}
      <section class="option-list">
        {#each launcherItems as command, index}
          <button
            class:active={selectedIndex === index}
            class="option-item"
            on:mouseenter={() => (selectedIndex = index)}
            on:click={() => run(command)}
            type="button"
          >
            <span class="app-avatar">{initials(command.plugin_name)}</span>
            <span>
              <strong>{commandDisplayName(command)}</strong>
              <small>{commandSubtitle(command)}</small>
            </span>
          </button>
        {/each}
        {#if launcherItems.length === 0}
          <div class="launcher-status">暂无匹配插件，点击左侧图标进入插件市场。</div>
        {/if}
      </section>
    {:else}
      <section class="history-plugins">
        <div class="launcher-section">
          <div class="section-title">最近使用</div>
          <div class="history-grid">
            {#each recentItems as command, index}
              <button
                class:active={selectedIndex === index}
                class="history-item"
                on:mouseenter={() => (selectedIndex = index)}
                on:click={() => run(command)}
                type="button"
              >
                <span class="app-avatar">{initials(command.plugin_name)}</span>
                <span>{launcherCardName(command)}</span>
              </button>
            {/each}
            {#if recentItems.length === 0}
              <div class="empty-section">安装或运行插件后会显示在这里</div>
            {/if}
          </div>
        </div>

        <div class="launcher-section">
          <div class="section-title">推荐</div>
          <div class="history-grid">
            <button class="history-item market-entry" on:click={() => openFeature("finder")} type="button">
              <span class="app-avatar"><PackageSearch size={20} /></span>
              <span>插件市场</span>
            </button>
            <button class="history-item market-entry" on:click={() => openFeature("installed")} type="button">
              <span class="app-avatar"><Heart size={20} /></span>
              <span>已安装</span>
            </button>
          </div>
        </div>
      </section>
    {/if}
  </main>
{:else}
  <main class="rubick-window feature-window">
    <aside class="left-menu" data-tauri-drag-region>
      <button class="back-mini" on:click={backToLauncher} type="button">
        <ArrowLeft size={16} />
        搜索
      </button>
      <nav>
        {#each categories as item}
          <button class:active={view === item.view} on:click={() => openFeature(item.view)} type="button">
            {#if item.view === "finder"}<Star size={16} />{/if}
            {#if item.view === "clipboard"}<ClipboardList size={16} />{/if}
            {item.label}
          </button>
        {/each}
      </nav>
      <div class="user-menu">
        <button class:active={view === "installed"} on:click={() => (view = "installed")} type="button">
          <Heart size={16} />
          已安装
        </button>
        <button class:active={view === "dev"} on:click={() => (view = "dev")} type="button">
          <Bug size={16} />
          开发者
        </button>
        <button class:active={view === "settings"} on:click={openSettings} type="button">
          <Settings size={16} />
          设置
        </button>
      </div>
    </aside>

    <section class="feature-container">
      <div class="feature-search">
        <Search size={16} />
        <input
          bind:this={searchInput}
          bind:value={query}
          on:input={onInput}
          on:keydown={handleKeydown}
          placeholder={view === "clipboard" ? "搜索剪贴板历史" : view === "installed" ? "搜索已安装插件" : "搜索插件"}
        />
        {#if query}
          <button class="clear" on:click={() => ((query = ""), onInput())} type="button"><X size={15} /></button>
        {/if}
      </div>

      {#if view === "finder"}
        <div class="view-title">{featureTitle()}</div>
        <div class="market-layout">
          <div class="market-list">
            {#each filteredMarket as item}
              <button
                class:active={marketDetail?.id === item.id}
                class="plugin-card"
                on:click={() => (selectedMarketId = item.id)}
                type="button"
              >
                <span class="plugin-icon">{initials(item.name)}</span>
                <span class="plugin-copy">
                  <strong>{item.name}</strong>
                  <small>{item.description}</small>
                </span>
                {#if installedIds.has(item.id)}
                  <Check size={17} />
                {:else}
                  <DownloadCloud size={18} />
                {/if}
              </button>
            {/each}
          </div>

          <article class="plugin-detail">
            {#if marketDetail}
              <div class="detail-head">
                <span class="plugin-icon large">
                  {#if isClipboardApp}<ClipboardList size={26} />{:else}{initials(marketDetail.name)}{/if}
                </span>
                <div>
                  <h2>{marketDetail.name}</h2>
                  <p>{marketDetail.description}</p>
                </div>
              </div>
              <div class="detail-meta">
                <span>版本 {marketDetail.version}</span>
                <span>权限 {marketDetail.permissions.join("、") || "无"}</span>
                <span>{marketDetail.bundled ? "捆绑内置" : "JSON-RPC · zip"}</span>
                <span>{marketDetail.runtime || "node"} · {marketDetail.entry || "main.js"}</span>
              </div>
              <div class="detail-home">
                {#if isClipboardApp}
                  <h3>剪切板</h3>
                  <p>该工具是随应用分发的捆绑内置插件，插件目录声明入口、图标、权限和宿主桥接；高权限剪贴板能力由 Rust 宿主执行。使用 Alt + V 会打开独立底部剪贴板窗口。</p>
                  <div class="clipboard-product-preview">
                    <div class="preview-toolbar">
                      <span>Clipboard</span>
                      <em>搜索</em>
                      <ClipboardList size={24} />
                    </div>
                    <div class="preview-tabs">
                      <span>全部</span>
                      <span>文本</span>
                      <span>文件</span>
                      <span>图片</span>
                    </div>
                    <div class="preview-row">
                      <strong>OpenVibeCoding</strong>
                      <small>刚刚 · 14 字符</small>
                    </div>
                    <div class="preview-row">
                      <strong>https://github.com/TencentCloudBase/OpenVibeCoding</strong>
                      <small>刚刚 · 50 字符</small>
                    </div>
                  </div>
                {:else}
                  <h3>插件主页</h3>
                  <p>这是静态插件市场条目。安装前会展示 manifest 权限，安装后插件命令会回到主搜索框。</p>
                {/if}
              </div>
              <button
                class="primary"
                disabled={isClipboardApp || installedIds.has(marketDetail.id) || loading}
                on:click={() => install(marketDetail)}
                type="button"
              >
                {#if loading}<Loader2 class="spin" size={16} />{/if}
                {isClipboardApp ? "已内置" : installedIds.has(marketDetail.id) ? "已安装" : "安装"}
              </button>
            {/if}
          </article>
        </div>
      {:else if view === "installed"}
        <div class="view-title">已安装</div>
        <div class="installed-layout">
          <div class="installed-list">
            {#each filteredPlugins as plugin}
              <button
                class:active={pluginDetail?.id === plugin.id}
                class="installed-item"
                on:click={() => (selectedPluginId = plugin.id)}
                type="button"
              >
                <span class="plugin-icon">{initials(plugin.name)}</span>
                <span>
                  <strong>{plugin.name}</strong>
                  <small>{plugin.description}</small>
                </span>
              </button>
            {/each}
          </div>
          <article class="plugin-detail">
            {#if pluginDetail}
              <div class="detail-head">
                <span class="plugin-icon large">{initials(pluginDetail.name)}</span>
                <div>
                  <h2>{pluginDetail.name}<em>{pluginDetail.version}</em></h2>
                  <p>{pluginDetail.description}</p>
                </div>
              </div>
              <div class="detail-meta">
                <span>{pluginDetail.commands} 个命令</span>
                <span>{pluginDetail.bundled ? "捆绑内置" : "本地安装"}</span>
                <span>{pluginDetail.runtime} · {pluginDetail.entry}</span>
                <span>权限 {pluginDetail.permissions.join("、") || "无"}</span>
              </div>
              <div class="feature-tags">
                {#each commands.filter((command) => command.plugin_id === pluginDetail?.id) as command}
                  <button on:click={() => run(command)} type="button"><Play size={13} />{command.keyword}</button>
                {/each}
              </div>
            {:else}
              <div class="empty-installed">
                <Box size={46} />
                <strong>暂无任何插件</strong>
                <button class="primary" on:click={() => (view = "finder")} type="button">去插件市场看看吧</button>
              </div>
            {/if}
          </article>
        </div>
      {:else if view === "dev"}
        <div class="view-title">开发者</div>
        <section class="developer-page">
          <Wrench size={34} />
          <h2>本地插件调试</h2>
          <p>第一版使用 `plugin.json` 声明插件，支持 shell 和 node 运行时，入口脚本通过 `VVITOOLS_RPC_INPUT` 接收 JSON-RPC 请求。</p>
          <pre>{`{
  "runtime": "node",
  "entry": "main.js",
  "permissions": ["clipboard"]
}`}</pre>
        </section>
      {:else}
        <div class="view-title">设置</div>
        <section class="developer-page settings-page">
          <Settings size={34} />
          <h2>应用设置</h2>
          <div class="settings-list">
            <div class="settings-row">
              <span>
                <strong>状态栏运行</strong>
                <small>隐藏 Dock 图标，保留状态栏入口和快捷键呼出。</small>
              </span>
              <em>已开启</em>
            </div>
            <div class="settings-row">
              <span>
                <strong>全局快捷键</strong>
                <small>使用 Alt + Space 显示或隐藏搜索，Alt + V 从底部打开剪贴板。</small>
              </span>
              <em>Alt Space / Alt V</em>
            </div>
            <div class="settings-row">
              <span>
                <strong>点击外部关闭</strong>
                <small>窗口失去焦点后自动隐藏。</small>
              </span>
              <em>已开启</em>
            </div>
          </div>
        </section>
      {/if}
    </section>
  </main>
{/if}
