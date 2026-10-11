import "@cloudscape-design/global-styles/index.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider, type I18nProviderProps } from "@cloudscape-design/components/i18n";
import enMessages from "@cloudscape-design/components/i18n/messages/all.en";
import jaMessages from "@cloudscape-design/components/i18n/messages/all.ja";
import { App } from "./App";
import { applyDesignTokens } from "./designTokens";
import { detectLocale, type Locale } from "./i18n/messages";
import "./styles.css";

const container = document.getElementById("root");
if (!container) {
  throw new Error("The #root element is missing from index.html");
}

const languages = navigator.languages.length > 0 ? navigator.languages : [navigator.language];
const locale = detectLocale(languages);
document.documentElement.lang = locale;

// FR4.6: Cloudscape's own texts (calendar month names, "Close modal", ...)
// follow the OS language like the app's catalog does.
const cloudscapeMessages: Record<Locale, I18nProviderProps.Messages> = {
  en: enMessages,
  ja: jaMessages,
};

// FR4.5: the parts drawn by the app itself read Cloudscape's tokens.
applyDesignTokens(document.body);

createRoot(container).render(
  <StrictMode>
    <I18nProvider locale={locale} messages={[cloudscapeMessages[locale]]}>
      <App locale={locale} />
    </I18nProvider>
  </StrictMode>,
);
