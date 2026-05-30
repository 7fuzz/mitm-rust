import React, { useEffect, useRef } from 'react';

interface ModalProps {
  isOpen: boolean;
  onClose: () => void;
  title: string;
  children: React.ReactNode;
  footer?: React.ReactNode;
  maxWidth?: 'sm' | 'md' | 'lg' | 'xl' | '2xl' | '3xl' | '4xl' | '5xl';
  className?: string;
}

export function Modal({ 
  isOpen, 
  onClose, 
  title, 
  children, 
  footer, 
  maxWidth = 'md',
  className = '' 
}: ModalProps) {
  const modalRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === 'Escape' && !e.defaultPrevented) onClose();
    };

    if (isOpen) {
      document.addEventListener('keydown', handleEscape);
      document.body.style.overflow = 'hidden';
    }

    return () => {
      document.removeEventListener('keydown', handleEscape);
      document.body.style.overflow = 'unset';
    };
  }, [isOpen, onClose]);

  const handleBackdropClick = (e: React.MouseEvent) => {
    if (modalRef.current && !modalRef.current.contains(e.target as Node)) {
      onClose();
    }
  };

  if (!isOpen) return null;

  const maxWidthClasses = {
    sm: 'max-w-sm',
    md: 'max-w-md',
    lg: 'max-w-lg',
    xl: 'max-w-xl',
    '2xl': 'max-w-2xl',
    '3xl': 'max-w-3xl',
    '4xl': 'max-w-4xl',
    '5xl': 'max-w-5xl',
  };

  return (
    <div 
      className="fixed inset-0 z-100 flex items-center justify-center bg-backdrop backdrop-blur-sm px-4 animate-in fade-in duration-200"
      onClick={handleBackdropClick}
    >
      <div 
        ref={modalRef}
        className={`bg-zinc-950 border border-zinc-800 rounded-lg shadow-2xl shadow-app-shadow w-full ${maxWidthClasses[maxWidth]} flex flex-col overflow-hidden animate-in zoom-in-95 duration-200 ${className}`}
      >
        {/* Header */}
        <div className="px-4 py-3 border-b border-zinc-800 bg-zinc-900/50 flex items-center justify-between">
          <h3 className="text-zinc-200 font-bold uppercase tracking-widest text-[11px]">{title}</h3>
          <button 
            onClick={onClose} 
            className="text-zinc-500 hover:text-zinc-300 transition-colors"
            title="Close (Esc)"
          >
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"></line>
              <line x1="6" y1="6" x2="18" y2="18"></line>
            </svg>
          </button>
        </div>

        {/* Content */}
        <div className="flex-1 overflow-y-auto max-h-[85vh] scrollbar-thin scrollbar-thumb-zinc-800">
          {children}
        </div>

        {/* Footer */}
        {footer && (
          <div className="p-4 bg-zinc-900/30 border-t border-zinc-800">
            {footer}
          </div>
        )}
      </div>
    </div>
  );
}
