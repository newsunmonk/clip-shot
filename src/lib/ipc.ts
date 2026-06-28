// Rust 백엔드와 주고받는 타입 정의 + invoke 래퍼.
import { invoke } from "@tauri-apps/api/core";

export type CaptureMode = "region" | "window" | "display" | "all";
export type ImageFormat = "png" | "jpg";

export interface Shortcuts {
  region: string;
  window: string;
  display: string;
  allDisplays: string;
  history: string;
}

export interface Settings {
  autostart: boolean;
  hotkeysEnabled: boolean;
  shortcuts: Shortcuts;
  showToast: boolean;
  shutterSound: boolean;
  flashEffect: boolean;
  imageFormat: ImageFormat;
  historyLimit: number;
  language: "ko" | "en";
  theme: "system" | "light" | "dark";
}

export interface HistoryEntry {
  id: string;
  createdAt: number;
  mode: string;
  width: number;
  height: number;
}

export interface WindowInfo {
  id: number;
  title: string;
  appName: string;
}

export interface WindowRect {
  id: number;
  x: number;
  y: number;
  width: number;
  height: number;
  z: number;
  appName: string;
}

export interface DisplayInfo {
  id: number;
  name: string;
  x: number;
  y: number;
  width: number;
  height: number;
  scale: number;
  isPrimary: boolean;
}

export const captureAllDisplays = () => invoke<HistoryEntry>("capture_all_displays");
export const openCaptureOverlay = (mode: "region" | "window" | "display") =>
  invoke<void>("open_capture_overlay", { mode });
export const listWindowRects = () => invoke<WindowRect[]>("list_window_rects");
export const commitWindow = (id: number) => invoke<HistoryEntry>("commit_window", { id });
export const commitDisplay = (id: number) => invoke<HistoryEntry>("commit_display", { id });
export const overlayBg = (id: number) => invoke<string | null>("overlay_bg", { id });
export const captureRegion = (x: number, y: number, w: number, h: number) =>
  invoke<HistoryEntry>("capture_region", { x, y, w, h });
export const cancelRegion = () => invoke<void>("cancel_region");

export const listHistory = () => invoke<HistoryEntry[]>("list_history");
export const historyThumb = (id: string) => invoke<string>("history_thumb", { id });
export const historyFull = (id: string) => invoke<string>("history_full", { id });
export const copyHistory = (id: string) => invoke<void>("copy_history", { id });
export const deleteHistory = (id: string) => invoke<void>("delete_history", { id });
export const clearHistory = () => invoke<void>("clear_history");
export const exportHistory = (id: string, dest: string) =>
  invoke<void>("export_history", { id, dest });
export const defaultExportName = (id: string) => invoke<string>("default_export_name", { id });

export const getSettings = () => invoke<Settings>("get_settings");
export const saveSettings = (newSettings: Settings) =>
  invoke<void>("save_settings", { newSettings });
export const showMain = () => invoke<void>("show_main");

export const screenPermission = () => invoke<boolean>("screen_permission");
export const requestScreenPermission = () => invoke<boolean>("request_screen_permission");
export const openScreenSettings = () => invoke<void>("open_screen_settings");
export const openKeyboardSettings = () => invoke<void>("open_keyboard_settings");
export const restartApp = () => invoke<void>("restart_app");
export const setNativeScreenshots = (enabled: boolean) =>
  invoke<void>("set_native_screenshots", { enabled });
export const autostartStatus = () => invoke<boolean>("autostart_status");
