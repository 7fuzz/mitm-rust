import React, { useEffect } from 'react';
import { MingCuteIcon } from '../MingCuteIcon';

export interface DialogProps {
  isOpen: boolean;
  onClose: () => void;
  title?: string;
  description?: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
  /** `full` is a large fixed-height dialog for side-by-side content */
  size?: 'sm' | 'md' | 'lg' | 'xl' | 'full';
  className?: string;
  /** Replaces the default padded, scrolling body classes */
  bodyClassName?: string;
}

export const Dialog: React.FC<DialogProps> = ({
  isOpen,
  onClose,
  title,
  description,
  children,
  footer,
  size = 'md',
  className = '',
  bodyClassName,
}) => {
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && isOpen) {
        onClose();
      }
    };
    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  if (!isOpen) return null;

  const sizeClasses =
    size === 'sm'
      ? 'max-w-sm'
      : size === 'md'
      ? 'max-w-md'
      : size === 'lg'
      ? 'max-w-lg'
      : size === 'xl'
      ? 'max-w-2xl'
      : 'max-w-6xl h-[85vh]';

  return (
    <div
      onClick={onClose}
      className="fixed inset-0 z-50 flex items-center justify-center p-4 bg-black/60 backdrop-blur-xs animate-in fade-in duration-150 select-none cursor-pointer"
    >
      {/* Dialog container */}
      <div
        onClick={(e) => e.stopPropagation()}
        className={`relative w-full ${sizeClasses} bg-surface border border-border rounded-xl shadow-2xl overflow-hidden flex flex-col z-10 text-xs text-foreground select-text cursor-default ${className}`}
      >
        {/* Header */}
        {(title || description) && (
          <div className="px-4 py-3 bg-header border-b border-border flex items-center justify-between shrink-0 select-none">
            <div>
              {title && <h3 className="text-sm font-semibold text-foreground tracking-tight">{title}</h3>}
              {description && <p className="text-muted-foreground text-2xs mt-0.5">{description}</p>}
            </div>
            <button
              onClick={onClose}
              className="p-1 rounded text-muted-foreground hover:text-foreground hover:bg-neutral-subtle transition-colors cursor-pointer select-none"
              title="Close (Esc)"
            >
              <MingCuteIcon name="close_line" size={16} />
            </button>
          </div>
        )}

        {/* Body */}
        <div className={bodyClassName ?? 'p-4 overflow-y-auto max-h-[75vh] space-y-3'}>{children}</div>

        {/* Footer */}
        {footer && (
          <div className="px-4 py-2.5 bg-header border-t border-border flex items-center justify-end gap-2 shrink-0 select-none">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
};
