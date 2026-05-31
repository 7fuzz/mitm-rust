import { createContext, useContext, useState, ReactNode, useCallback, useRef } from 'react';
import { Modal } from './Modal';
import { Button } from './Button';
import { Input } from './Input';

interface DialogOptions {
  title: string;
  message: string;
  initialValue?: string;
  confirmText?: string;
  cancelText?: string;
  isDestructive?: boolean;
  type: 'alert' | 'confirm' | 'prompt';
}

interface DialogState extends DialogOptions {
  resolve: (value: any) => void;
}

interface DialogContextType {
  confirm: (title: string, message: string, isDestructive?: boolean) => Promise<boolean>;
  prompt: (title: string, message: string, initialValue?: string) => Promise<string | null>;
  alert: (title: string, message: string) => Promise<void>;
}

const DialogContext = createContext<DialogContextType | null>(null);

export function DialogProvider({ children }: { children: ReactNode }) {
  const [state, setState] = useState<DialogState | null>(null);
  const [promptValue, setPromptValue] = useState('');
  const inputRef = useRef<HTMLInputElement>(null);

  const confirm = useCallback((title: string, message: string, isDestructive = false) => {
    return new Promise<boolean>((resolve) => {
      setState({
        type: 'confirm',
        title,
        message,
        isDestructive,
        confirmText: 'Confirm',
        cancelText: 'Cancel',
        resolve
      });
    });
  }, []);

  const prompt = useCallback((title: string, message: string, initialValue = '') => {
    setPromptValue(initialValue);
    return new Promise<string | null>((resolve) => {
      setState({
        type: 'prompt',
        title,
        message,
        initialValue,
        confirmText: 'Submit',
        cancelText: 'Cancel',
        resolve
      });
    });
  }, []);

  const alert = useCallback((title: string, message: string) => {
    return new Promise<void>((resolve) => {
      setState({
        type: 'alert',
        title,
        message,
        confirmText: 'OK',
        resolve
      });
    });
  }, []);

  const handleClose = () => {
    if (!state) return;
    state.resolve(state.type === 'prompt' ? null : false);
    setState(null);
  };

  const handleConfirm = () => {
    if (!state) return;
    const value = state.type === 'prompt' ? promptValue : true;
    state.resolve(value);
    setState(null);
  };

  return (
    <DialogContext.Provider value={{ confirm, prompt, alert }}>
      {children}

      <Modal
        isOpen={!!state}
        onClose={handleClose}
        title={state?.title || ''}
        maxWidth="sm"
        footer={
          <div className="flex items-center justify-end gap-2">
            {state?.type !== 'alert' && (
              <Button variant="ghost" size="sm" onClick={handleClose}>
                {state?.cancelText || 'Cancel'}
              </Button>
            )}
            <Button
              variant={state?.isDestructive ? 'destructive' : 'primary'}
              size="md"
              onClick={handleConfirm}
              className="px-6"
            >
              {state?.confirmText || 'OK'}
            </Button>
          </div>
        }
      >
        <div className="p-5 flex flex-col gap-4">
          {state?.message && (
            <div className="text-zinc-400 text-xs leading-relaxed font-medium">
              {state.message}
            </div>
          )}

          {state?.type === 'prompt' && (
            <div className="mt-2">
              <Input
                autoFocus
                value={promptValue}
                onChange={(e) => setPromptValue(e.target.value)}
                onKeyDown={(e) => e.key === 'Enter' && handleConfirm()}
                variant="amber"
                className="p-3 placeholder:text-zinc-600 font-bold"
                placeholder="Type here..."
              />
            </div>
          )}
        </div>
      </Modal>
    </DialogContext.Provider>
  );
}

export function useDialog() {
  const context = useContext(DialogContext);
  if (!context) throw new Error('useDialog must be used within a DialogProvider');
  return context;
}
