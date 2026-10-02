interface DragResizeOptions {
  cursor: 'col-resize' | 'row-resize';
  /** Apply the new size straight to the DOM; called at most once per animation frame */
  onMove: (e: PointerEvent) => void;
  /** Commit the final size to state (and persistence) once */
  onEnd: () => void;
}

/**
 * Runs a resize drag without re-rendering React on every pointer move: callers mutate
 * element styles in `onMove` and write state only in `onEnd`.
 */
export function startDragResize(e: React.PointerEvent, { cursor, onMove, onEnd }: DragResizeOptions) {
  e.preventDefault();
  let frame = 0;
  let latest: PointerEvent | null = null;

  const handlePointerMove = (ev: PointerEvent) => {
    latest = ev;
    if (frame) return;
    frame = requestAnimationFrame(() => {
      frame = 0;
      if (latest) onMove(latest);
    });
  };

  const handlePointerUp = () => {
    cancelAnimationFrame(frame);
    if (latest) onMove(latest);
    window.removeEventListener('pointermove', handlePointerMove);
    window.removeEventListener('pointerup', handlePointerUp);
    document.body.style.cursor = '';
    document.body.style.userSelect = '';
    onEnd();
  };

  document.body.style.cursor = cursor;
  document.body.style.userSelect = 'none';
  window.addEventListener('pointermove', handlePointerMove);
  window.addEventListener('pointerup', handlePointerUp);
}

export const clamp = (value: number, min: number, max: number) => Math.min(Math.max(value, min), max);

export const clampPercent = (value: number, min = 15, max = 85) => clamp(value, min, max);
