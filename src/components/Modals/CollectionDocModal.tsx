import { useState, useEffect } from 'react';
import { Button, Modal, Textarea } from '../ui';
import { MarkdownViewer } from '../ui/MarkdownViewer';

interface CollectionDocModalProps {
  isOpen: boolean;
  groupName: string;
  initialDescription?: string;
  onClose: () => void;
  onSave: (newDescription: string) => void;
}

export function CollectionDocModal({
  isOpen,
  groupName,
  initialDescription = '',
  onClose,
  onSave,
}: CollectionDocModalProps) {
  const [description, setDescription] = useState(initialDescription);
  const [mode, setMode] = useState<'preview' | 'edit'>('preview');

  useEffect(() => {
    setDescription(initialDescription || '');
    setMode('preview');
  }, [initialDescription, isOpen]);

  const handleSave = () => {
    onSave(description);
    onClose();
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={`Collection Docs: ${groupName}`}
      maxWidth="2xl"
      footer={
        <div className="flex justify-between items-center w-full">
          <div className="flex bg-zinc-950 p-0.5 rounded border border-zinc-800">
            <button
              onClick={() => setMode('preview')}
              className={`px-3 py-1 text-[10px] font-bold uppercase tracking-wider rounded transition-all ${
                mode === 'preview' ? 'bg-purple-500/20 text-purple-300' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              Preview Docs
            </button>
            <button
              onClick={() => setMode('edit')}
              className={`px-3 py-1 text-[10px] font-bold uppercase tracking-wider rounded transition-all ${
                mode === 'edit' ? 'bg-purple-500/20 text-purple-300' : 'text-zinc-500 hover:text-zinc-300'
              }`}
            >
              Edit Markdown
            </button>
          </div>

          <div className="flex gap-2">
            <Button variant="ghost" size="sm" onClick={onClose}>
              Cancel
            </Button>
            <Button variant="purple" size="sm" onClick={handleSave} className="px-6">
              Save Documentation
            </Button>
          </div>
        </div>
      }
    >
      <div className="p-6 space-y-4">
        {mode === 'edit' ? (
          <div className="space-y-2">
            <label className="text-[10px] font-mono text-zinc-400 font-bold uppercase tracking-widest block">
              Collection Documentation & Testing Examples (Markdown Format)
            </label>
            <Textarea
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder={`# ${groupName} API Suite\n\nProvide overview documentation, authentication flows, environment requirements, and test scenarios for this collection.\n\n### Usage Example\n\`\`\`json\n{\n  "status": "success"\n}\n\`\`\``}
              className="w-full h-80 font-mono text-[11px] bg-zinc-950/80 border-zinc-800 p-3 leading-relaxed"
            />
          </div>
        ) : (
          <div className="p-3 bg-zinc-950/40 rounded border border-zinc-800/80">
            <MarkdownViewer content={description} />
          </div>
        )}
      </div>
    </Modal>
  );
}
