import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { detectLocale } from "./i18n/messages";
import "./styles.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("The #root element is missing from index.html");
}

const languages = navigator.languages.length > 0 ? navigator.languages : [navigator.language];
const locale = detectLocale(languages);
document.documentElement.lang = locale;

createRoot(container).render(
  <StrictMode>
    <App locale={locale} />
  </StrictMode>,
);
