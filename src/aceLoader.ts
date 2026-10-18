/**
 * Loads Ace, the editor Cloudscape's CodeEditor runs on (261017-insights-query
 * refined-mockups Q3), when the query field is first shown. The app's content
 * security policy loads scripts from the app itself only, so the modules the
 * editor asks for (the plain text mode and the two themes CodeEditor uses
 * for light and dark) are bundled and registered here instead of being
 * fetched from a base path. Nothing is loaded from the network. Tests
 * replace this module.
 *
 * Since 261017-insights-query U2 two more are registered: the completion
 * extension CodeEditor loads (`ext-language_tools`, U2:BR9.1), and the
 * query's own mode, whose highlight rules are `QUERY_HIGHLIGHT_RULES`
 * (U2:BR9.2). Both are Should: without them the field still works as a
 * plain text field.
 */

import { QUERY_HIGHLIGHT_RULES, QUERY_LANGUAGE } from "./analytics";

/** The Ace module object CodeEditor takes as its `ace` property. */
export type AceModule = typeof import("ace-builds");

let loading: Promise<AceModule> | null = null;

type ModeClass = new () => object;
type RulesClass = new () => { $rules: unknown };

/**
 * The query's mode: Ace's text mode with the query's highlight rules. Built
 * from the classes the loaded Ace defines, so no other module is needed.
 */
export function queryMode(ace: AceModule): { Mode: ModeClass } {
  const { Mode: TextMode } = ace.require("ace/mode/text") as { Mode: ModeClass };
  const { TextHighlightRules } = ace.require("ace/mode/text_highlight_rules") as {
    TextHighlightRules: RulesClass;
  };
  class QueryHighlightRules extends TextHighlightRules {
    constructor() {
      super();
      this.$rules = { start: QUERY_HIGHLIGHT_RULES.map((rule) => ({ ...rule })) };
    }
  }
  class QueryMode extends TextMode {
    HighlightRules = QueryHighlightRules;
    $id = `ace/mode/${QUERY_LANGUAGE}`;
  }
  return { Mode: QueryMode };
}

/** Loads Ace once and registers the bundled modules; later calls share it. */
export function loadAce(): Promise<AceModule> {
  loading ??= import("ace-builds").then((module) => {
    const ace = ((module as { default?: AceModule }).default ?? module) as AceModule;
    ace.config.setModuleLoader(
      "ace/mode/text",
      () => import("ace-builds/src-noconflict/mode-text"),
    );
    ace.config.setModuleLoader(`ace/mode/${QUERY_LANGUAGE}`, () => Promise.resolve(queryMode(ace)));
    ace.config.setModuleLoader(
      "ace/ext/language_tools",
      () => import("ace-builds/src-noconflict/ext-language_tools"),
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
