<script lang="ts">
  import { convertFileSrc } from "@tauri-apps/api/core";
  import { Check, ClipboardList, Copy, Heart, Search, Terminal, Trash2, X } from "lucide-svelte";

  import "./clipboard.css";
  import type { ClipboardFilter, ClipboardItem } from "./types";

  export let items: ClipboardItem[];
  export let query: string;
  export let filter: ClipboardFilter;
  export let selectedIndex: number;
  export let error: string;
  export let status: string;
  export let permissionWarning: boolean;
  export let permissionSupported: boolean;
  export let searchInput: HTMLInputElement;
  export let board: HTMLElement;
  export let onInput: () => void | Promise<void>;
  export let onKeydown: (event: KeyboardEvent) => void | Promise<void>;
  export let onFilter: (filter: ClipboardFilter) => void;
  export let onPaste: (item: ClipboardItem) => void | Promise<void>;
  export let onFavorite: (item: ClipboardItem) => void | Promise<void>;
  export let onDelete: (item: ClipboardItem) => void | Promise<void>;
  export let onPermission: () => void | Promise<void>;

  const filters: Array<{ key: ClipboardFilter; label: string }> = [
    { key: "all", label: "全部" },
    { key: "text", label: "文本" },
    { key: "image", label: "图片" },
    { key: "file", label: "文件" },
    { key: "favorite", label: "收藏" },
  ];

  function emptyText() {
    if (query) return "没有匹配记录";
    if (filter === "image") return "暂无图片记录";
    if (filter === "file") return "暂无文件记录";
    if (filter === "favorite") return "暂无收藏记录";
    if (filter === "text") return "暂无文本记录";
    return "暂无剪贴板历史";
  }

  function imageSrc(item: ClipboardItem) {
    return item.image_path ? convertFileSrc(item.image_path) : "";
  }

  function itemLabel(item: ClipboardItem) {
    if (item.kind === "image") return "图片";
    if (item.kind === "file") return item.file_paths?.length && item.file_paths.length > 1 ? "多文件" : "文件";
    return "纯文本";
  }

  function itemMeta(item: ClipboardItem) {
    if (item.kind === "image") return `${item.width || 0} x ${item.height || 0}`;
    if (item.kind === "file") return `${item.file_paths?.length || 1} 个文件`;
    return `${item.text.length} 个字符`;
  }

  function formatTime(value: string) {
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return "";
    return date.toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  }
</script>

<main class="clipboard-window" data-tauri-drag-region>
  <header class="copycat-topbar">
    <strong>剪贴板</strong>
    <div class="copycat-search">
      <Search size={18} />
      <input
        bind:this={searchInput}
        bind:value={query}
        on:input={onInput}
        on:keydown={onKeydown}
        placeholder="搜索"
        spellcheck="false"
      />
      {#if query}
        <button class="copycat-clear" aria-label="清空搜索" on:click={() => ((query = ""), onInput())} type="button">
          <X size={15} />
        </button>
      {/if}
    </div>
    <div class="copycat-filters">
      {#each filters as item}
        <button class:active={filter === item.key} on:click={() => onFilter(item.key)} type="button">
          {#if filter === item.key}<Check size={15} />{/if}{item.label}
        </button>
      {/each}
    </div>
    <div class="copycat-hotkey">
      <span>↯</span>
      <strong>系统快捷键</strong>
      <kbd>⌥ V</kbd>
    </div>
  </header>
  <section class="copycat-board" bind:this={board}>
    {#if error}
      <div class="copycat-status error"><Terminal size={18} />{error}</div>
    {:else}
      {#if status}
        <div class:warning={permissionWarning} class="copycat-status compact">
          <Terminal size={18} />{status}
          {#if permissionSupported}
            <button class="copycat-inline-link" on:click={onPermission} type="button">查看授权</button>
          {/if}
        </div>
      {/if}
      {#if items.length}
        {#each items as item, index}
          <article
            class:image-card={item.kind === "image"}
            class:file-card={item.kind === "file"}
            class:selected={selectedIndex === index}
            class="copycat-card text-card"
            data-selected={selectedIndex === index}
          >
            <button
              class="copycat-card-main"
              on:click={() => onPaste(item)}
              on:mouseenter={() => (selectedIndex = index)}
              type="button"
            >
              <span class="copycat-card-head">
                <span class="copycat-card-type">{itemLabel(item)}</span>
                <small>{formatTime(item.copied_at)}</small>
              </span>
              {#if item.kind === "image"}
                <div class="copycat-image-preview">
                  <img src={imageSrc(item)} alt={item.preview || "剪贴板图片"} />
                </div>
              {:else if item.kind === "file"}
                <p class="copycat-file-name">{item.preview || item.text}</p>
              {:else}
                <p>{item.preview || item.text}</p>
              {/if}
            </button>
            <footer>
              <span>{itemMeta(item)}</span>
              <div>
                <button
                  class:active={item.favorite}
                  aria-label={item.favorite ? "取消收藏" : "收藏"}
                  on:click={() => onFavorite(item)}
                  type="button"
                >
                  <Heart size={15} fill={item.favorite ? "currentColor" : "none"} />
                </button>
                <button aria-label="复制" on:click={() => onPaste(item)} type="button">
                  <Copy size={16} />
                </button>
                <button aria-label="删除" on:click={() => onDelete(item)} type="button">
                  <Trash2 size={16} />
                </button>
              </div>
            </footer>
          </article>
        {/each}
      {:else}
        <div class="copycat-empty">
          <ClipboardList size={46} />
          <strong>{emptyText()}</strong>
        </div>
      {/if}
    {/if}
  </section>
</main>
