import React, { useState } from 'react';
import { useWebhookStore } from '../../../../stores/useWebhookStore';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { Button, Input, Select } from '../../../common/ui';

export const HMAC_PROVIDER_OPTIONS = [
  { value: 'github', label: 'GitHub (X-Hub-Signature-256 / sha256=...)' },
  { value: 'stripe', label: 'Stripe (Stripe-Signature / t=...,v1=...)' },
  { value: 'shopify', label: 'Shopify (X-Shopify-Hmac-SHA256 / Base64)' },
  { value: 'raw_sha256', label: 'Generic SHA-256 (Hex)' },
  { value: 'raw_sha1', label: 'Generic SHA-1 (Hex)' },
  { value: 'raw_sha512', label: 'Generic SHA-512 (Hex)' },
] as const;

export const WebhookHmacSandbox: React.FC = () => {
  const {
    hmacSecret,
    setHmacSecret,
    hmacBody,
    setHmacBody,
    hmacProvider,
    setHmacProvider,
    computedSignature,
    calculateHmac,
  } = useWebhookStore();

  const [copiedKey, setCopiedKey] = useState<string | null>(null);

  const handleCopy = (text: string, key: string) => {
    navigator.clipboard.writeText(text);
    setCopiedKey(key);
    setTimeout(() => setCopiedKey(null), 2000);
  };

  return (
    <div className="bg-surface border border-border rounded-lg p-3 space-y-3 shadow-2xs">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <MingCuteIcon name="key_line" size={16} className="text-primary" />
          <span className="font-semibold text-foreground text-sm">HMAC Signature Sandbox & Generator</span>
        </div>
        <Button variant="primary" size="xs" icon="key_line" onClick={calculateHmac}>
          Calculate Signature
        </Button>
      </div>

      <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
        <div>
          <label className="block text-muted-foreground mb-1 text-2xs">Provider Format:</label>
          <Select
            value={hmacProvider}
            onChange={(e) => setHmacProvider(e.target.value as any)}
            options={HMAC_PROVIDER_OPTIONS}
            className="w-full"
          />
        </div>

        <div className="md:col-span-2">
          <label className="block text-muted-foreground mb-1 text-2xs">Signing Secret Key:</label>
          <Input
            value={hmacSecret}
            onChange={(e) => setHmacSecret(e.target.value)}
            placeholder="e.g. whsec_secret_key"
            className="font-mono"
          />
        </div>
      </div>

      <div>
        <label className="block text-muted-foreground mb-1 text-2xs">Payload Body:</label>
        <textarea
          rows={3}
          value={hmacBody}
          onChange={(e) => setHmacBody(e.target.value)}
          placeholder="JSON or raw webhook payload body..."
          className="w-full bg-background border border-border rounded p-2 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
        />
      </div>

      {computedSignature && (
        <div className="p-3 bg-background border border-border rounded font-mono text-xs space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">
              Header: <strong className="text-primary">{computedSignature.header_name}</strong>
            </span>
            <Button
              variant="ghost"
              size="xs"
              icon={copiedKey === 'hmac-sig' ? 'check_line' : 'copy_line'}
              onClick={() =>
                handleCopy(
                  `${computedSignature.header_name}: ${computedSignature.header_value}`,
                  'hmac-sig',
                )
              }
            >
              {copiedKey === 'hmac-sig' ? 'Copied' : 'Copy Header'}
            </Button>
          </div>
          <div className="p-2 bg-surface border border-border rounded text-foreground font-bold break-all text-xs">
            {computedSignature.header_value}
          </div>
        </div>
      )}
    </div>
  );
};
