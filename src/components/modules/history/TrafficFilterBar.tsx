import React, { useState, useEffect, useRef } from 'react';
import { useProxyStore } from '../../../stores/useProxyStore';
import { Input, TriStateFilter, Button, TriState } from '../../common/ui';
import { MingCuteIcon } from '../../common/MingCuteIcon';
import { useUiPref } from '../../../stores/useUiPrefsStore';

const METHOD_OPTIONS = [
  { value: 'GET', label: 'GET' },
  { value: 'POST', label: 'POST' },
  { value: 'PUT', label: 'PUT' },
  { value: 'DELETE', label: 'DELETE' },
  { value: 'PATCH', label: 'PATCH' },
  { value: 'OPTIONS', label: 'OPTIONS' },
] as const;

const STATUS_OPTIONS = [
  { value: '2xx', label: '2xx' },
  { value: '3xx', label: '3xx' },
  { value: '4xx', label: '4xx' },
  { value: '5xx', label: '5xx' },
] as const;

const FLAG_OPTIONS = [
  { value: 'waiting', label: 'Waiting' },
  { value: 'intercepted', label: 'Intercepted' },
  { value: 'rewritten', label: 'Rewritten' },
  { value: 'failed', label: 'Failed' },
] as const;

const LIMIT_PRESETS = [100, 250, 500, 1000, 2500, 5000, 10000];

const FilterGroup: React.FC<{ label: string; children: React.ReactNode }> = ({ label, children }) => (
  <div className="flex items-center gap-1.5">
    <span className="text-3xs uppercase tracking-wider font-semibold text-muted-foreground">{label}</span>
    {children}
  </div>
);

export const TrafficFilterBar: React.FC = () => {
  const {
    searchQuery,
    setSearchQuery,
    methodFilters,
    setMethodFilter,
    statusFilters,
    setStatusFilter,
    flagFilters,
    setFlagFilter,
    clearTraffic,
    historySettings,
    updateHistorySettings,
    traffic,
    totalDbCount,
    listeners,
    listenerFilter,
    setListenerFilter,
    resetFilters,
  } = useProxyStore();

  const [localSearch, setLocalSearch] = useState(searchQuery);
  const [filtersOpen, setFiltersOpen] = useUiPref('history.filterBarOpen');
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

  const handleStatusTriStateChange = (status: string, nextState: TriState) => {
    setStatusFilter(status, nextState);
  };

  const handleFlagTriStateChange = (flag: string, nextState: TriState) => {
    setFlagFilter(flag, nextState);
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

  const countActive = (filters: Record<string, TriState>) =>
    Object.values(filters).filter((st) => st !== 'neutral').length;
  const activeFilterCount =
    countActive(methodFilters) + countActive(statusFilters) + countActive(flagFilters) + (listenerFilter ? 1 : 0);

  return (
    <div className="bg-header border-b border-border text-xs shrink-0 select-none">
      <div className="p-2 flex items-center justify-between gap-3">
        {/* Left: Search & Filters Toggle */}
        <div className="flex items-center gap-2 flex-1 min-w-0">
          {/* Keyword Search Input */}
          <div className="w-full max-w-md min-w-[180px]">
            <Input
              value={localSearch}
              onChange={(e) => setLocalSearch(e.target.value)}
              placeholder="Search host, path, status, header..."
              leftIcon="search_line"
              rightIcon={localSearch ? 'close_line' : undefined}
              onRightIconClick={handleClearSearch}
            />
          </div>

          <button
            onClick={() => setFiltersOpen(!filtersOpen)}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded border transition-colors cursor-pointer text-xs font-medium shrink-0 ${
              filtersOpen || activeFilterCount > 0
                ? 'bg-primary/10 border-primary/40 text-primary'
                : 'bg-surface border-border text-muted-foreground hover:text-foreground hover:bg-neutral-subtle'
            }`}
            title={filtersOpen ? 'Hide filters' : 'Show filters'}
          >
            <MingCuteIcon name="filter_line" size={13} />
            <span>Filters{activeFilterCount > 0 ? ` (${activeFilterCount})` : ''}</span>
            <MingCuteIcon name={filtersOpen ? 'up_line' : 'down_line'} size={12} className="opacity-60" />
          </button>

          <span
            className="text-2xs font-mono text-muted-foreground tabular-nums whitespace-nowrap shrink-0"
            title={`${traffic.length.toLocaleString()} loaded in the table`}
          >
            <span className="text-foreground font-semibold">{totalDbCount.toLocaleString()}</span>{' '}
            {activeFilterCount > 0 || searchQuery ? 'matching' : totalDbCount === 1 ? 'request' : 'requests'}
          </span>
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
                  <span className="text-2xs font-mono text-muted-foreground">
                    Total: {totalDbCount.toLocaleString()}
                  </span>
                </div>

                {/* Enable / Disable Limiter Switch */}
                <div className="flex items-center justify-between bg-header p-2 rounded-lg border border-border">
                  <div className="flex flex-col">
                    <span className="font-medium text-foreground">Limit Max Capacity</span>
                    <span className="text-3xs text-muted-foreground">Delete oldest logs when full</span>
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
                    <label className="text-2xs text-muted-foreground font-mono flex items-center justify-between">
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
                          className={`px-2 py-0.5 rounded text-3xs font-mono transition-colors cursor-pointer ${
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
            onClick={() => clearTraffic().catch(() => {})}
            className="text-muted-foreground hover:text-rose-500"
            title="Clear all traffic logs"
          />
        </div>
      </div>

      {/* Collapsible Filter Row */}
      {filtersOpen && (
        <div className="px-2 pb-2 flex items-center gap-3 flex-wrap">
          <FilterGroup label="Method">
            <TriStateFilter items={METHOD_OPTIONS} values={methodFilters} onChange={handleTriStateChange} />
          </FilterGroup>

          <div className="h-4 w-px bg-border" />

          <FilterGroup label="Status">
            <TriStateFilter items={STATUS_OPTIONS} values={statusFilters} onChange={handleStatusTriStateChange} />
          </FilterGroup>

          <div className="h-4 w-px bg-border" />

          <FilterGroup label="Flags">
            <TriStateFilter items={FLAG_OPTIONS} values={flagFilters} onChange={handleFlagTriStateChange} />
          </FilterGroup>

          {/* Source / Listener Filter (Visible when multiple listeners exist) */}
          {listeners && listeners.length > 1 && (
            <>
              <div className="h-4 w-px bg-border" />
              <FilterGroup label="Source">
                <select
                  value={listenerFilter || ''}
                  onChange={(e) => setListenerFilter(e.target.value)}
                  className="bg-surface border border-border rounded px-2 py-1 text-foreground text-xs font-mono focus:outline-none cursor-pointer shadow-2xs"
                  title="Filter traffic by source listener"
                >
                  <option value="">All Sources</option>
                  {listeners.map((l) => (
                    <option key={l.id} value={l.label}>
                      {l.label} ({l.address})
                    </option>
                  ))}
                </select>
              </FilterGroup>
            </>
          )}

          {activeFilterCount > 0 && (
            <button
              onClick={resetFilters}
              className="ml-auto text-2xs text-muted-foreground hover:text-foreground underline cursor-pointer"
            >
              Reset filters
            </button>
          )}
        </div>
      )}
    </div>
  );
};
