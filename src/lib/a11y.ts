import { tick } from "svelte";

const FOCUSABLE =
  'button:not([disabled]), [href], input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])';

function visibleFocusables(node: HTMLElement): HTMLElement[] {
  return Array.from(node.querySelectorAll<HTMLElement>(FOCUSABLE)).filter(
    (element) => element.getClientRects().length > 0,
  );
}

/**
 * Trap focus inside a modal dialog, close on Escape, and restore focus to the
 * element that opened it.
 */
export function focusTrap(node: HTMLElement, onClose: () => void) {
  const previous = document.activeElement as HTMLElement | null;

  function handleKeydown(event: KeyboardEvent) {
    if (event.key === "Escape") {
      event.preventDefault();
      onClose();
      return;
    }
    if (event.key !== "Tab") return;

    const items = visibleFocusables(node);
    if (items.length === 0) {
      event.preventDefault();
      node.focus();
      return;
    }
    const first = items[0];
    const last = items[items.length - 1];
    const active = document.activeElement as HTMLElement | null;
    if (event.shiftKey && (active === first || active === node)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  }

  node.addEventListener("keydown", handleKeydown);
  void tick().then(() => {
    const preferred =
      node.querySelector<HTMLElement>("[data-autofocus]") ??
      visibleFocusables(node)[0] ??
      node;
    preferred.focus();
  });

  return {
    destroy() {
      node.removeEventListener("keydown", handleKeydown);
      previous?.focus?.();
    },
  };
}
