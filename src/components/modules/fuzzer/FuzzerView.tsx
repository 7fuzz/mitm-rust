import React from 'react';
import { MingCuteIcon } from '../../common/MingCuteIcon';

export const FuzzerView: React.FC = () => (
  <div className="h-full flex flex-col items-center justify-center gap-3 bg-background text-xs p-6 text-center">
    <MingCuteIcon name="fast_forward_line" size={36} className="text-muted-foreground opacity-30" />
    <div className="space-y-1">
      <div className="text-sm font-semibold text-foreground">Fuzzer</div>
      <p className="text-muted-foreground max-w-sm leading-relaxed">
        Send one request many times with varying payloads, then compare the responses. Not built yet.
      </p>
    </div>
  </div>
);
