import { useEffect, useRef } from 'react';
import { Button, Modal } from '../ui';

interface ConfirmModalProps {
  isOpen: boolean;
  title: string;
  message: string | React.ReactNode;
  confirmText?: string;
  cancelText?: string;
  isDestructive?: boolean;
  onClose: () => void;
  onConfirm: () => void;
}

export function ConfirmModal({
  isOpen,
  title,
  message,
  confirmText = 'Confirm',
  cancelText = 'Cancel',
  isDestructive = false,
  onClose,
  onConfirm
}: ConfirmModalProps) {
  const confirmBtnRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => confirmBtnRef.current?.focus(), 10);
    }
  }, [isOpen]);

  const handleConfirm = () => {
    onConfirm();
    onClose();
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={title}
      maxWidth="sm"
      footer={
        <div className="flex items-center justify-end gap-2">
          <Button
            variant="ghost"
            size="sm"
            onClick={onClose}
          >
            {cancelText}
          </Button>
          <Button
            ref={confirmBtnRef}
            variant={isDestructive ? 'destructive' : 'primary'}
            size="md"
            onClick={handleConfirm}
            className="px-6"
          >
            {confirmText}
          </Button>
        </div>
      }
    >
      <div className="p-5 flex flex-col gap-6">
        <div className="text-zinc-400 text-xs leading-relaxed font-medium">
          {message}
        </div>
      </div>
    </Modal>
  );
}
