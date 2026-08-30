import React, { useEffect, useState } from "react";
import { useInterceptStore } from "../../stores/useInterceptStore";
import { MingCuteIcon } from "../common/MingCuteIcon";
import { Dialog } from "../common/ui/Dialog";

export const InterceptViewer: React.FC = () => {
  const {
    interceptEnabled,
    interceptMode,
    pendingFlows,
    selectedFlowId,
    editedHeaders,
    editedBodyText,
    rules,
    initInterceptStore,
    setInterceptEnabled,
    setInterceptMode,
    selectFlow,
    setEditedHeaders,
    setEditedBodyText,
    forwardCurrentFlow,
    dropCurrentFlow,
    forwardAllFlows,
    dropAllFlows,
    addRule,
    toggleRule,
    deleteRule,
  } = useInterceptStore();

  const [activeTab, setActiveTab] = useState<"headers" | "body">("headers");
  const [rulesModalOpen, setRulesModalOpen] = useState(false);

  const [newRulePhase, setNewRulePhase] = useState<"request" | "response" | "both">("both");
  const [newRuleField, setNewRuleField] = useState<"url" | "host" | "path" | "method" | "header">("url");
  const [newRuleOperator, setNewRuleOperator] = useState<"contains" | "equals" | "regex">("contains");
  const [newRuleValue, setNewRuleValue] = useState("");

  const selectedFlow = pendingFlows.find((f) => f.flowId === selectedFlowId);

  useEffect(() => {
    initInterceptStore();
  }, []);

  const handleHeaderChange = (index: number, key: string, value: string) => {
    const updated: [string, string][] = [...editedHeaders];
    updated[index] = [key, value];
    setEditedHeaders(updated);
  };

  const handleAddHeader = () => {
    setEditedHeaders([...editedHeaders, ["", ""]]);
  };

  const handleRemoveHeader = (index: number) => {
    setEditedHeaders(editedHeaders.filter((_, i) => i !== index));
  };

  const handleCreateRule = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newRuleValue.trim()) return;
    await addRule({
      isEnabled: true,
      targetPhase: newRulePhase,
      matchField: newRuleField,
      operator: newRuleOperator,
      matchValue: newRuleValue.trim(),
      orderIndex: rules.length,
    });
    setNewRuleValue("");
  };

  return (
    <div className="flex flex-col h-full w-full bg-background text-foreground font-sans antialiased overflow-hidden select-none">
      {/* Top Header Controls */}
      <header className="flex items-center justify-between px-4 py-2.5 bg-header border-b border-border shrink-0">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="shield_line" size={18} className="text-amber-500" />
            <h1 className="text-sm font-bold tracking-tight text-foreground">
              Traffic Interceptor
            </h1>
          </div>

          <div className="h-4 w-px bg-border" />

          {/* Master Intercept Toggle Button */}
          <button
            onClick={() => setInterceptEnabled(!interceptEnabled)}
            className={`flex items-center gap-2 px-3 py-1 rounded-lg text-xs font-semibold tracking-wide transition-all shadow-2xs cursor-pointer ${
              interceptEnabled
                ? "bg-amber-500/20 text-amber-600 dark:text-amber-300 border border-amber-500/40"
                : "bg-surface text-muted-foreground border border-border hover:bg-neutral-subtle hover:text-foreground"
            }`}
          >
            <MingCuteIcon name="shield_line" size={14} className={interceptEnabled ? "text-amber-500 animate-pulse" : ""} />
            <span>Intercept is {interceptEnabled ? "ON" : "OFF"}</span>
          </button>

          {/* Intercept Phase Selector */}
          <div className="flex items-center bg-surface p-0.5 rounded-lg border border-border text-xs">
            {(["request", "response", "both"] as const).map((mode) => (
              <button
                key={mode}
                onClick={() => setInterceptMode(mode)}
                className={`px-2.5 py-0.5 rounded uppercase text-[11px] transition-colors cursor-pointer capitalize ${
                  interceptMode === mode
                    ? "bg-primary text-primary-foreground font-bold shadow-2xs"
                    : "text-muted-foreground hover:text-foreground hover:bg-neutral-subtle"
                }`}
              >
                {mode === "both" ? "Both" : `${mode}s`}
              </button>
            ))}
          </div>
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-2 font-mono">
          <button
            onClick={() => setRulesModalOpen(true)}
            className="flex items-center gap-1.5 px-3 py-1 bg-surface hover:bg-neutral-subtle text-foreground text-xs rounded-lg transition-colors border border-border font-medium cursor-pointer"
          >
            <MingCuteIcon name="tool_line" size={14} /> Rules ({rules.length})
          </button>

          <div className="h-4 w-px bg-border mx-1" />

          <button
            disabled={!selectedFlow}
            onClick={forwardCurrentFlow}
            className="flex items-center gap-1.5 px-3 py-1 bg-emerald-500/15 border border-emerald-500/30 hover:bg-emerald-500/25 text-emerald-600 dark:text-emerald-400 text-xs rounded-lg transition-colors font-semibold disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
          >
            <MingCuteIcon name="play_line" size={14} /> Forward
          </button>

          <button
            disabled={!selectedFlow}
            onClick={dropCurrentFlow}
            className="flex items-center gap-1.5 px-3 py-1 bg-rose-500/15 border border-rose-500/30 hover:bg-rose-500/25 text-rose-600 dark:text-rose-400 text-xs rounded-lg transition-colors font-semibold disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
          >
            <MingCuteIcon name="close_line" size={14} /> Drop
          </button>

          <button
            disabled={pendingFlows.length === 0}
            onClick={forwardAllFlows}
            className="flex items-center gap-1.5 px-3 py-1 bg-sky-500/15 border border-sky-500/30 hover:bg-sky-500/25 text-sky-600 dark:text-sky-400 text-xs rounded-lg transition-colors font-medium disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
          >
            <MingCuteIcon name="fast_forward_line" size={14} /> Forward All
          </button>

          <button
            disabled={pendingFlows.length === 0}
            onClick={dropAllFlows}
            className="flex items-center gap-1.5 px-3 py-1 bg-surface hover:bg-neutral-subtle text-muted-foreground hover:text-foreground text-xs rounded-lg transition-colors font-medium disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer border border-border"
          >
            <MingCuteIcon name="delete_2_line" size={14} /> Drop All
          </button>
        </div>
      </header>

      {/* Main Split Pane Layout */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left Drawer: Paused Flow Queue */}
        <div className="w-80 flex flex-col border-r border-border bg-surface shrink-0">
          <div className="px-3 py-2 bg-header border-b border-border flex items-center justify-between text-xs font-semibold text-muted-foreground uppercase tracking-wider font-mono">
            <span>Paused Queue</span>
            <span className="px-2 py-0.5 rounded-full bg-amber-500/20 text-amber-600 dark:text-amber-300 text-[10px] font-mono font-bold">
              {pendingFlows.length}
            </span>
          </div>

          <div className="flex-1 overflow-y-auto divide-y divide-border/60">
            {pendingFlows.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full p-6 text-center text-muted-foreground">
                <MingCuteIcon name="shield_line" size={32} className="mb-2 opacity-40" />
                <p className="text-xs font-semibold">No traffic paused</p>
                <p className="text-[11px] text-muted-foreground mt-1">
                  Enable interceptor and send requests to capture flows.
                </p>
              </div>
            ) : (
              pendingFlows.map((flow) => {
                const isSelected = selectedFlowId === flow.flowId;
                return (
                  <div
                    key={flow.flowId}
                    onClick={() => selectFlow(flow.flowId)}
                    className={`p-3 cursor-pointer transition-colors border-l-2 ${
                      isSelected
                        ? "bg-amber-500/10 border-amber-500"
                        : "border-transparent hover:bg-neutral-subtle"
                    }`}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="px-1.5 py-0.5 rounded bg-header border border-border text-amber-600 dark:text-amber-400 font-mono text-[10px] font-bold">
                        {flow.method}
                      </span>
                      <span className="text-[10px] font-mono uppercase text-muted-foreground">
                        {flow.phase}
                      </span>
                    </div>
                    <div className="text-xs font-mono text-foreground truncate">{flow.url}</div>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* Right Pane: Headers & Body Payload Editor */}
        <div className="flex-1 flex flex-col bg-background min-w-0">
          {selectedFlow ? (
            <div className="flex flex-col h-full">
              {/* Flow Detail Bar */}
              <div className="px-4 py-2 bg-header border-b border-border flex items-center justify-between text-xs font-mono">
                <div className="flex items-center gap-2 truncate">
                  <span className="px-2 py-0.5 rounded bg-amber-500/20 text-amber-600 dark:text-amber-300 font-bold">
                    {selectedFlow.method}
                  </span>
                  <span className="text-foreground truncate font-semibold">{selectedFlow.url}</span>
                </div>
                <div className="text-muted-foreground shrink-0 text-[11px]">
                  Phase: <span className="uppercase text-amber-500 font-bold">{selectedFlow.phase}</span>
                </div>
              </div>

              {/* Payload Editor Tabs */}
              <div className="flex items-center gap-2 px-4 py-1.5 bg-surface border-b border-border font-mono">
                <button
                  onClick={() => setActiveTab("headers")}
                  className={`px-3 py-1 rounded text-xs font-medium transition-colors cursor-pointer ${
                    activeTab === "headers"
                      ? "bg-primary text-primary-foreground font-bold shadow-2xs"
                      : "text-muted-foreground hover:text-foreground hover:bg-neutral-subtle"
                  }`}
                >
                  Headers ({editedHeaders.length})
                </button>
                <button
                  onClick={() => setActiveTab("body")}
                  className={`px-3 py-1 rounded text-xs font-medium transition-colors cursor-pointer ${
                    activeTab === "body"
                      ? "bg-primary text-primary-foreground font-bold shadow-2xs"
                      : "text-muted-foreground hover:text-foreground hover:bg-neutral-subtle"
                  }`}
                >
                  Body Payload
                </button>
              </div>

              {/* Editor Workspace */}
              <div className="flex-1 overflow-y-auto p-4 font-mono text-xs">
                {activeTab === "headers" ? (
                  <div className="space-y-2">
                    <div className="flex items-center justify-between mb-2">
                      <span className="text-muted-foreground font-sans text-xs font-semibold">
                        Edit HTTP Headers
                      </span>
                      <button
                        onClick={handleAddHeader}
                        className="flex items-center gap-1 px-2.5 py-1 bg-surface hover:bg-neutral-subtle border border-border text-primary rounded text-xs transition-colors cursor-pointer font-sans font-semibold"
                      >
                        <MingCuteIcon name="plus_line" size={14} /> Add Header
                      </button>
                    </div>

                    {editedHeaders.map(([key, val], idx) => (
                      <div key={idx} className="flex items-center gap-2">
                        <input
                          type="text"
                          placeholder="Header Key"
                          value={key}
                          onChange={(e) => handleHeaderChange(idx, e.target.value, val)}
                          className="flex-1 bg-surface border border-border focus:border-primary rounded px-2.5 py-1.5 text-foreground outline-none font-mono text-xs"
                        />
                        <input
                          type="text"
                          placeholder="Header Value"
                          value={val}
                          onChange={(e) => handleHeaderChange(idx, key, e.target.value)}
                          className="flex-2 bg-surface border border-border focus:border-primary rounded px-2.5 py-1.5 text-foreground outline-none font-mono text-xs"
                        />
                        <button
                          onClick={() => handleRemoveHeader(idx)}
                          className="p-1.5 text-muted-foreground hover:text-rose-500 rounded transition-colors cursor-pointer"
                        >
                          <MingCuteIcon name="close_line" size={14} />
                        </button>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="flex flex-col h-full space-y-2">
                    <span className="text-muted-foreground font-sans text-xs font-semibold">
                      Edit Body Content
                    </span>
                    <textarea
                      value={editedBodyText}
                      onChange={(e) => setEditedBodyText(e.target.value)}
                      placeholder="Enter raw request/response body payload..."
                      className="flex-1 min-h-[350px] w-full bg-surface border border-border focus:border-primary rounded-lg p-3 text-foreground outline-none resize-none font-mono text-xs leading-relaxed"
                    />
                  </div>
                )}
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-6 text-center">
              <MingCuteIcon name="code_line" size={36} className="mb-2 opacity-30" />
              <p className="text-xs font-semibold">Select a paused flow from the left queue</p>
            </div>
          )}
        </div>
      </div>

      {/* Intercept Rules Modal */}
      <Dialog
        isOpen={rulesModalOpen}
        onClose={() => setRulesModalOpen(false)}
        title="Intercept Rules Engine"
        description="Configure whitelist and blacklist criteria to selectively pause and inspect traffic."
        size="xl"
      >
        {/* Add New Rule Form */}
        <form onSubmit={handleCreateRule} className="pb-3 border-b border-border space-y-3">
          <div className="text-xs font-semibold text-muted-foreground">Add New Matching Rule</div>
          <div className="grid grid-cols-4 gap-2 text-xs">
            <select
              value={newRulePhase}
              onChange={(e: any) => setNewRulePhase(e.target.value)}
              className="bg-background border border-border rounded px-2 py-1.5 text-foreground outline-none cursor-pointer"
            >
              <option value="both">Both Phases</option>
              <option value="request">Request</option>
              <option value="response">Response</option>
            </select>
            <select
              value={newRuleField}
              onChange={(e: any) => setNewRuleField(e.target.value)}
              className="bg-background border border-border rounded px-2 py-1.5 text-foreground outline-none cursor-pointer"
            >
              <option value="url">URL</option>
              <option value="host">Host</option>
              <option value="path">Path</option>
              <option value="method">Method</option>
              <option value="header">Header</option>
            </select>
            <select
              value={newRuleOperator}
              onChange={(e: any) => setNewRuleOperator(e.target.value)}
              className="bg-background border border-border rounded px-2 py-1.5 text-foreground outline-none cursor-pointer"
            >
              <option value="contains">Contains</option>
              <option value="equals">Equals</option>
              <option value="regex">Regex</option>
            </select>
            <button
              type="submit"
              className="flex items-center justify-center gap-1 bg-amber-500 hover:bg-amber-600 text-white font-semibold rounded px-3 py-1.5 transition-colors cursor-pointer"
            >
              <MingCuteIcon name="plus_line" size={14} /> Add Rule
            </button>
          </div>
          <input
            type="text"
            placeholder="Match pattern value (e.g. api.example.com or /v1/auth)..."
            value={newRuleValue}
            onChange={(e) => setNewRuleValue(e.target.value)}
            className="w-full bg-background border border-border focus:border-primary rounded px-3 py-1.5 text-xs text-foreground outline-none font-mono"
          />
        </form>

        {/* Existing Rules List */}
        <div className="overflow-y-auto py-1 space-y-2 text-xs max-h-80">
          {rules.length === 0 ? (
            <div className="text-center py-6 text-muted-foreground italic">
              No custom rules added. All traffic will be paused when interceptor is active.
            </div>
          ) : (
            rules.map((rule) => (
              <div
                key={rule.id}
                className="flex items-center justify-between p-2.5 bg-background rounded-lg border border-border"
              >
                <div className="flex items-center gap-2">
                  <input
                    type="checkbox"
                    checked={rule.isEnabled}
                    onChange={() => toggleRule(rule.id)}
                    className="rounded accent-amber-500 cursor-pointer"
                  />
                  <span className="font-mono text-amber-500 font-bold uppercase text-[10px]">
                    [{rule.targetPhase}]
                  </span>
                  <span className="font-mono text-foreground">
                    {rule.matchField} {rule.operator} "{rule.matchValue}"
                  </span>
                </div>
                <button
                  type="button"
                  onClick={() => deleteRule(rule.id)}
                  className="text-muted-foreground hover:text-rose-500 transition-colors p-1 cursor-pointer"
                >
                  <MingCuteIcon name="delete_2_line" size={14} />
                </button>
              </div>
            ))
          )}
        </div>
      </Dialog>
    </div>
  );
};
