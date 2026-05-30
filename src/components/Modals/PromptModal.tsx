import { useState, useEffect, useRef } from 'react';
import { Button, Input, Modal } from '../ui';

interface PromptModalProps {
  isOpen: boolean;
  title: string;
  initialValue?: string;
  onClose: () => void;
  onSubmit: (value: string) => void;
}

export function PromptModal({ isOpen, title, initialValue = '', onClose, onSubmit }: PromptModalProps) {
  const [value, setValue] = useState(initialValue);
  const [prevOpen, setPrevOpen] = useState(isOpen);
  const [prevInitial, setPrevInitial] = useState(initialValue);
  const inputRef = useRef<HTMLInputElement>(null);

  if (isOpen && (!prevOpen || initialValue !== prevInitial)) {
    setPrevOpen(true);
    setPrevInitial(initialValue);
    setValue(initialValue);
  } else if (!isOpen && prevOpen) {
    setPrevOpen(false);
  }

  useEffect(() => {
    if (isOpen) {
      setTimeout(() => inputRef.current?.focus(), 10);
    }
  }, [isOpen]);

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    if (value.trim()) onSubmit(value.trim());
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
          <Button variant="ghost" size="sm" type="button" onClick={onClose}>
            Cancel
          </Button>
          <Button 
            variant="primary" 
            size="md" 
            type="button" 
            onClick={handleSubmit} 
            disabled={!value.trim()} 
            className="px-6"
          >
            Confirm
          </Button>
        </div>
      }
    >
      <form onSubmit={handleSubmit} className="p-4 flex flex-col gap-4">
        <Input
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          variant="amber"
          className="p-3 placeholder:text-zinc-600 font-bold"
          placeholder="Type here..."
        />
      </form>
    </Modal>
  );
}
