/* Ambient typing for Tauri's global API, which is injected into the WebView by
   `withGlobalTauri: true` in tauri.conf.json. There are no runtime imports: this
   file describes what the window already provides. */

interface TauriInvoke {
  <T>(command: string, args?: Record<string, unknown>): Promise<T>;
}

interface TauriGlobal {
  core: {
    invoke: TauriInvoke;
  };
}

interface Window {
  __TAURI__?: TauriGlobal;
}
