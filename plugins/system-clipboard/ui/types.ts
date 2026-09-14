export type ClipboardItem = {
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

export type ClipboardSettings = {
  enabled: boolean;
  retention_days: number;
  max_items: number;
  capture_text: boolean;
  capture_images: boolean;
  capture_files: boolean;
};

export type ClipboardStorageInfo = {
  directory: string;
  total_bytes: number;
  item_count: number;
  image_count: number;
};

export type ClipboardFilter = "all" | "text" | "image" | "file" | "favorite";
