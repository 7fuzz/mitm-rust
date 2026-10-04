import React from 'react';
import { UrlEncodedEditor } from '../UrlEncodedEditor';
import { MultipartEditor } from '../MultipartEditor';
import { BinaryBodyEditor } from './BinaryBodyEditor';
import {
  isMultipartType,
  isUrlEncodedType,
  readStoredMultipart,
  readStoredUrlEncoded,
  storeMultipart,
  storeUrlEncoded,
} from '../../../utils/bodyFormat';

interface StructuredBodyEditorProps {
  bodyType: string;
  content: string;
  onChange: (content: string) => void;
  readOnly?: boolean;
  /** Editor for JSON / raw text bodies. */
  renderText: () => React.ReactNode;
}

/** Body editor for Repeater-style requests, where form bodies are stored as JSON field lists. */
export const StructuredBodyEditor: React.FC<StructuredBodyEditorProps> = ({ bodyType, content, onChange, readOnly, renderText }) => {
  if (isUrlEncodedType(bodyType)) {
    return (
      <div className="h-full overflow-y-auto p-2">
        <UrlEncodedEditor params={readStoredUrlEncoded(content)} onChange={(p) => onChange(storeUrlEncoded(p))} readOnly={readOnly} />
      </div>
    );
  }
  if (isMultipartType(bodyType)) {
    return (
      <div className="h-full overflow-y-auto p-2">
        <MultipartEditor fields={readStoredMultipart(content)} onChange={(f) => onChange(storeMultipart(f))} readOnly={readOnly} />
      </div>
    );
  }
  if (bodyType === 'binary') return <BinaryBodyEditor value={content} onChange={onChange} readOnly={readOnly} />;
  if (bodyType === 'none') return <div className="p-4 text-muted-foreground italic">This request has no body.</div>;
  return <div className="h-full overflow-hidden">{renderText()}</div>;
};
