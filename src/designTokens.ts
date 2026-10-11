/**
 * Cloudscape design tokens for the parts the screen still draws itself (the
 * virtual-scroll log list, the two-pane layout; NFR2, design-system-mapping
 * "明暗モード"). Each token is a `var(--awsui-…, fallback)` string that the
 * Cloudscape global styles redefine on the element carrying the dark-mode
 * class, so the custom properties set here must live on that same element
 * (`document.body`, where `applyMode` puts the class): the stylesheet then
 * reads `var(--ls-…)` and follows the light or dark mode without any color
 * of its own (FR4.5, accessibility C2).
 */

import {
  borderRadiusContainer,
  colorBackgroundCellShaded,
  colorBackgroundContainerContent,
  colorBackgroundItemSelected,
  colorBackgroundLayoutMain,
  colorBorderDividerDefault,
  colorBorderDividerSecondary,
  colorBorderItemFocused,
  colorBorderItemSelected,
  colorTextBodyDefault,
  colorTextBodySecondary,
  colorTextHeadingDefault,
  fontFamilyBase,
  fontFamilyMonospace,
  fontSizeBodyM,
  fontSizeBodyS,
  fontWeightHeadingS,
  lineHeightBodyM,
  spaceScaledM,
  spaceScaledS,
  spaceScaledXs,
  spaceScaledXxs,
} from "@cloudscape-design/design-tokens";

/** The custom properties `styles.css` reads, by name, with their token values. */
export const APP_DESIGN_TOKENS: Readonly<Record<string, string>> = {
  "--ls-color-text": colorTextBodyDefault,
  "--ls-color-text-secondary": colorTextBodySecondary,
  "--ls-color-heading": colorTextHeadingDefault,
  "--ls-color-background": colorBackgroundContainerContent,
  "--ls-color-background-layout": colorBackgroundLayoutMain,
  "--ls-color-background-header": colorBackgroundCellShaded,
  "--ls-color-background-selected": colorBackgroundItemSelected,
  "--ls-color-border": colorBorderDividerDefault,
  "--ls-color-border-secondary": colorBorderDividerSecondary,
  "--ls-color-border-selected": colorBorderItemSelected,
  "--ls-color-focus": colorBorderItemFocused,
  "--ls-font-family": fontFamilyBase,
  "--ls-font-family-monospace": fontFamilyMonospace,
  "--ls-font-size": fontSizeBodyM,
  "--ls-font-size-small": fontSizeBodyS,
  "--ls-line-height": lineHeightBodyM,
  "--ls-font-weight-heading": fontWeightHeadingS,
  "--ls-space-xxs": spaceScaledXxs,
  "--ls-space-xs": spaceScaledXs,
  "--ls-space-s": spaceScaledS,
  "--ls-space-m": spaceScaledM,
  "--ls-radius": borderRadiusContainer,
};

/** Sets every token of {@link APP_DESIGN_TOKENS} as a custom property of `target`. */
export function applyDesignTokens(target: HTMLElement): void {
  for (const [name, value] of Object.entries(APP_DESIGN_TOKENS)) {
    target.style.setProperty(name, value);
  }
}
