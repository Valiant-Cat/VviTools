<script lang="ts">
  import { ArrowLeft, FolderOpen, Trash2 } from "lucide-svelte";

  import "./clipboard.css";
  import type { ClipboardSettings, ClipboardStorageInfo } from "./types";

  export let settings: ClipboardSettings;
  export let storageInfo: ClipboardStorageInfo;
  export let loading: boolean;
  export let clearConfirm: boolean;
  export let onBack: () => void | Promise<void>;
  export let onUpdate: (patch: Partial<ClipboardSettings>) => void | Promise<void>;
  export let onOpenStorage: () => void | Promise<void>;
  export let onClear: () => void | Promise<void>;
  export let onCancelClear: () => void;

  function changeRetention(event: Event) {
    void onUpdate({ retention_days: Number((event.currentTarget as HTMLSelectElement).value) });
  }

  function changeMaxItems(event: Event) {
    void onUpdate({ max_items: Number((event.currentTarget as HTMLSelectElement).value) });
  }

  function formatBytes(value: number) {
    if (value < 1024) return `${value} B`;
    const units = ["KB", "MB", "GB"];
    let size = value / 1024;
    let unitIndex = 0;
    while (size >= 1024 && unitIndex < units.length - 1) {
      size /= 1024;
      unitIndex += 1;
    }
    return `${size >= 10 ? size.toFixed(0) : size.toFixed(1)} ${units[unitIndex]}`;
  }
</script>

<div class="clipboard-settings-page">
  <button class="clipboard-settings-back" on:click={onBack} type="button">
    <ArrowLeft size={18} />
    返回系统剪贴板
  </button>
  <div class="clipboard-settings-heading">
    <div>剪贴板设置</div>
    <small>控制记录范围、历史保留和本地存储</small>
  </div>
  <section class="clipboard-settings-panel">
    <div class="clipboard-settings-group">
      <h2>记录</h2>
      <div class="clipboard-settings-list">
        <div class="clipboard-settings-row">
          <span>
            <strong>记录剪贴板</strong>
            <small>关闭后暂停新增记录，已有历史和收藏不会被删除。</small>
          </span>
          <button
            class="clipboard-settings-switch"
            class:enabled={settings.enabled}
            disabled={loading}
            aria-pressed={settings.enabled}
            on:click={() => onUpdate({ enabled: !settings.enabled })}
            type="button"
          >
            <span class="clipboard-sr-only">{settings.enabled ? "已开启" : "已关闭"}</span>
            <span class="clipboard-settings-switch-thumb"></span>
          </button>
        </div>
        <div class="clipboard-settings-row compact">
          <span>
            <strong>文本</strong>
            <small>保存复制的文字、链接和代码片段。</small>
          </span>
          <button
            class="clipboard-settings-switch"
            class:enabled={settings.capture_text}
            disabled={loading}
            aria-pressed={settings.capture_text}
            on:click={() => onUpdate({ capture_text: !settings.capture_text })}
            type="button"
          >
            <span class="clipboard-sr-only">{settings.capture_text ? "已开启" : "已关闭"}</span>
            <span class="clipboard-settings-switch-thumb"></span>
          </button>
        </div>
        <div class="clipboard-settings-row compact">
          <span>
            <strong>图片</strong>
            <small>图片会作为 PNG 文件保存在本机。</small>
          </span>
          <button
            class="clipboard-settings-switch"
            class:enabled={settings.capture_images}
            disabled={loading}
            aria-pressed={settings.capture_images}
            on:click={() => onUpdate({ capture_images: !settings.capture_images })}
            type="button"
          >
            <span class="clipboard-sr-only">{settings.capture_images ? "已开启" : "已关闭"}</span>
            <span class="clipboard-settings-switch-thumb"></span>
          </button>
        </div>
        <div class="clipboard-settings-row compact">
          <span>
            <strong>文件</strong>
            <small>只保存文件路径，不会复制或移动原文件。</small>
          </span>
          <button
            class="clipboard-settings-switch"
            class:enabled={settings.capture_files}
            disabled={loading}
            aria-pressed={settings.capture_files}
            on:click={() => onUpdate({ capture_files: !settings.capture_files })}
            type="button"
          >
            <span class="clipboard-sr-only">{settings.capture_files ? "已开启" : "已关闭"}</span>
            <span class="clipboard-settings-switch-thumb"></span>
          </button>
        </div>
      </div>
    </div>
    <div class="clipboard-settings-group">
      <h2>历史</h2>
      <div class="clipboard-settings-list">
        <label class="clipboard-settings-row" for="clipboard-retention">
          <span>
            <strong>保留时长</strong>
            <small>到期的普通记录会自动清理，收藏内容不受影响。</small>
          </span>
          <select
            id="clipboard-retention"
            class="clipboard-settings-select"
            disabled={loading}
            value={settings.retention_days}
            on:change={changeRetention}
          >
            <option value="1">1 天</option>
            <option value="7">7 天</option>
            <option value="30">30 天</option>
            <option value="90">90 天</option>
            <option value="0">永久保留</option>
          </select>
        </label>
        <label class="clipboard-settings-row" for="clipboard-max-items">
          <span>
            <strong>最大历史条数</strong>
            <small>达到上限后优先保留收藏，再清理较旧的普通记录。</small>
          </span>
          <select
            id="clipboard-max-items"
            class="clipboard-settings-select"
            disabled={loading}
            value={settings.max_items}
            on:change={changeMaxItems}
          >
            <option value="100">100 条</option>
            <option value="200">200 条</option>
            <option value="500">500 条</option>
            <option value="1000">1000 条</option>
          </select>
        </label>
      </div>
    </div>
    <div class="clipboard-settings-group">
      <h2>存储</h2>
      <div class="clipboard-settings-list">
        <div class="clipboard-settings-row">
          <span>
            <strong>本地存储位置</strong>
            <small class="clipboard-settings-path" title={storageInfo.directory}>
              {storageInfo.directory || "正在读取..."}
            </small>
          </span>
          <button class="clipboard-settings-secondary clipboard-settings-storage" on:click={onOpenStorage} type="button">
            <FolderOpen size={15} />
            在 Finder 中打开
          </button>
        </div>
        <div class="clipboard-settings-row">
          <span>
            <strong>存储占用</strong>
            <small>{storageInfo.item_count} 条记录，其中 {storageInfo.image_count} 张图片</small>
          </span>
          <em>{formatBytes(storageInfo.total_bytes)}</em>
        </div>
        <div class="clipboard-settings-row">
          <span>
            <strong>清空全部历史</strong>
            <small>删除所有记录和已保存图片，剪贴板设置保持不变。</small>
          </span>
          <div class="clipboard-settings-actions">
            {#if clearConfirm}
              <button class="clipboard-settings-secondary" on:click={onCancelClear} type="button">取消</button>
            {/if}
            <button
              class:confirming={clearConfirm}
              class="clipboard-settings-danger"
              disabled={loading || storageInfo.item_count === 0}
              on:click={onClear}
              type="button"
            >
              <Trash2 size={15} />
              {clearConfirm ? "确认清空" : "清空"}
            </button>
          </div>
        </div>
      </div>
    </div>
  </section>
</div>
