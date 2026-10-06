import React, { useMemo } from 'react';
import { UrlEncodedTree } from '../UrlEncodedTree';
import type { MultipartField } from '../../../types';
import { detectBodyKind, multipartBoundary, parseMultipart, parseUrlEncoded } from '../../../utils/bodyFormat';

const NO_DRAFTS = new Set<string>();
const noop = () => {};

export const isFormBody = (contentType: string, body: string) => {
  const kind = detectBodyKind(contentType, body);
  return kind === 'urlencoded' || kind === 'multipart';
};

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / 1024 / 1024).toFixed(1)} MB`;
}

const dataUriBytes = (uri: string) => {
  const b64 = uri.slice(uri.indexOf(',') + 1);
  return Math.floor((b64.length * 3) / 4) - (b64.endsWith('==') ? 2 : b64.endsWith('=') ? 1 : 0);
};

const MultipartFieldValue: React.FC<{ field: MultipartField }> = ({ field }) => {
  if (field.type === 'text') {
    return <span className="block px-2 py-0.5 text-foreground break-all whitespace-pre-wrap">{field.value}</span>;
  }
  return (
    <div className="px-2 py-0.5 space-y-1">
      <div className="flex flex-wrap items-center gap-1.5">
        <span className="text-foreground break-all">{field.file_name}</span>
        <span className="text-3xs text-muted-foreground bg-background px-1.5 rounded border border-border">
          {field.content_type}
        </span>
        <span className="text-3xs text-muted-foreground tabular-nums">{formatBytes(dataUriBytes(field.value))}</span>
      </div>
      {field.content_type?.startsWith('image/') && (
        <img src={field.value} alt={field.file_name} className="max-h-32 max-w-full rounded border border-border" />
      )}
    </div>
  );
};

const MultipartView: React.FC<{ fields: MultipartField[] }> = ({ fields }) => (
  <div className="border border-border rounded-lg overflow-hidden bg-surface divide-y divide-border">
    {fields.map((field) => (
      <div key={field.id} className="flex items-start gap-2 px-2 py-1 hover:bg-neutral-subtle/50">
        <span className="w-1/3 shrink-0 px-2 py-0.5 text-muted-foreground break-all">{field.key}</span>
        <div className="flex-1 min-w-0">
          <MultipartFieldValue field={field} />
        </div>
      </div>
    ))}
  </div>
);

export const FormBodyView: React.FC<{ body: string; contentType: string }> = ({ body, contentType }) => {
  const content = useMemo(() => {
    const kind = detectBodyKind(contentType, body);
    if (kind === 'urlencoded') {
      return <UrlEncodedTree params={parseUrlEncoded(body)} onChange={noop} readOnly draftIds={NO_DRAFTS} />;
    }
    const boundary = multipartBoundary(contentType, body);
    if (kind === 'multipart' && boundary) return <MultipartView fields={parseMultipart(body, boundary)} />;
    return null;
  }, [body, contentType]);

  return <div className="h-full overflow-auto p-2 font-mono text-xs">{content}</div>;
};
