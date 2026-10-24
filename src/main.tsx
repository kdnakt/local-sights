import "@cloudscape-design/global-styles/index.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { applyDesignTokens } from "./designTokens";
import "./styles.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("The #root element is missing from index.html");
}

// FR4.5: the parts drawn by the app itself read Cloudscape's tokens.
applyDesignTokens(document.body);

// 261022-small-features U1: the locale, Cloudscape's texts and the `lang`
// attribute are decided by `App` from the saved language preference and the
// OS languages (U1:BR1.2); `App` reads this browser's languages itself.
createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
