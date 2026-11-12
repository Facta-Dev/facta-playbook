// <app> — the side panel transport.
//
// The side panel is opened from the toolbar; the behavior is set once, at the
// top level of the worker (PLAYBOOK §2.2).

export interface PanelBehavior {
  /** Open the side panel when the toolbar icon is clicked. */
  readonly openPanelOnActionClick: boolean;
}

export function setPanelBehavior(behavior: PanelBehavior): Promise<void> {
  return chrome.sidePanel.setPanelBehavior({
    openPanelOnActionClick: behavior.openPanelOnActionClick,
  });
}
