import { useState, useEffect, useRef } from 'react';
import { Modal, Button, Textarea } from '../ui';

interface CurlImportModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (curlCommand: string) => void;
}

export function CurlImportModal({ isOpen, onClose, onSubmit }: CurlImportModalProps) {
  const [curlText, setCurlText] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (isOpen) {
      setCurlText('');
      // Attempt silent clipboard pre-fill if accessible, else default to empty
      navigator.clipboard?.readText?.()
        .then(clip => {
          if (clip && clip.trim().toLowerCase().startsWith('curl')) {
            setCurlText(clip.trim());
          }
        })
        .catch(() => { /* Clipboard permission denied or unavailable, user can paste manually */ });

      setTimeout(() => textareaRef.current?.focus(), 50);
    }
  }, [isOpen]);

  const handleSubmit = (e?: React.FormEvent) => {
    if (e) e.preventDefault();
    if (curlText.trim()) {
      onSubmit(curlText.trim());
      onClose();
    }
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title="Import Request from cURL"
      maxWidth="2xl"
      footer={
        <div className="flex items-center justify-between w-full">
          <span className="text-[10px] text-zinc-500 font-mono">
            Paste your raw cURL command above
          </span>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" type="button" onClick={onClose}>
              Cancel
            </Button>
            <Button
              variant="purple"
              size="md"
              type="button"
              onClick={() => handleSubmit()}
              disabled={!curlText.trim()}
              className="px-6"
            >
              Import Request
            </Button>
          </div>
        </div>
      }
    >
      <form onSubmit={handleSubmit} className="space-y-4 p-1">
        <div className="space-y-2">
          <label className="text-[10px] text-zinc-400 font-bold uppercase tracking-widest block">
            cURL Command
          </label>
          <Textarea
            ref={textareaRef}
            value={curlText}
            onChange={(e) => setCurlText(e.target.value)}
            placeholder={`curl -X POST 'https://api.example.com/v1/resource' \\\n  -H 'Content-Type: application/json' \\\n  -H 'Authorization: Bearer token123' \\\n  -d '{"key": "value"}'`}
            className="min-h-48 font-mono text-xs text-zinc-200 bg-zinc-950/80 border-zinc-700/60 focus:border-purple-500 leading-relaxed resize-y"
            spellCheck={false}
          />
        </div>
      </form>
    </Modal>
  );
}
