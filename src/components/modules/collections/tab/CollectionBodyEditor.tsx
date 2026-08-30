import React, { useState } from 'react';
import { MingCuteIcon } from '../../../common/MingCuteIcon';
import { CodeEditor } from '../../../common/CodeEditor';
import { MultipartEditor } from '../../../common/MultipartEditor';
import { UrlEncodedEditor } from '../../../common/UrlEncodedEditor';
import type { MultipartField, UrlEncodedParam } from '../../../../types';
import {
  convertJsonToFormData,
  convertJsonToUrlEncoded,
  convertFormDataToJson,
  convertUrlEncodedToJson,
  convertRawToJson,
  convertFormDataToUrlEncoded,
  convertUrlEncodedToFormData,
} from '../../../../utils/bodyConverters';

interface CollectionBodyEditorProps {
  requestId: string;
  bodyType: string;
  bodyJson: string;
  bodyRaw: string;
  bodyFormData: string;
  bodyUrlencoded: string;
  onBodyTypeChange: (type: string) => void;
  onBodyJsonChange: (val: string) => void;
  onBodyRawChange: (val: string) => void;
  onBodyFormDataChange: (val: string) => void;
  onBodyUrlencodedChange: (val: string) => void;
}

export const CollectionBodyEditor: React.FC<CollectionBodyEditorProps> = ({
  requestId,
  bodyType,
  bodyJson,
  bodyRaw,
  bodyFormData,
  bodyUrlencoded,
  onBodyTypeChange,
  onBodyJsonChange,
  onBodyRawChange,
  onBodyFormDataChange,
  onBodyUrlencodedChange,
}) => {
  const [isConvertMenuOpen, setIsConvertMenuOpen] = useState(false);

  const parseUrlEncodedParams = (val: string): UrlEncodedParam[] => {
    try {
      const parsed = JSON.parse(val);
      if (Array.isArray(parsed)) return parsed;
    } catch (_) {}
    return [];
  };

  const parseMultipartFields = (val: string): MultipartField[] => {
    try {
      const parsed = JSON.parse(val);
      if (Array.isArray(parsed)) return parsed;
      if (parsed && Array.isArray(parsed.__form_data)) return parsed.__form_data;
    } catch (_) {}
    return [];
  };

  const handleConvert = async (type: string) => {
    setIsConvertMenuOpen(false);

    if (type === 'json_to_formdata') {
      const converted = convertJsonToFormData(bodyJson);
      onBodyFormDataChange(converted);
      onBodyTypeChange('form-data');
    } else if (type === 'json_to_urlencoded') {
      const converted = convertJsonToUrlEncoded(bodyJson);
      onBodyUrlencodedChange(converted);
      onBodyTypeChange('urlencoded');
    } else if (type === 'formdata_to_json') {
      const converted = await convertFormDataToJson(bodyFormData);
      onBodyJsonChange(converted);
      onBodyTypeChange('json');
    } else if (type === 'urlencoded_to_json') {
      const converted = convertUrlEncodedToJson(bodyUrlencoded);
      onBodyJsonChange(converted);
      onBodyTypeChange('json');
    } else if (type === 'raw_to_json') {
      const converted = convertRawToJson(bodyRaw);
      onBodyJsonChange(converted);
      onBodyTypeChange('json');
    } else if (type === 'formdata_to_urlencoded') {
      const converted = convertFormDataToUrlEncoded(bodyFormData);
      onBodyUrlencodedChange(converted);
      onBodyTypeChange('urlencoded');
    } else if (type === 'urlencoded_to_formdata') {
      const converted = convertUrlEncodedToFormData(bodyUrlencoded);
      onBodyFormDataChange(converted);
      onBodyTypeChange('form-data');
    }
  };

  return (
    <div className="h-full flex flex-col gap-2">
      <div className="flex items-center justify-between font-mono text-xs text-muted-foreground pb-1 shrink-0">
        <div className="flex items-center gap-3">
          {[
            { id: 'none', label: 'none' },
            { id: 'json', label: 'json' },
            { id: 'raw', label: 'raw' },
            { id: 'form-data', label: 'form-data' },
            { id: 'urlencoded', label: 'x-www-form-urlencoded' },
          ].map((opt) => (
            <label key={opt.id} className="flex items-center gap-1.5 cursor-pointer">
              <input
                type="radio"
                name={`body-type-${requestId}`}
                value={opt.id}
                checked={bodyType === opt.id}
                onChange={(e) => onBodyTypeChange(e.target.value)}
                className="accent-primary cursor-pointer"
              />
              <span className={bodyType === opt.id ? 'text-foreground font-semibold' : ''}>{opt.label}</span>
            </label>
          ))}
        </div>

        {/* Format Converter Dropdown */}
        <div className="relative">
          <button
            type="button"
            onClick={() => setIsConvertMenuOpen(!isConvertMenuOpen)}
            className="flex items-center gap-1 px-2 py-0.5 rounded bg-background hover:bg-neutral-subtle border border-border text-amber-500 font-bold text-[11px] cursor-pointer transition-colors"
            title="Auto Convert Format"
          >
            <MingCuteIcon name="transfer_line" size={13} />
            <span>Convert Format</span>
            <MingCuteIcon name="down_line" size={12} />
          </button>

          {isConvertMenuOpen && (
            <div className="absolute right-0 top-full mt-1 z-50 bg-surface border border-border rounded-lg shadow-xl py-1 text-[11px] w-48 font-sans text-foreground flex flex-col">
              <button
                type="button"
                onClick={() => handleConvert('json_to_formdata')}
                className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="right_line" size={12} className="text-amber-500" />
                <span>JSON ➔ Form-Data</span>
              </button>
              <button
                type="button"
                onClick={() => handleConvert('json_to_urlencoded')}
                className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="right_line" size={12} className="text-amber-500" />
                <span>JSON ➔ UrlEncoded</span>
              </button>
              <button
                type="button"
                onClick={() => handleConvert('formdata_to_json')}
                className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="right_line" size={12} className="text-emerald-500" />
                <span>Form-Data ➔ JSON (Base64)</span>
              </button>
              <button
                type="button"
                onClick={() => handleConvert('urlencoded_to_json')}
                className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="right_line" size={12} className="text-emerald-500" />
                <span>UrlEncoded ➔ JSON</span>
              </button>
              <button
                type="button"
                onClick={() => handleConvert('raw_to_json')}
                className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="right_line" size={12} className="text-emerald-500" />
                <span>Raw ➔ JSON</span>
              </button>
              <div className="my-1 border-t border-border" />
              <button
                type="button"
                onClick={() => handleConvert('formdata_to_urlencoded')}
                className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="right_line" size={12} className="text-primary" />
                <span>Form-Data ➔ UrlEncoded</span>
              </button>
              <button
                type="button"
                onClick={() => handleConvert('urlencoded_to_formdata')}
                className="flex items-center gap-1.5 px-3 py-1.5 hover:bg-neutral-subtle text-left transition-colors cursor-pointer"
              >
                <MingCuteIcon name="right_line" size={12} className="text-primary" />
                <span>UrlEncoded ➔ Form-Data</span>
              </button>
            </div>
          )}
        </div>
      </div>

      {bodyType === 'urlencoded' || bodyType === 'x-www-form-urlencoded' ? (
        <div className="flex-1 overflow-y-auto">
          <UrlEncodedEditor
            params={parseUrlEncodedParams(bodyUrlencoded)}
            onChange={(fields) => onBodyUrlencodedChange(JSON.stringify(fields))}
          />
        </div>
      ) : bodyType === 'form-data' || bodyType === 'multipart' ? (
        <div className="flex-1 overflow-y-auto">
          <MultipartEditor
            fields={parseMultipartFields(bodyFormData)}
            onChange={(fields) => onBodyFormDataChange(JSON.stringify(fields))}
          />
        </div>
      ) : bodyType !== 'none' ? (
        <div className="flex-1 border border-border rounded-lg overflow-hidden bg-background">
          <CodeEditor
            value={bodyType === 'json' ? bodyJson : bodyRaw}
            onChange={(val) => {
              if (bodyType === 'json') {
                onBodyJsonChange(val);
              } else {
                onBodyRawChange(val);
              }
            }}
            language={bodyType === 'json' ? 'json' : 'plaintext'}
          />
        </div>
      ) : (
        <div className="h-full flex flex-col items-center justify-center text-muted-foreground italic text-xs">
          <MingCuteIcon name="file_text_line" size={32} className="opacity-30 mb-1" />
          This request does not have a body.
        </div>
      )}
    </div>
  );
};
