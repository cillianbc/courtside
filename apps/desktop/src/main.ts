import "@fontsource/barlow-condensed/500.css";
import "@fontsource/barlow-condensed/600.css";
import "@fontsource/barlow-condensed/700.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "@fontsource/ibm-plex-mono/600.css";
import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-sans/500.css";
import "@fontsource/ibm-plex-sans/600.css";
import "./app.css";
import { mount } from "svelte";
import App from "./App.svelte";
import { inTauri } from "./lib/backend";

// In the app, the webview's own browser keys would reload the page (and
// lose what's typed), print, zoom (the video view would no longer match
// the panel) or open a find bar. None is a Courtside key; Courtside's own
// (⌘F search) are handled before this by the keymap.
if (inTauri) {
  window.addEventListener("keydown", (e) => {
    const mod = e.metaKey || e.ctrlKey;
    const key = e.key.toLowerCase();
    if (
      e.key === "F5" ||
      e.key === "F12" ||
      (mod && ["r", "p", "f", "g", "=", "+", "-", "0"].includes(key))
    ) {
      e.preventDefault();
    }
  });
}

const target = document.getElementById("app");
if (!target) throw new Error("no #app element");
export default mount(App, { target });
