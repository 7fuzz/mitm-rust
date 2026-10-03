import { create } from 'zustand';

export type UtilityTab = 'cvss' | 'json' | 'encoding' | 'hashing';

export type EncodingAlgorithm =
  | 'base64'
  | 'base64url'
  | 'url'
  | 'hex'
  | 'html'
  | 'binary'
  | 'jwt';

export type HashAlgorithm =
  | 'bcrypt'
  | 'sha256'
  | 'md5'
  | 'sha1'
  | 'sha384'
  | 'sha512'
  | 'hmac-sha256'
  | 'hmac-md5';

export type JsonMode = 'format' | 'diff';
export type JsonOperation = 'pretty' | 'minify' | 'escape' | 'unescape';
export type JsonIndent = '2' | '4' | 'tab';

export type Direction = 'encode' | 'decode';
export type HashMode = 'generate' | 'verify';

interface UtilitiesState {
  activeTab: UtilityTab;
  setActiveTab: (tab: UtilityTab) => void;

  // JSON
  jsonMode: JsonMode;
  setJsonMode: (mode: JsonMode) => void;
  jsonInput: string;
  setJsonInput: (input: string) => void;
  jsonOperation: JsonOperation;
  setJsonOperation: (op: JsonOperation) => void;
  jsonIndent: JsonIndent;
  setJsonIndent: (indent: JsonIndent) => void;
  jsonSortKeys: boolean;
  setJsonSortKeys: (sort: boolean) => void;
  jsonDiffLeft: string;
  setJsonDiffLeft: (text: string) => void;
  jsonDiffRight: string;
  setJsonDiffRight: (text: string) => void;
  jsonDiffInline: boolean;
  setJsonDiffInline: (inline: boolean) => void;

  // Encoding & Decoding
  encodingInput: string;
  setEncodingInput: (input: string) => void;
  encodingAlgorithm: EncodingAlgorithm;
  setEncodingAlgorithm: (algo: EncodingAlgorithm) => void;
  encodingDirection: Direction;
  setEncodingDirection: (dir: Direction) => void;
  encodingHexDelimiter: '' | ' ' | ':' | '\\x' | '0x';
  setEncodingHexDelimiter: (del: '' | ' ' | ':' | '\\x' | '0x') => void;
  encodingHexCase: 'lower' | 'upper';
  setEncodingHexCase: (c: 'lower' | 'upper') => void;
  encodingUrlFull: boolean;
  setEncodingUrlFull: (full: boolean) => void;

  // Hashing & Bcrypt
  hashingInput: string;
  setHashingInput: (input: string) => void;
  hashingAlgorithm: HashAlgorithm;
  setHashingAlgorithm: (algo: HashAlgorithm) => void;
  hashingMode: HashMode;
  setHashingMode: (mode: HashMode) => void;
  hashingVerifyTarget: string;
  setHashingVerifyTarget: (target: string) => void;
  hashingBcryptRounds: number;
  setHashingBcryptRounds: (rounds: number) => void;
  hashingBcryptSalt: string;
  setHashingBcryptSalt: (salt: string) => void;
  hashingCase: 'lower' | 'upper';
  setHashingCase: (c: 'lower' | 'upper') => void;
  hashingHmacKey: string;
  setHashingHmacKey: (key: string) => void;
}

export const useUtilitiesStore = create<UtilitiesState>((set) => ({
  activeTab: 'encoding',
  setActiveTab: (activeTab) => set({ activeTab }),

  jsonMode: 'format',
  setJsonMode: (jsonMode) => set({ jsonMode }),
  jsonInput: '',
  setJsonInput: (jsonInput) => set({ jsonInput }),
  jsonOperation: 'pretty',
  setJsonOperation: (jsonOperation) => set({ jsonOperation }),
  jsonIndent: '2',
  setJsonIndent: (jsonIndent) => set({ jsonIndent }),
  jsonSortKeys: false,
  setJsonSortKeys: (jsonSortKeys) => set({ jsonSortKeys }),
  jsonDiffLeft: '',
  setJsonDiffLeft: (jsonDiffLeft) => set({ jsonDiffLeft }),
  jsonDiffRight: '',
  setJsonDiffRight: (jsonDiffRight) => set({ jsonDiffRight }),
  jsonDiffInline: false,
  setJsonDiffInline: (jsonDiffInline) => set({ jsonDiffInline }),

  // Encoding state: initial empty string
  encodingInput: '',
  setEncodingInput: (encodingInput) => set({ encodingInput }),
  encodingAlgorithm: 'base64',
  setEncodingAlgorithm: (encodingAlgorithm) => set({ encodingAlgorithm }),
  encodingDirection: 'encode',
  setEncodingDirection: (encodingDirection) => set({ encodingDirection }),
  encodingHexDelimiter: ' ',
  setEncodingHexDelimiter: (encodingHexDelimiter) => set({ encodingHexDelimiter }),
  encodingHexCase: 'lower',
  setEncodingHexCase: (encodingHexCase) => set({ encodingHexCase }),
  encodingUrlFull: false,
  setEncodingUrlFull: (encodingUrlFull) => set({ encodingUrlFull }),

  // Hashing state: initial empty string
  hashingInput: '',
  setHashingInput: (hashingInput) => set({ hashingInput }),
  hashingAlgorithm: 'bcrypt',
  setHashingAlgorithm: (hashingAlgorithm) => set({ hashingAlgorithm }),
  hashingMode: 'generate',
  setHashingMode: (hashingMode) => set({ hashingMode }),
  hashingVerifyTarget: '',
  setHashingVerifyTarget: (hashingVerifyTarget) => set({ hashingVerifyTarget }),
  hashingBcryptRounds: 10,
  setHashingBcryptRounds: (hashingBcryptRounds) => set({ hashingBcryptRounds }),
  hashingBcryptSalt: '',
  setHashingBcryptSalt: (hashingBcryptSalt) => set({ hashingBcryptSalt }),
  hashingCase: 'lower',
  setHashingCase: (hashingCase) => set({ hashingCase }),
  hashingHmacKey: 'secret-key',
  setHashingHmacKey: (hashingHmacKey) => set({ hashingHmacKey }),
}));
