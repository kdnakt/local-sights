/**
 * Loads Ace, the editor Cloudscape's CodeEditor runs on (261017-insights-query
 * refined-mockups Q3), when the query field is first shown. The app's content
 * security policy loads scripts from the app itself only, so the modules the
 * editor asks for (the plain text mode and the two themes CodeEditor uses
 * for light and dark) are bundled and registered here instead of being
 * fetched from a base path. Nothing is loaded from the network. Tests
 * replace this module.
 */

/** The Ace module object CodeEditor takes as its `ace` property. */
export type AceModule = typeof import("ace-builds");

let loading: Promise<AceModule> | null = null;

/** Loads Ace once and registers the bundled modules; later calls share it. */
export function loadAce(): Promise<AceModule> {
  loading ??= import("ace-builds").then((module) => {
    const ace = ((module as { default?: AceModule }).default ?? module) as AceModule;
    ace.config.setModuleLoader(
      "ace/mode/text",
      () => import("ace-builds/src-noconflict/mode-text"),
    );
    ace.config.setModuleLoader(
      "ace/theme/dawn",
      () => import("ace-builds/src-noconflict/theme-dawn"),
    );
    ace.config.setModuleLoader(
      "ace/theme/tomorrow_night_bright",
      () => import("ace-builds/src-noconflict/theme-tomorrow_night_bright"),
    );
    return ace;
  });
  return loading;
}
