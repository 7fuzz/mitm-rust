import React, { useState } from 'react';
import { useRepeaterStore } from '../../../stores/useRepeaterStore';
import { Dialog, Input, Button } from '../../common/ui';

export const CurlImportModal: React.FC = () => {
  const { isCurlModalOpen, setCurlModalOpen, importCurlCommand } = useRepeaterStore();
  const [curlText, setCurlText] = useState('');
  const [reqName, setReqName] = useState('Imported cURL');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!curlText.trim()) return;
    await importCurlCommand(curlText.trim(), reqName.trim());
    setCurlText('');
  };

  return (
    <Dialog
      isOpen={isCurlModalOpen}
      onClose={() => setCurlModalOpen(false)}
      title="Import cURL Command"
      description="Paste raw cURL request to parse headers, parameters, and body payload."
      size="lg"
      footer={
        <>
          <Button variant="subtle" onClick={() => setCurlModalOpen(false)}>
            Cancel
          </Button>
          <Button variant="primary" onClick={handleSubmit}>
            Import Request
          </Button>
        </>
      }
    >
      <form onSubmit={handleSubmit} className="space-y-3 font-sans text-xs">
        <div>
          <label className="block text-muted-foreground mb-1 font-medium">Request Name:</label>
          <Input
            value={reqName}
            onChange={(e) => setReqName(e.target.value)}
            placeholder="Imported cURL"
          />
        </div>

        <div>
          <label className="block text-muted-foreground mb-1 font-medium">Paste Raw cURL String:</label>
          <textarea
            rows={6}
            value={curlText}
            onChange={(e) => setCurlText(e.target.value)}
            placeholder={`curl -X POST "https://api.example.com/v1/auth" \\\n  -H "Content-Type: application/json" \\\n  -d '{"username": "admin"}'`}
            className="w-full bg-background border border-border rounded p-2.5 text-xs font-mono text-foreground focus:outline-none focus:border-primary select-text"
          />
        </div>
      </form>
    </Dialog>
  );
};
