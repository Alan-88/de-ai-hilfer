export function edgeSwipe(element: HTMLElement, onClose: () => void) {
  let start: { x: number; y: number } | null = null;
  const cancel = () => { start = null; };
  const handleStart = (event: TouchEvent) => {
    cancel();
    if (event.defaultPrevented || event.touches.length !== 1 || !matchMedia("(max-width: 600px)").matches) return;
    const touch = event.touches[0];
    const edgeOffset = touch.clientX - element.getBoundingClientRect().left;
    if (edgeOffset >= 0 && edgeOffset <= 20) start = { x: touch.clientX, y: touch.clientY };
  };
  const handleMove = (event: TouchEvent) => {
    if (!start) return;
    if (event.touches.length !== 1) return cancel();
    const dx = event.touches[0].clientX - start.x;
    const dy = Math.abs(event.touches[0].clientY - start.y);
    if (dx < -8 || (dy > 8 && dy >= Math.abs(dx))) return cancel();
    // Reserve only a confirmed horizontal edge gesture; vertical scrolling remains native.
    if (dx > 8 && dx > dy * 1.5 && event.cancelable) event.preventDefault();
  };
  const handleEnd = (event: TouchEvent) => {
    const origin = start;
    cancel();
    if (!origin || event.changedTouches.length !== 1 || event.touches.length !== 0) return;
    const dx = event.changedTouches[0].clientX - origin.x;
    const dy = Math.abs(event.changedTouches[0].clientY - origin.y);
    if (dx >= 72 && dx > dy * 1.5) onClose();
  };
  element.addEventListener("touchstart", handleStart, { passive: true });
  element.addEventListener("touchmove", handleMove, { passive: false });
  element.addEventListener("touchend", handleEnd, { passive: true });
  element.addEventListener("touchcancel", cancel, { passive: true });
  return {
    update(callback: () => void) { onClose = callback; },
    destroy() {
      element.removeEventListener("touchstart", handleStart);
      element.removeEventListener("touchmove", handleMove);
      element.removeEventListener("touchend", handleEnd);
      element.removeEventListener("touchcancel", cancel);
    },
  };
}
