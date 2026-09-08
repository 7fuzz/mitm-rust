import React, { useState, useEffect, useRef } from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { Input, Select, TriStateFilter, Checkbox, Button, TriState } from '../../common/ui';
import { MingCuteIcon } from '../../common/MingCuteIcon';

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

const LIMIT_PRESETS = [100, 250, 500, 1000, 2500, 5000, 10000];

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
    onlyRewritten,
    setOnlyRewritten,
    onlyFailed,
    setOnlyFailed,
    clearTraffic,
    historySettings,
    updateHistorySettings,
    traffic,
    totalDbCount,
  } = useProxyStore();

  const [localSearch, setLocalSearch] = useState(searchQuery);
  const [popoverOpen, setPopoverOpen] = useState(false);
  const [localEnabled, setLocalEnabled] = useState(historySettings.limiterEnabled);
  const [localMaxRows, setLocalMaxRows] = useState(historySettings.maxRows);
  const [isApplying, setIsApplying] = useState(false);
  const popoverRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    setLocalSearch(searchQuery);
  }, [searchQuery]);

  useEffect(() => {
    setLocalEnabled(historySettings.limiterEnabled);
    setLocalMaxRows(historySettings.maxRows);
  }, [historySettings]);

  useEffect(() => {
    const timer = setTimeout(() => {
      setSearchQuery(localSearch);
    }, 250);
    return () => clearTimeout(timer);
  }, [localSearch, setSearchQuery]);

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setPopoverOpen(false);
      }
    };
    document.addEventListener('mousedown', handleClickOutside);
    return () => document.removeEventListener('mousedown', handleClickOutside);
  }, []);

  const handleTriStateChange = (method: string, nextState: TriState) => {
    setMethodFilter(method, nextState);
  };

  const handleClearSearch = () => {
    setLocalSearch('');
    setSearchQuery('');
  };

  const handleApplySettings = async () => {
    setIsApplying(true);
    await updateHistorySettings(localEnabled, localMaxRows);
    setIsApplying(false);
    setPopoverOpen(false);
  };

  return (
    <div className="p-2 bg-header border-b border-border flex items-center justify-between gap-3 text-xs shrink-0 select-none">
      {/* Left Filters */}
      <div className="flex items-center gap-2 flex-1 overflow-x-auto no-scrollbar">
        {/* Keyword Search Input */}
        <div className="min-w-[220px] max-w-xs">
          <Input
            value={localSearch}
            onChange={(e) => setLocalSearch(e.target.value)}
            placeholder="Search host, path, status, header..."
            leftIcon="search_line"
            rightIcon={localSearch ? 'close_line' : undefined}
            onRightIconClick={handleClearSearch}
          />
        </div>

        {/* Tri-State Method Filter */}
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
            label="Intercepted"
            checked={onlyIntercepted}
            onChange={(e) => setOnlyIntercepted(e.target.checked)}
          />
        </div>

        {/* Rewritten Only Checkbox */}
        <div className="px-2 py-1 bg-surface border border-border rounded shadow-2xs">
          <Checkbox
            label="Rewritten"
            checked={onlyRewritten}
            onChange={(e) => setOnlyRewritten(e.target.checked)}
          />
        </div>

        {/* Failed Only Checkbox */}
        <div className="px-2 py-1 bg-surface border border-border rounded shadow-2xs">
          <Checkbox
            label="Failed"
            checked={onlyFailed}
            onChange={(e) => setOnlyFailed(e.target.checked)}
          />
        </div>
      </div>

      {/* Right Controls: Limit Settings & Clear Logs */}
      <div className="flex items-center gap-2">
        {/* History Limit Settings Trigger & Popover */}
        <div className="relative" ref={popoverRef}>
          <button
            onClick={() => setPopoverOpen(!popoverOpen)}
            className="flex items-center gap-1.5 px-2.5 py-1 rounded bg-surface hover:bg-neutral-subtle border border-border text-muted-foreground hover:text-foreground transition-colors cursor-pointer text-xs font-mono"
            title="Configure History Log Limits & Auto-Pruning"
          >
            <MingCuteIcon name="settings_3_line" size={14} className="text-primary" />
            <span>
              Limit: {historySettings.limiterEnabled ? `${historySettings.maxRows} logs` : 'Off'}
            </span>
            <MingCuteIcon name="down_line" size={12} className="opacity-60" />
          </button>

          {popoverOpen && (
            <div className="absolute right-0 mt-2 w-72 bg-surface border border-border rounded-xl shadow-2xl p-3.5 z-50 font-sans text-xs text-foreground flex flex-col gap-3">
              <div className="flex items-center justify-between border-b border-border pb-2">
                <span className="font-semibold text-foreground flex items-center gap-1.5">
                  <MingCuteIcon name="history_line" size={15} className="text-primary" />
                  History Log Limit
                </span>
                <span className="text-[11px] font-mono text-muted-foreground">
                  Total: {totalDbCount > 0 ? totalDbCount : traffic.length}
                </span>
              </div>

              {/* Enable / Disable Limiter Switch */}
              <div className="flex items-center justify-between bg-header p-2 rounded-lg border border-border">
                <div className="flex flex-col">
                  <span className="font-medium text-foreground">Limit Max Capacity</span>
                  <span className="text-[10px] text-muted-foreground">Delete oldest logs when full</span>
                </div>
                <input
                  type="checkbox"
                  checked={localEnabled}
                  onChange={(e) => setLocalEnabled(e.target.checked)}
                  className="accent-primary h-4 w-4 rounded cursor-pointer"
                />
              </div>

              {/* Max Rows Input & Presets */}
              {localEnabled && (
                <div className="flex flex-col gap-1.5">
                  <label className="text-[11px] text-muted-foreground font-mono flex items-center justify-between">
                    <span>Max Rows Limit:</span>
                    <strong className="text-primary font-bold">{localMaxRows} rows</strong>
                  </label>
                  <input
                    type="number"
                    min={10}
                    max={50000}
                    value={localMaxRows}
                    onChange={(e) => setLocalMaxRows(Math.max(10, parseInt(e.target.value) || 100))}
                    className="w-full bg-background border border-border rounded px-2.5 py-1 font-mono text-xs text-foreground focus:outline-none focus:border-primary"
                  />

                  {/* Quick Preset Buttons */}
                  <div className="flex items-center gap-1 flex-wrap mt-1">
                    {LIMIT_PRESETS.map((preset) => (
                      <button
                        key={preset}
                        onClick={() => setLocalMaxRows(preset)}
                        className={`px-2 py-0.5 rounded text-[10px] font-mono transition-colors cursor-pointer ${
                          localMaxRows === preset
                            ? 'bg-primary text-primary-foreground font-bold shadow-2xs'
                            : 'bg-header text-muted-foreground hover:text-foreground hover:bg-neutral-subtle border border-border/60'
                        }`}
                      >
                        {preset}
                      </button>
                    ))}
                  </div>
                </div>
              )}

              {/* Action Buttons */}
              <div className="flex items-center justify-end gap-2 pt-2 border-t border-border">
                <button
                  onClick={() => setPopoverOpen(false)}
                  className="px-3 py-1 rounded bg-header hover:bg-neutral-subtle text-muted-foreground hover:text-foreground border border-border transition-colors cursor-pointer text-xs font-medium"
                >
                  Cancel
                </button>
                <button
                  onClick={handleApplySettings}
                  disabled={isApplying}
                  className="px-3 py-1 rounded bg-primary hover:bg-primary-hover text-primary-foreground font-semibold transition-colors cursor-pointer text-xs shadow-2xs disabled:opacity-50 flex items-center gap-1"
                >
                  <MingCuteIcon name="delete_2_line" size={13} />
                  <span>{isApplying ? 'Saving...' : 'Apply & Delete Old'}</span>
                </button>
              </div>
            </div>
          )}
        </div>

        {/* Clear Logs Button */}
        <Button
          variant="ghost"
          icon="delete_2_line"
          onClick={clearTraffic}
          className="text-muted-foreground hover:text-rose-500"
          title="Clear all traffic logs"
        >
          Clear Logs
        </Button>
      </div>
    </div>
  );
};
