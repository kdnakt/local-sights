import { useEffect, useRef } from "react";
import Box from "@cloudscape-design/components/box";
import Button, { type ButtonProps } from "@cloudscape-design/components/button";
import SideNavigation from "@cloudscape-design/components/side-navigation";
import { APP_PAGES, PAGE_NAME_KEY, type AppPage } from "../analytics";
import type { Translate } from "../i18n/messages";

const HREF_PREFIX = "#";

export interface AppNavigationProps {
  page: AppPage;
  /** Whether the navigation is open; closed leaves only the open button. */
  open: boolean;
  t: Translate;
  onNavigate: (page: AppPage) => void;
  onOpenChange: (open: boolean) => void;
}

/**
 * The navigation at the left edge (261017-insights-query U1:BR1.1,
 * refined-mockups screens 1 and 1b): Cloudscape's SideNavigation with Log
 * Management and Log Analytics, the current one marked (bold and
 * `aria-current="page"`, the component's own). The close button at the
 * right of its heading closes it, leaving a slim strip with the button that
 * opens it again; closing moves the focus to that button and opening moves
 * it back to the close button, as the log group pane does. The parent keeps
 * whether it is open until the app closes. Choosing a page never reloads
 * anything: the link's default is prevented and the parent switches.
 */
export function AppNavigation({ page, open, t, onNavigate, onOpenChange }: AppNavigationProps) {
  const closeRef = useRef<ButtonProps.Ref>(null);
  const openRef = useRef<ButtonProps.Ref>(null);
  const firstRender = useRef(true);

  useEffect(() => {
    if (firstRender.current) {
      firstRender.current = false;
      return;
    }
    (open ? closeRef : openRef).current?.focus();
  }, [open]);

  if (!open) {
    return (
      <div className="app-navigation-collapsed" data-testid="app-navigation-collapsed">
        <Button
          ref={openRef}
          variant="icon"
          iconName="menu"
          ariaLabel={t("nav.open")}
          onClick={() => onOpenChange(true)}
          data-testid="app-navigation-open-button"
        />
      </div>
    );
  }

  return (
    <nav className="app-navigation" aria-label={t("nav.label")} data-testid="app-navigation">
      <div className="app-navigation-header">
        <Box variant="h2" fontSize="heading-s" padding="n">
          {t("nav.title")}
        </Box>
        <Button
          ref={closeRef}
          variant="icon"
          iconName="close"
          ariaLabel={t("nav.close")}
          onClick={() => onOpenChange(false)}
          data-testid="app-navigation-close-button"
        />
      </div>
      <SideNavigation
        activeHref={`${HREF_PREFIX}${page}`}
        items={APP_PAGES.map((item) => ({
          type: "link",
          text: t(PAGE_NAME_KEY[item]),
          href: `${HREF_PREFIX}${item}`,
        }))}
        onFollow={(event) => {
          event.preventDefault();
          const target = APP_PAGES.find((item) => `${HREF_PREFIX}${item}` === event.detail.href);
          if (target !== undefined) {
            onNavigate(target);
          }
        }}
        data-testid="app-navigation-links"
      />
    </nav>
  );
}
