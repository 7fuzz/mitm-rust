import React from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { Input, Select, TriStateFilter, Checkbox, Button, TriState } from '../../common/ui';

const METHOD_OPTIONS = [
  { value: 'GET', label: 'GET' },
  { value: 'POST', label: 'POST' },
  { value: 'PUT', label: 'PUT' },
  { value: 'DELETE', label: 'DELETE' },
  { value: 'PATCH', label: 'PATCH' },
  { value: 'OPTIONS', label: 'OPTIONS' },
] as const;

const STATUS_OPTIONS = [
  { value: 'all', label: 'All Status Codes' },
  { value: '2xx', label: '2xx Success' },
  { value: '3xx', label: '3xx Redirection' },
  { value: '4xx', label: '4xx Client Error' },
  { value: '5xx', label: '5xx Server Error' },
] as const;

export const TrafficFilterBar: React.FC = () => {
  const {
    searchQuery,
    setSearchQuery,
    methodFilters,
    setMethodFilter,
    statusCodeRange,
    setStatusCodeRange,
    onlyIntercepted,
    setOnlyIntercepted,
    clearTraffic,
  } = useProxyStore();

  const handleTriStateChange = (method: string, nextState: TriState) => {
    setMethodFilter(method, nextState);
  };

  return (
    <div className="p-2 bg-header border-b border-border flex items-center justify-between gap-3 text-xs shrink-0 select-none">
      {/* Left Filters */}
      <div className="flex items-center gap-2 flex-1 overflow-x-auto no-scrollbar">
        {/* Live Keyword Search Input */}
        <div className="min-w-[220px] max-w-xs">
          <Input
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            placeholder="Search host, path, status, header..."
            leftIcon="search_line"
            rightIcon={searchQuery ? 'close_line' : undefined}
          />
        </div>

        {/* Tri-State Method Filter (Neutral -> Whitelist Green -> Blacklist Red) */}
        <TriStateFilter
          items={METHOD_OPTIONS}
          values={methodFilters}
          onChange={handleTriStateChange}
        />

        {/* Status Code Range Atomic Select */}
        <Select
          value={statusCodeRange}
          onChange={(e) => setStatusCodeRange(e.target.value as any)}
          options={STATUS_OPTIONS}
        />

        {/* Intercepted Only Checkbox */}
        <div className="px-2 py-1 bg-surface border border-border rounded shadow-2xs">
          <Checkbox
            label="Intercepted Only"
            checked={onlyIntercepted}
            onChange={(e) => setOnlyIntercepted(e.target.checked)}
          />
        </div>
      </div>

      {/* Right Controls: Clear Logs Atomic Button */}
      <div className="flex items-center gap-2">
        <Button
          variant="ghost"
          icon="delete_2_line"
          onClick={clearTraffic}
          className="text-muted-foreground hover:text-rose-500"
          title="Clear traffic log"
        >
          Clear Logs
        </Button>
      </div>
    </div>
  );
};
