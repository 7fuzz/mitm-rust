import React from 'react';
import { Modal } from './Modal';
import { Button } from './Button';
import { useHotkeys } from '@/hooks/ui/useHotkeys';

interface ConfirmationModalProps {
  isOpen: boolean;
  onClose: () => void;
  onConfirm: () => void;
  title: string;
  message: string;
  confirmText?: string;
  cancelText?: string;
  variant?: 'destructive' | 'primary' | 'purple' | 'sky' | 'amber';
}

export function ConfirmationModal({
  isOpen,
  onClose,
  onConfirm,
  title,
  message,
  confirmText = 'Confirm',
  cancelText = 'Cancel',
  variant = 'primary'
}: ConfirmationModalProps) {
  
  useHotkeys([
    {
      key: 'Enter',
      enabled: isOpen,
      handler: () => {
        onConfirm();
        onClose();
      },
      stopPropagation: true
    },
    {
      key: 'Escape',
      enabled: isOpen,
      handler: () => onClose(),
      stopPropagation: true
    }
  ], [isOpen, onConfirm, onClose]);

  return (
    <Modal isOpen={isOpen} onClose={onClose} title={title} maxWidth="sm">
      <div className="p-6 space-y-6">
        <div className="text-sm text-zinc-400 font-medium leading-relaxed">
          {message}
        </div>
        
        <div className="flex items-center justify-end gap-3 pt-2">
          <Button variant="ghost" size="sm" onClick={onClose} className="font-bold uppercase tracking-widest text-[10px]">
            {cancelText} <span className="ml-2 opacity-30 text-[9px]">(ESC)</span>
          </Button>
          <Button 
            variant={variant === 'primary' ? 'secondary' : variant} 
            size="sm" 
            onClick={() => {
              onConfirm();
              onClose();
            }}
            className="font-bold uppercase tracking-widest text-[10px] min-w-24"
          >
            {confirmText} <span className="ml-2 opacity-30 text-[9px]">(ENTER)</span>
          </Button>
        </div>
      </div>
    </Modal>
  );
}
