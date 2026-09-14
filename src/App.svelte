<script lang="ts">
  import { convertFileSrc, invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { onDestroy, tick } from "svelte";
  import {
    ArrowLeft,
    ArrowRight,
    Box,
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
    ShieldCheck,
    Star,
    Terminal,
    Trash2,
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
    categories: PluginCategoryKey[];
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
    categories: PluginCategoryKey[];
    permissions: string[];
  };

  type UpdateInfo = {
    current_version: string;
    latest_version: string;
    has_update: boolean;
    release_url: string;
    message: string;
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

  type AccessibilityPermissionStatus = {
    granted: boolean;
    supported: boolean;
    app_path: string;
    message: string;
  };

  type ClipboardPasteResult = {
    copied: boolean;
    paste_requested: boolean;
    needs_accessibility_permission: boolean;
    message: string;
  };

  type ClipboardFilter = "all" | "text" | "image" | "file" | "favorite";

  type View = "launcher" | "finder" | "clipboard" | "installed" | "settings" | "permission";
  type PluginCategoryKey = "efficiency" | "search" | "image" | "developer" | "system";
  type PluginCategory = "explore" | "developer" | "custom";

  const CLIPBOARD_APP_ID = "system-clipboard";
  const params = new URLSearchParams(window.location.search);
  const isClipboardWindow = params.get("window") === "clipboard";
  const pluginCategories: Array<{ key: PluginCategory; label: string }> = [
    { key: "explore", label: "探索" },
    { key: "developer", label: "开发" },
    { key: "custom", label: "自定义" },
  ];
  const categoryLabels: Record<PluginCategoryKey, string> = {
    efficiency: "效率",
    search: "搜索",
    image: "图像",
    developer: "开发",
    system: "系统",
  };

  let view: View = isClipboardWindow ? "clipboard" : params.get("view") === "finder" ? "finder" : "launcher";
  let selectedPluginCategory: PluginCategory = "explore";
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
  let showCustomImportDialog = false;
  let customImportMode: "local" | "remote" = "local";
  let customRemoteUrl = "";
  let customImportStatus = "";
  let marketDetailOpen = false;
  let autostartEnabled = false;
  let autostartLoading = false;
  let dockVisibleEnabled = false;
  let dockVisibleLoading = false;
  let accessibilityPermissionGranted = false;
  let accessibilityPermissionSupported = false;
  let accessibilityPermissionLoading = false;
  let showAccessibilityDialog = false;
  let updateLoading = false;
  let updateStatus = "";
  let updateReleaseUrl = "";
  let searchInput: HTMLInputElement;
  let clipboardBoard: HTMLElement;
  let customFileInput: HTMLInputElement;
  let unlistenShowLauncher: (() => void) | undefined;
  let unlistenOpenSettings: (() => void) | undefined;
  let unlistenOpenClipboard: (() => void) | undefined;
  let unlistenOpenAccessibilityPermission: (() => void) | undefined;

  $: installedIds = new Set(plugins.map((plugin) => plugin.id));
  $: customMarket = plugins.filter((plugin) => !plugin.bundled).map(pluginViewToMarketEntry);
  $: marketplaceItems = selectedPluginCategory === "custom" ? customMarket : market;
  $: filteredMarket = marketplaceItems.filter(
    (item) => matchText(item, query) && matchesPluginCategory(item, selectedPluginCategory)
  );
  $: filteredPlugins = plugins.filter((plugin) => matchText(plugin, query));
  $: installedMarket = filteredPlugins.map(pluginViewToMarketEntry);
  $: filteredClipboardItems = clipboardItems.filter(
    (item) =>
      (clipboardFilter === "favorite" ? item.favorite : clipboardFilter === "all" || item.kind === clipboardFilter) &&
      matchClipboardText(item, query)
  );
  $: marketDetail = filteredMarket.find((item) => item.id === selectedMarketId) ?? filteredMarket[0];
  $: installedDetail = installedMarket.find((item) => item.id === selectedPluginId) ?? installedMarket[0];
  $: activeDetail = view === "installed" ? installedDetail : marketDetail;
  $: selectedCategoryLabel =
    pluginCategories.find((category) => category.key === selectedPluginCategory)?.label ?? "探索";
  $: launcherItems = query ? commands : [];
  $: recentItems = commands.slice(0, 8);
  $: visibleCount =
    view === "clipboard"
      ? filteredClipboardItems.length
      : view === "launcher"
        ? Math.max(launcherItems.length, recentItems.length)
        : view === "installed"
          ? installedMarket.length
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

  async function loadAutostartStatus() {
    try {
      autostartEnabled = await invoke<boolean>("is_autostart_enabled");
    } catch (err) {
      error = String(err);
    }
  }

  async function loadDockVisibleStatus() {
    try {
      dockVisibleEnabled = await invoke<boolean>("is_dock_visible_enabled");
    } catch (err) {
      error = String(err);
    }
  }

  async function loadAccessibilityPermissionStatus() {
    try {
      const status = await invoke<AccessibilityPermissionStatus>("accessibility_permission_status");
      accessibilityPermissionGranted = status.granted;
      accessibilityPermissionSupported = status.supported;
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
      await openClipboardWindow();
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

  async function openClipboardWindow() {
    error = "";
    try {
      await invoke("open_clipboard_window");
    } catch (err) {
      error = String(err);
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

  function openCustomImportDialog() {
    customImportStatus = "";
    customRemoteUrl = "";
    customImportMode = "local";
    showCustomImportDialog = true;
  }

  function closeCustomImportDialog() {
    showCustomImportDialog = false;
    customImportStatus = "";
  }

  async function importCustomPluginContent(content: string) {
    customImportStatus = "";
    loading = true;
    try {
      await invoke("import_custom_plugin_config", { request: { content } });
      await refreshAll();
      selectedPluginCategory = "custom";
      showCustomImportDialog = false;
    } catch (err) {
      customImportStatus = String(err);
    } finally {
      loading = false;
    }
  }

  async function importCustomPluginFromUrl() {
    customImportStatus = "";
    if (!customRemoteUrl.trim()) {
      customImportStatus = "请输入远程 JSON 地址";
      return;
    }
    loading = true;
    try {
      await invoke("import_custom_plugin_config", { request: { url: customRemoteUrl.trim() } });
      await refreshAll();
      selectedPluginCategory = "custom";
      showCustomImportDialog = false;
    } catch (err) {
      customImportStatus = String(err);
    } finally {
      loading = false;
    }
  }

  function chooseCustomPluginFile() {
    customFileInput?.click();
  }

  async function handleCustomFileSelected(event: Event) {
    const input = event.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    input.value = "";
    if (!file) return;
    await importCustomPluginContent(await file.text());
  }

  async function confirmCustomPluginImport() {
    if (customImportMode === "remote") {
      await importCustomPluginFromUrl();
      return;
    }
    chooseCustomPluginFile();
  }

  async function deleteSelectedCustomPlugin() {
    const target = filteredMarket.find((item) => item.id === selectedMarketId) ?? filteredMarket[0];
    if (!target) {
      error = "请选择要删除的自定义插件";
      return;
    }
    if (target.bundled) {
      error = "内置插件不能删除";
      return;
    }
    const approved = window.confirm(`删除自定义插件 ${target.name}？`);
    if (!approved) return;
    error = "";
    try {
      await invoke("delete_custom_plugin", { request: { plugin_id: target.id } });
      selectedMarketId = "";
      selectedPluginId = "";
      await refreshAll();
      selectedPluginCategory = "custom";
    } catch (err) {
      error = String(err);
    }
  }

  async function activateMarketItem(item: MarketplaceEntry) {
    selectedMarketId = item.id;
    marketDetailOpen = true;
  }

  async function activateInstalledItem(item: MarketplaceEntry) {
    selectedPluginId = item.id;
    marketDetailOpen = true;
  }

  function closeMarketDetail() {
    marketDetailOpen = false;
  }

  async function openFeature(nextView: View = "finder") {
    view = nextView;
    query = "";
    result = null;
    resultPluginId = "";
    error = "";
    clipboardStatus = "";
    if (nextView === "clipboard") await loadClipboardHistory();
    await invoke("set_launcher_view", { view: "feature" }).catch(() => {
      // 浏览器预览环境没有 Tauri 窗口。
    });
    await tick();
    resetViewport();
    searchInput?.focus();
  }

  async function openPluginCategory(category: PluginCategory) {
    selectedPluginCategory = category;
    marketDetailOpen = false;
    await openFeature("finder");
  }

  async function openInstalled() {
    marketDetailOpen = false;
    await openFeature("installed");
  }

  async function backToLauncher() {
    await invoke("set_launcher_view", { view: "launcher" }).catch(() => {
      // 浏览器预览环境没有 Tauri 窗口。
    });
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
    await loadAutostartStatus();
    await loadDockVisibleStatus();
    await loadAccessibilityPermissionStatus();
    await tick();
    resetViewport();
    searchInput?.focus();
  }

  async function toggleAutostart() {
    if (autostartLoading) return;
    autostartLoading = true;
    error = "";
    try {
      autostartEnabled = await invoke<boolean>("set_autostart_enabled", {
        request: { enabled: !autostartEnabled },
      });
    } catch (err) {
      error = String(err);
      await loadAutostartStatus();
    } finally {
      autostartLoading = false;
    }
  }

  async function toggleDockVisible() {
    if (dockVisibleLoading) return;
    dockVisibleLoading = true;
    error = "";
    try {
      dockVisibleEnabled = await invoke<boolean>("set_dock_visible_enabled", {
        request: { enabled: !dockVisibleEnabled },
      });
    } catch (err) {
      error = String(err);
      await loadDockVisibleStatus();
    } finally {
      dockVisibleLoading = false;
    }
  }

  async function requestAccessibilityPermission() {
    if (accessibilityPermissionLoading) return;
    accessibilityPermissionLoading = true;
    error = "";
    await closeAccessibilityDialog();
    try {
      const status = await invoke<AccessibilityPermissionStatus>("request_accessibility_permission");
      accessibilityPermissionGranted = status.granted;
      accessibilityPermissionSupported = status.supported;
    } catch (err) {
      error = String(err);
    } finally {
      accessibilityPermissionLoading = false;
    }
  }

  async function openAccessibilityDialog() {
    if (isClipboardWindow) {
      await invoke("open_accessibility_permission_window").catch((err) => {
        error = String(err);
      });
      return;
    }
    showAccessibilityDialog = true;
  }

  async function closeAccessibilityDialog() {
    showAccessibilityDialog = false;
    if (view === "permission") {
      view = "launcher";
      await hideLauncher();
    }
  }

  async function checkUpdate() {
    if (updateLoading) return;
    updateLoading = true;
    updateStatus = "";
    updateReleaseUrl = "";
    error = "";
    try {
      const info = await invoke<UpdateInfo>("check_for_update");
      updateReleaseUrl = info.release_url;
      updateStatus = info.has_update
        ? `发现新版本 ${info.latest_version}，当前版本 ${info.current_version}`
        : `${info.message}，当前版本 ${info.current_version}`;
    } catch (err) {
      updateStatus = "";
      error = String(err);
    } finally {
      updateLoading = false;
    }
  }

  async function openUpdateRelease() {
    if (!updateReleaseUrl) return;
    try {
      await invoke("open_external", { url: updateReleaseUrl });
    } catch (err) {
      error = String(err);
    }
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
      if (closeAfterCopy) {
        const pasteResult = await invoke<ClipboardPasteResult>("paste_clipboard_item", { request: { id: item.id } });
        if (pasteResult.needs_accessibility_permission) {
          accessibilityPermissionGranted = false;
          accessibilityPermissionSupported = true;
          await openAccessibilityDialog();
        }
      } else {
        await invoke("copy_clipboard_item", { request: { id: item.id } });
        clipboardStatus = "已复制";
        await loadClipboardHistory();
      }
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

  async function handleOpenAccessibilityPermission() {
    view = "permission";
    query = "";
    result = null;
    resultPluginId = "";
    error = "";
    await loadAccessibilityPermissionStatus();
    showAccessibilityDialog = true;
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
    if (view === "finder") {
      if (marketDetail) {
        selectedMarketId = marketDetail.id;
        marketDetailOpen = true;
      }
      return;
    }
    if (view !== "launcher") {
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
    return [
      item.id,
      item.name,
      item.description,
      item.version,
      item.runtime,
      item.entry,
      item.categories.join(" "),
      item.permissions.join(" "),
    ]
      .join(" ")
      .toLowerCase()
      .includes(q);
  }

  function pluginViewToMarketEntry(plugin: PluginView): MarketplaceEntry {
    return {
      id: plugin.id,
      name: plugin.name,
      version: plugin.version,
      description: plugin.description,
      icon: plugin.icon,
      runtime: plugin.runtime,
      entry: plugin.entry,
      bundled: plugin.bundled,
      download_url: "",
      sha256: null,
      categories: plugin.categories,
      permissions: plugin.permissions,
    };
  }

  function matchClipboardText(item: ClipboardItem, value: string) {
    const q = value.trim().toLowerCase();
    if (!q) return true;
    return [item.preview, item.text, item.copied_at].join(" ").toLowerCase().includes(q);
  }

  function matchesPluginCategory(item: MarketplaceEntry, category: PluginCategory) {
    if (category === "explore") return true;
    if (category === "custom") return !item.bundled;
    return item.categories.includes(category);
  }

  function categoryLabel(category: PluginCategoryKey) {
    return categoryLabels[category];
  }

  function permissionLabel(permission: string) {
    const labels: Record<string, string> = {
      "clipboard:read": "读取剪贴板内容",
      "clipboard:write": "写入剪贴板内容",
      "accessibility:paste": "粘贴到当前输入框",
      shell: "运行本地命令",
      network: "访问网络",
      filesystem: "访问本地文件",
    };
    return labels[permission] ?? "插件运行所需权限";
  }

  function categoryEmptyText() {
    if (query) return "没有匹配插件";
    if (selectedPluginCategory === "developer") return "暂无开发工具";
    if (selectedPluginCategory === "custom") return "还没有导入自定义插件";
    return "这里还没有插件";
  }

  function isClipboardEntry(item: MarketplaceEntry) {
    return item.id === CLIPBOARD_APP_ID;
  }

  function customImportHelpText() {
    return "支持导入 VviTools plugin.json，或包含 plugins 数组的 JSON 配置文件。自定义插件支持 node / shell 运行时。";
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
    if (view === "settings") return "设置";
    if (view === "clipboard") return "剪贴板";
    return selectedCategoryLabel;
  }

  refreshAll();
  tick().then(() => searchInput?.focus());
  window.addEventListener("vvitools-show-launcher", handleShowLauncher);
  window.addEventListener("vvitools-open-settings", handleOpenSettings);
  window.addEventListener("vvitools-open-clipboard", handleOpenClipboard);
  window.addEventListener("vvitools-open-accessibility-permission", handleOpenAccessibilityPermission);
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
  listen("open-accessibility-permission", handleOpenAccessibilityPermission)
    .then((unlisten) => {
      unlistenOpenAccessibilityPermission = unlisten;
    })
    .catch(() => {
      // 浏览器预览环境没有 Tauri 事件总线。
    });
  onDestroy(() => {
    window.removeEventListener("vvitools-show-launcher", handleShowLauncher);
    window.removeEventListener("vvitools-open-settings", handleOpenSettings);
    window.removeEventListener("vvitools-open-clipboard", handleOpenClipboard);
    window.removeEventListener("vvitools-open-accessibility-permission", handleOpenAccessibilityPermission);
    unlistenShowLauncher?.();
    unlistenOpenSettings?.();
    unlistenOpenClipboard?.();
    unlistenOpenAccessibilityPermission?.();
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
      {:else}
        {#if clipboardStatus}
          <div
            class:warning={showAccessibilityDialog || (!accessibilityPermissionGranted && accessibilityPermissionSupported)}
            class="launcher-status compact"
          >
            <Terminal size={18} />{clipboardStatus}
            {#if !accessibilityPermissionGranted && accessibilityPermissionSupported}
              <button class="inline-link" on:click={() => openAccessibilityDialog()} type="button">查看授权</button>
            {/if}
          </div>
        {/if}
        {#if filteredClipboardItems.length}
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
      {/if}
    </section>
  </main>
{:else if view === "permission"}
  <main class="rubick-window permission-window" data-tauri-drag-region>
    <div class="permission-dialog standalone" role="dialog" aria-modal="true" aria-labelledby="permission-title">
      <header>
        <div class="permission-heading">
          <span class="permission-icon"><ShieldCheck size={21} /></span>
          <span>
            <small>自动粘贴</small>
            <h2 id="permission-title">开启辅助功能权限</h2>
          </span>
        </div>
        <button class="permission-close" aria-label="关闭" on:click={() => closeAccessibilityDialog()} type="button">
          <X size={18} />
        </button>
      </header>
      <div class="permission-body">
        <p>接下来 macOS 会显示系统授权提示。开启后，选择剪贴板记录即可自动粘贴回原输入框。</p>
        <div class="permission-flow">
          <div class="current">
            <span>1</span>
            <div>
              <strong>确认授权说明</strong>
              <small>VviTools 仅使用此权限完成自动粘贴。</small>
            </div>
          </div>
          <div>
            <span>2</span>
            <div>
              <strong>在系统中允许</strong>
              <small>按 macOS 提示开启 VviTools。</small>
            </div>
          </div>
        </div>
      </div>
      <footer>
        <button class="permission-cancel" on:click={() => closeAccessibilityDialog()} type="button">暂不开启</button>
        <button
          class="permission-primary"
          disabled={accessibilityPermissionLoading}
          on:click={requestAccessibilityPermission}
          type="button"
        >
          继续授权
          <ArrowRight size={16} />
        </button>
      </footer>
    </div>
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
              <div class="empty-section">输入关键词搜索插件，或点击左上角进入插件市场</div>
            {/if}
          </div>
        </div>
      </section>
    {/if}
  </main>
{:else}
  <main class="rubick-window feature-window">
    <input
      bind:this={customFileInput}
      accept="application/json,.json"
      class="hidden-file-input"
      on:change={handleCustomFileSelected}
      type="file"
    />
    <aside class="left-menu" data-tauri-drag-region>
      <button class="back-mini" on:click={backToLauncher} type="button">
        <ArrowLeft size={16} />
        搜索
      </button>
      <div class="feature-search">
        <Search size={16} />
        <input
          bind:this={searchInput}
          bind:value={query}
          on:input={onInput}
          on:keydown={handleKeydown}
          placeholder={view === "installed" ? "搜索已安装" : "搜索插件"}
        />
        {#if query}
          <button class="clear" on:click={() => ((query = ""), onInput())} type="button"><X size={15} /></button>
        {/if}
      </div>
      <nav>
        {#each pluginCategories as item}
          <button
            class:active={view === "finder" && selectedPluginCategory === item.key}
            on:click={() => openPluginCategory(item.key)}
            type="button"
          >
            {#if item.key === "explore"}<Star size={16} />{/if}
            {#if item.key === "developer"}<Terminal size={16} />{/if}
            {#if item.key === "custom"}<PackageSearch size={16} />{/if}
            {item.label}
          </button>
        {/each}
      </nav>
      <div class="user-menu">
        <button class:active={view === "installed"} on:click={openInstalled} type="button">
          <Heart size={16} />
          已安装
        </button>
        <button class:active={view === "settings"} on:click={openSettings} type="button">
          <Settings size={16} />
          设置
        </button>
      </div>
    </aside>

    <section class="feature-container">
      {#if (view === "finder" || view === "installed") && marketDetailOpen && activeDetail}
          <article class="market-detail-page">
            <button class="detail-back" on:click={closeMarketDetail} type="button" aria-label="返回插件列表">
              <ArrowLeft size={18} />
              返回
            </button>
            <section class="market-detail-hero">
              <span class="plugin-icon market-detail-icon">
                {#if isClipboardEntry(activeDetail)}<ClipboardList size={27} />{:else}{initials(activeDetail.name)}{/if}
              </span>
              <div class="market-detail-title">
                <div class="market-detail-name">
                  <h2>{activeDetail.name}</h2>
                  <span>{activeDetail.bundled ? "内置" : installedIds.has(activeDetail.id) ? "已安装" : "可获取"}</span>
                </div>
                <p>{activeDetail.description}</p>
                <div class="market-detail-summary">
                  <span>版本 {activeDetail.version}</span>
                  <span>{activeDetail.categories.map(categoryLabel).join("、") || "通用工具"}</span>
                  <span>{activeDetail.bundled ? "随 VviTools 提供" : "用户安装"}</span>
                </div>
              </div>
              <div class="market-detail-actions">
                {#if isClipboardEntry(activeDetail)}
                  <button class="primary detail-action" on:click={openClipboardWindow} type="button">
                    <Play size={15} />
                    打开
                  </button>
                {:else if installedIds.has(activeDetail.id)}
                  <button class="primary detail-action" disabled type="button"><Check size={15} />已安装</button>
                {:else}
                  <button class="primary detail-action" disabled={loading} on:click={() => install(activeDetail)} type="button">
                    {#if loading}<Loader2 class="spin" size={15} />{:else}<DownloadCloud size={15} />{/if}
                    获取
                  </button>
                {/if}
                {#if view === "finder" && selectedPluginCategory === "custom" && !activeDetail.bundled}
                  <button class="detail-delete" on:click={deleteSelectedCustomPlugin} type="button">
                    <Trash2 size={15} />
                    删除
                  </button>
                {/if}
              </div>
            </section>
            <section class="market-detail-section">
              <header>
                <h3>功能</h3>
                <p>从搜索窗口或对应快捷入口直接使用。</p>
              </header>
              {#if isClipboardEntry(activeDetail)}
                <div class="capability-list">
                  <div><span class="capability-icon"><ClipboardList size={17} /></span><span><strong>剪贴板历史</strong><small>自动保存最近复制的文本、图片和文件。</small></span></div>
                  <div><span class="capability-icon"><Search size={17} /></span><span><strong>快速查找</strong><small>按内容搜索，并按类型或收藏筛选记录。</small></span></div>
                  <div><span class="capability-icon"><Star size={17} /></span><span><strong>收藏常用内容</strong><small>保留经常使用的片段，随时再次调用。</small></span></div>
                  <div><span class="capability-icon"><Play size={17} /></span><span><strong>选择并粘贴</strong><small>使用 Alt + V 呼出，选中后粘贴回原输入框。</small></span></div>
                </div>
              {:else}
                <div class="capability-list">
                  {#each commands.filter((command) => command.plugin_id === activeDetail?.id) as command}
                    <div><span class="capability-icon"><Play size={17} /></span><span><strong>{command.title}</strong><small>可在 VviTools 搜索中运行。</small></span></div>
                  {:else}
                    <div><span class="capability-icon"><Box size={17} /></span><span><strong>插件工具</strong><small>{activeDetail.description}</small></span></div>
                  {/each}
                </div>
              {/if}
            </section>
            <section class="market-detail-section permission-section">
              <header>
                <h3>权限</h3>
                <p>仅在使用相关功能时调用。</p>
              </header>
              <div class="permission-list">
                {#each activeDetail.permissions as permission}
                  <span><ShieldCheck size={16} />{permissionLabel(permission)}</span>
                {:else}
                  <span><ShieldCheck size={16} />无需额外权限</span>
                {/each}
              </div>
            </section>
          </article>
      {:else if view === "finder"}
        <div class="market-page">
          <div class="market-head">
            <div>
              <div class="view-title">{featureTitle()}</div>
              <small>
                {#if query}
                  找到 {filteredMarket.length} 个结果
                {:else if selectedPluginCategory === "explore"}
                  发现适合你的工具
                {:else if selectedPluginCategory === "developer"}
                  面向开发工作的实用工具
                {:else}
                  管理从本地或远程导入的插件
                {/if}
              </small>
            </div>
            {#if selectedPluginCategory === "custom"}
              <div class="custom-actions">
                <button class="import-button" on:click={openCustomImportDialog} type="button">
                  <DownloadCloud size={17} />
                  导入插件
                </button>
              </div>
            {/if}
          </div>
          <div class="market-plugin-grid">
            {#each filteredMarket as item}
              <button
                class:active={marketDetail?.id === item.id}
                class="market-plugin-card"
                on:click={() => activateMarketItem(item)}
                type="button"
              >
                <span class="plugin-icon large">
                  {#if isClipboardEntry(item)}<ClipboardList size={24} />{:else}{initials(item.name)}{/if}
                </span>
                <span>
                  <strong>{item.name}</strong>
                  <small>{item.description}</small>
                </span>
                <em>{item.bundled ? "内置" : installedIds.has(item.id) ? "已安装" : "插件"}</em>
                {#if isClipboardEntry(item) || installedIds.has(item.id)}
                  <Check size={18} />
                {:else}
                  <DownloadCloud size={19} />
                {/if}
              </button>
            {/each}
            {#if filteredMarket.length === 0}
              <div class="market-empty">
                {#if selectedPluginCategory === "custom"}
                  <PackageSearch size={28} />
                {:else}
                  <Box size={28} />
                {/if}
                <strong>{categoryEmptyText()}</strong>
                <small>
                  {selectedPluginCategory === "custom"
                    ? "可导入 plugin.json，添加自己的 Node 或 Shell 工具。"
                    : "新的工具会在这里出现。"}
                </small>
                {#if selectedPluginCategory === "custom"}
                  <button class="import-button" on:click={openCustomImportDialog} type="button">
                    <DownloadCloud size={16} />
                    导入插件
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {:else if view === "installed"}
        <div class="market-page">
          <div class="market-head">
            <div>
              <div class="view-title">已安装</div>
              <small>{query ? `找到 ${installedMarket.length} 个结果` : `${installedMarket.length} 个插件，可直接打开使用`}</small>
            </div>
          </div>
          <div class="market-plugin-grid">
            {#each installedMarket as item}
              <button
                class:active={installedDetail?.id === item.id}
                class="market-plugin-card"
                on:click={() => activateInstalledItem(item)}
                type="button"
              >
                <span class="plugin-icon large">
                  {#if isClipboardEntry(item)}<ClipboardList size={24} />{:else}{initials(item.name)}{/if}
                </span>
                <span>
                  <strong>{item.name}</strong>
                  <small>{item.description}</small>
                </span>
                <em>{item.bundled ? "内置" : "已安装"}</em>
                <Check size={18} />
              </button>
            {/each}
            {#if installedMarket.length === 0}
              <div class="market-empty">
                <Box size={46} />
                <strong>{query ? "没有匹配插件" : "还没有安装插件"}</strong>
                <small>从探索页获取插件后，会统一显示在这里。</small>
                {#if !query}
                  <button class="import-button" on:click={() => openPluginCategory("explore")} type="button">
                    <Star size={16} />
                    去探索
                  </button>
                {/if}
              </div>
            {/if}
          </div>
        </div>
      {:else}
        <div class="page-heading settings-heading">
          <div class="view-title">设置</div>
          <small>调整 VviTools 的启动方式与快捷操作</small>
        </div>
        <section class="settings-panel settings-page">
          <div class="settings-group">
            <h2>启动与显示</h2>
            <div class="settings-list">
              <div class="settings-row">
                <span>
                  <strong>开机启动</strong>
                  <small>登录系统后自动启动 VviTools，保留状态栏入口和快捷键。</small>
                </span>
                <button
                  class="settings-switch"
                  class:enabled={autostartEnabled}
                  disabled={autostartLoading}
                  aria-pressed={autostartEnabled}
                  on:click={toggleAutostart}
                  type="button"
                >
                  {#if autostartLoading}
                    <Loader2 class="spin" size={14} />
                  {:else}
                    <span class="sr-only">{autostartEnabled ? "已开启" : "已关闭"}</span>
                    <span class="settings-switch-thumb"></span>
                  {/if}
                </button>
              </div>
              <div class="settings-row">
                <span>
                  <strong>Dock 栏显示</strong>
                  <small>开启后在 Dock 中显示图标；关闭后仅保留状态栏入口和快捷键呼出。</small>
                </span>
                <button
                  class="settings-switch"
                  class:enabled={dockVisibleEnabled}
                  disabled={dockVisibleLoading}
                  aria-pressed={dockVisibleEnabled}
                  on:click={toggleDockVisible}
                  type="button"
                >
                  {#if dockVisibleLoading}
                    <Loader2 class="spin" size={14} />
                  {:else}
                    <span class="sr-only">{dockVisibleEnabled ? "已开启" : "已关闭"}</span>
                    <span class="settings-switch-thumb"></span>
                  {/if}
                </button>
              </div>
            </div>
          </div>
          <div class="settings-group">
            <h2>快捷操作</h2>
            <div class="settings-list">
              <div class="settings-row">
                <span>
                  <strong>自动粘贴权限</strong>
                  <small>用于选择剪贴板记录后自动粘贴到原输入框。</small>
                </span>
                <div class="settings-actions">
                  <em class:enabled={accessibilityPermissionGranted}>
                    {accessibilityPermissionGranted ? "已开启" : accessibilityPermissionSupported ? "未开启" : "无需授权"}
                  </em>
                  {#if !accessibilityPermissionGranted && accessibilityPermissionSupported}
                    <button class="secondary" on:click={() => openAccessibilityDialog()} type="button">开启</button>
                  {/if}
                </div>
              </div>
              <div class="settings-row">
                <span>
                  <strong>全局快捷键</strong>
                  <small>使用 Alt + Space 显示或隐藏搜索，Alt + V 从底部打开剪贴板。</small>
                </span>
                <em>Alt Space / Alt V</em>
              </div>
            </div>
          </div>
          <div class="settings-group">
            <h2>应用</h2>
            <div class="settings-list">
              <div class="settings-row">
                <span>
                  <strong>检查更新</strong>
                  <small>{updateStatus || "检查 GitHub Releases 中是否有新版本。"}</small>
                </span>
                <div class="settings-actions">
                  {#if updateReleaseUrl}
                    <button class="secondary" on:click={openUpdateRelease} type="button">发布页</button>
                  {/if}
                  <button class="secondary" disabled={updateLoading} on:click={checkUpdate} type="button">
                    {#if updateLoading}<Loader2 class="spin" size={14} />{/if}
                    检查
                  </button>
                </div>
              </div>
              <div class="settings-row">
                <span>
                  <strong>点击外部关闭</strong>
                  <small>窗口失去焦点后自动隐藏。</small>
                </span>
                <em>已开启</em>
              </div>
            </div>
          </div>
        </section>
      {/if}
    </section>

    {#if showCustomImportDialog}
      <section class="modal-backdrop">
        <article class="import-dialog">
          <header>
            <h2>导入配置文件</h2>
            <button aria-label="关闭" on:click={closeCustomImportDialog} type="button"><X size={24} /></button>
          </header>
          <div class="import-body">
            <p class="import-note">{customImportHelpText()}</p>
            <div class="import-mode">
              <span>导入方式：</span>
              <label>
                <input bind:group={customImportMode} type="radio" value="local" />
                本地导入
              </label>
              <label>
                <input bind:group={customImportMode} type="radio" value="remote" />
                远程导入
              </label>
            </div>
            {#if customImportMode === "remote"}
              <label class="remote-field">
                <span>远程地址：</span>
                <input bind:value={customRemoteUrl} placeholder="https://example.com/plugin.json" />
              </label>
            {:else}
              <div class="local-import-row">
                <span>本地导入：</span>
                <button class="import-button" on:click={chooseCustomPluginFile} type="button">
                  <DownloadCloud size={17} />
                  导入文件
                </button>
              </div>
            {/if}
            {#if customImportStatus}
              <div class="import-error">{customImportStatus}</div>
            {/if}
          </div>
          <footer>
            <button class="secondary" on:click={closeCustomImportDialog} type="button">关闭</button>
            <button class="import-button" disabled={loading} on:click={confirmCustomPluginImport} type="button">
              {loading ? "导入中" : "导入"}
            </button>
          </footer>
        </article>
      </section>
    {/if}
  </main>
{/if}

{#if showAccessibilityDialog && view !== "permission"}
  <section class="modal-backdrop permission-backdrop">
    <div class="permission-dialog" role="dialog" aria-modal="true" aria-labelledby="permission-modal-title">
      <header>
        <div class="permission-heading">
          <span class="permission-icon"><ShieldCheck size={21} /></span>
          <span>
            <small>自动粘贴</small>
            <h2 id="permission-modal-title">开启辅助功能权限</h2>
          </span>
        </div>
        <button class="permission-close" aria-label="关闭" on:click={() => closeAccessibilityDialog()} type="button">
          <X size={18} />
        </button>
      </header>
      <div class="permission-body">
        <p>接下来 macOS 会显示系统授权提示。开启后，选择剪贴板记录即可自动粘贴回原输入框。</p>
        <div class="permission-flow">
          <div class="current">
            <span>1</span>
            <div>
              <strong>确认授权说明</strong>
              <small>VviTools 仅使用此权限完成自动粘贴。</small>
            </div>
          </div>
          <div>
            <span>2</span>
            <div>
              <strong>在系统中允许</strong>
              <small>按 macOS 提示开启 VviTools。</small>
            </div>
          </div>
        </div>
      </div>
      <footer>
        <button class="permission-cancel" on:click={() => closeAccessibilityDialog()} type="button">暂不开启</button>
        <button
          class="permission-primary"
          disabled={accessibilityPermissionLoading}
          on:click={requestAccessibilityPermission}
          type="button"
        >
          继续授权
          <ArrowRight size={16} />
        </button>
      </footer>
    </div>
  </section>
{/if}
