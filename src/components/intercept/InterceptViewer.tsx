import React, { useEffect, useState, useMemo } from "react";
import { useInterceptStore } from "../../stores/useInterceptStore";
import { MingCuteIcon } from "../common/MingCuteIcon";
import { Dialog } from "../common/ui/Dialog";
import { Select } from "../common/ui";
import { KeyValueEditor } from "../common/KeyValueEditor";

const METHOD_OPTIONS = [
  { value: "GET", label: "GET" },
  { value: "POST", label: "POST" },
  { value: "PUT", label: "PUT" },
  { value: "DELETE", label: "DELETE" },
  { value: "PATCH", label: "PATCH" },
  { value: "OPTIONS", label: "OPTIONS" },
  { value: "HEAD", label: "HEAD" },
] as const;

export const InterceptViewer: React.FC = () => {
  const {
    interceptEnabled,
    interceptMode,
    focusOnIntercepted,
    pendingFlows,
    selectedFlowId,
    editedMethod,
    editedUrl,
    editedParams,
    editedHeaders,
    editedBodyText,
    rules,
    initInterceptStore,
    setInterceptEnabled,
    setInterceptMode,
    setFocusOnIntercepted,
    selectFlow,
    setEditedMethod,
    setEditedUrl,
    setEditedParams,
    setEditedHeaders,
    setEditedBodyText,
    forwardCurrentFlow,
    dropCurrentFlow,
    forwardAllFlows,
    dropAllFlows,
    addRule,
    updateRule,
    toggleRule,
    updateRuleAction,
    deleteRule,
  } = useInterceptStore();

  const [activeTab, setActiveTab] = useState<"params" | "headers" | "body">("params");
  const [rulesModalOpen, setRulesModalOpen] = useState(false);

  const [editingRuleId, setEditingRuleId] = useState<string | null>(null);
  const [newRuleAction, setNewRuleAction] = useState<"intercept" | "pass">("intercept");
  const [newRulePhase, setNewRulePhase] = useState<"request" | "response" | "both">("request");
  const [newRuleField, setNewRuleField] = useState<"url" | "host" | "path" | "method" | "header">("url");
  const [newRuleOperator, setNewRuleOperator] = useState<"contains" | "equals" | "regex">("contains");
  const [newRuleValue, setNewRuleValue] = useState("");
  const [ruleFilterTab, setRuleFilterTab] = useState<"all" | "intercept" | "pass">("all");

  const selectedFlow = pendingFlows.find((f) => f.flowId === selectedFlowId);
  const isRequestPhase = selectedFlow?.phase === "request";

  useEffect(() => {
    initInterceptStore();
  }, []);

  // When switching between request and response, ensure valid active tab
  useEffect(() => {
    if (!isRequestPhase && activeTab === "params") {
      setActiveTab("headers");
    }
  }, [isRequestPhase, activeTab]);

  // Keyboard shortcut: Cmd/Ctrl + Enter to forward
  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
        if (!rulesModalOpen && selectedFlow) {
          e.preventDefault();
          forwardCurrentFlow();
        }
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [rulesModalOpen, selectedFlow, forwardCurrentFlow]);

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

  const handleStartEditRule = (rule: any) => {
    setEditingRuleId(rule.id);
    setNewRuleAction(rule.action === "pass" ? "pass" : "intercept");
    setNewRulePhase(rule.targetPhase);
    setNewRuleField(rule.matchField);
    setNewRuleOperator(rule.operator);
    setNewRuleValue(rule.matchValue);
  };

  const handleCancelEditRule = () => {
    setEditingRuleId(null);
    setNewRuleAction("intercept");
    setNewRulePhase("request");
    setNewRuleField("url");
    setNewRuleOperator("contains");
    setNewRuleValue("");
  };

  const handleCreateRule = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newRuleValue.trim()) return;

    if (editingRuleId) {
      const existing = rules.find((r) => r.id === editingRuleId);
      if (existing) {
        await updateRule({
          ...existing,
          targetPhase: newRulePhase,
          matchField: newRuleField,
          operator: newRuleOperator,
          matchValue: newRuleValue.trim(),
          action: newRuleAction,
        });
      }
      setEditingRuleId(null);
    } else {
      await addRule({
        isEnabled: true,
        targetPhase: newRulePhase,
        matchField: newRuleField,
        operator: newRuleOperator,
        matchValue: newRuleValue.trim(),
        action: newRuleAction,
        orderIndex: rules.length,
      });
    }
    setNewRuleValue("");
  };

  const activeParamsCount = editedParams.filter((p) => p.enabled && p.key.trim()).length;

  const handlePrettifyBody = () => {
    try {
      const parsed = JSON.parse(editedBodyText.trim());
      setEditedBodyText(JSON.stringify(parsed, null, 2));
    } catch {
      // Not valid JSON, keep as is
    }
  };

  const isJsonBody = useMemo(() => {
    const trimmed = editedBodyText.trim();
    if (
      (trimmed.startsWith("{") && trimmed.endsWith("}")) ||
      (trimmed.startsWith("[") && trimmed.endsWith("]"))
    ) {
      try {
        JSON.parse(trimmed);
        return true;
      } catch {
        return false;
      }
    }
    return false;
  }, [editedBodyText]);

  const filteredRules = rules.filter((rule) => {
    if (ruleFilterTab === "intercept") return rule.action !== "pass";
    if (ruleFilterTab === "pass") return rule.action === "pass";
    return true;
  });

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

          {/* Focus on Intercepted Toggle Option */}
          <label
            className="flex items-center gap-1.5 px-2.5 py-1 bg-surface rounded-lg border border-border text-xs text-foreground cursor-pointer hover:bg-neutral-subtle transition-colors select-none"
            title="Automatically bring MITM window to front and select intercepted request when traffic is paused"
          >
            <input
              type="checkbox"
              checked={focusOnIntercepted}
              onChange={(e) => setFocusOnIntercepted(e.target.checked)}
              className="rounded accent-primary text-primary cursor-pointer w-3.5 h-3.5"
            />
            <span className="font-medium text-[11px] whitespace-nowrap">Focus on Intercepted</span>
          </label>
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
            title="Forward modified flow (⌘↵)"
          >
            <MingCuteIcon name="play_line" size={14} /> Forward (⌘↵)
          </button>

          <button
            disabled={!selectedFlow}
            onClick={dropCurrentFlow}
            className="flex items-center gap-1.5 px-3 py-1 bg-rose-500/15 border border-rose-500/30 hover:bg-rose-500/25 text-rose-600 dark:text-rose-400 text-xs rounded-lg transition-colors font-semibold disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
            title="Drop flow and return 502"
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
                        {isSelected ? editedMethod : flow.method}
                      </span>
                      <span className="text-[10px] font-mono uppercase text-muted-foreground">
                        {flow.phase}
                      </span>
                    </div>
                    <div className="text-xs font-mono text-foreground truncate">
                      {isSelected ? editedUrl : flow.url}
                    </div>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* Right Pane: Editable Method/URL Bar, Params, Headers & Body */}
        <div className="flex-1 flex flex-col bg-background min-w-0">
          {selectedFlow ? (
            <div className="flex flex-col h-full">
              {/* Editable Address Bar for Intercepted Flow */}
              <div className="p-2.5 bg-header border-b border-border flex items-center gap-2 shrink-0 font-mono">
                {isRequestPhase ? (
                  <>
                    <div className="w-28 shrink-0 font-bold">
                      <Select
                        value={editedMethod}
                        onChange={(e) => setEditedMethod(e.target.value)}
                        options={METHOD_OPTIONS}
                        sizeVariant="sm"
                      />
                    </div>
                    <div className="flex-1 relative flex items-center">
                      <input
                        type="text"
                        value={editedUrl}
                        onChange={(e) => setEditedUrl(e.target.value)}
                        placeholder="https://api.example.com/endpoint?param=value"
                        className="w-full bg-background border border-border rounded px-3 py-1.5 font-mono text-xs text-foreground focus:outline-none focus:border-primary shadow-2xs"
                      />
                    </div>
                    <span className="px-2 py-1 rounded bg-amber-500/20 text-amber-600 dark:text-amber-300 font-bold text-[10px] uppercase shrink-0">
                      REQUEST
                    </span>
                  </>
                ) : (
                  <div className="flex items-center justify-between w-full text-xs">
                    <div className="flex items-center gap-2 truncate">
                      <span className="px-2 py-0.5 rounded bg-purple-500/20 text-purple-600 dark:text-purple-300 font-bold">
                        {selectedFlow.method}
                      </span>
                      <span className="text-foreground truncate font-semibold">{selectedFlow.url}</span>
                    </div>
                    <span className="px-2 py-1 rounded bg-purple-500/20 text-purple-600 dark:text-purple-300 font-bold text-[10px] uppercase shrink-0">
                      RESPONSE
                    </span>
                  </div>
                )}
              </div>

              {/* Payload Editor Tabs */}
              <div className="flex items-center gap-2 px-4 py-1.5 bg-surface border-b border-border font-mono text-xs">
                {isRequestPhase && (
                  <button
                    onClick={() => setActiveTab("params")}
                    className={`px-3 py-1 rounded text-xs font-medium transition-colors cursor-pointer ${
                      activeTab === "params"
                        ? "bg-primary text-primary-foreground font-bold shadow-2xs"
                        : "text-muted-foreground hover:text-foreground hover:bg-neutral-subtle"
                    }`}
                  >
                    Params {activeParamsCount > 0 ? `(${activeParamsCount})` : ""}
                  </button>
                )}
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
                {activeTab === "params" && isRequestPhase ? (
                  <div className="space-y-3">
                    <div className="flex items-center justify-between">
                      <span className="text-muted-foreground font-sans text-xs font-semibold">
                        Query Parameters (Synced with URL)
                      </span>
                    </div>
                    <KeyValueEditor
                      items={editedParams}
                      onChange={(params) => setEditedParams(params)}
                      keyPlaceholder="Parameter Key"
                      valuePlaceholder="Parameter Value"
                    />
                  </div>
                ) : activeTab === "headers" ? (
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
                    <div className="flex items-center justify-between">
                      <div className="flex items-center gap-2">
                        <span className="text-muted-foreground font-sans text-xs font-semibold">
                          Edit Body Content
                        </span>
                        {isJsonBody && (
                          <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-bold bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30">
                            JSON
                          </span>
                        )}
                      </div>
                      <button
                        type="button"
                        onClick={handlePrettifyBody}
                        className="flex items-center gap-1.5 px-2.5 py-1 bg-surface hover:bg-neutral-subtle border border-border text-foreground rounded text-xs transition-colors cursor-pointer font-sans font-medium"
                        title="Format / Prettify JSON with 2-space indentation"
                      >
                        <MingCuteIcon name="code_line" size={13} className="text-amber-500" />
                        <span>Prettify JSON</span>
                      </button>
                    </div>
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
        {/* Add/Edit Rule Form */}
        <form onSubmit={handleCreateRule} className="pb-3 border-b border-border space-y-3">
          <div className="flex items-center justify-between text-xs font-semibold">
            <span className={editingRuleId ? "text-amber-500 font-bold flex items-center gap-1.5" : "text-muted-foreground"}>
              {editingRuleId ? (
                <>
                  <MingCuteIcon name="edit_line" size={14} /> Editing Matching Rule
                </>
              ) : (
                "Add New Matching Rule"
              )}
            </span>
            {editingRuleId && (
              <button
                type="button"
                onClick={handleCancelEditRule}
                className="text-[11px] text-muted-foreground hover:text-foreground underline cursor-pointer"
              >
                Cancel Edit
              </button>
            )}
          </div>
          <div className="grid grid-cols-5 gap-2 text-xs">
            <select
              value={newRuleAction}
              onChange={(e: any) => setNewRuleAction(e.target.value)}
              className="bg-background border border-border rounded px-2 py-1.5 text-foreground outline-none cursor-pointer font-semibold"
            >
              <option value="intercept">Whitelist (Pause)</option>
              <option value="pass">Blacklist (Pass)</option>
            </select>
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
            <div className="flex items-center gap-1">
              <button
                type="submit"
                disabled={!newRuleValue.trim()}
                className={`flex-1 flex items-center justify-center gap-1 disabled:opacity-40 disabled:cursor-not-allowed text-white font-semibold rounded px-3 py-1.5 transition-colors cursor-pointer ${
                  editingRuleId
                    ? "bg-sky-500 hover:bg-sky-600"
                    : "bg-amber-500 hover:bg-amber-600"
                }`}
              >
                <MingCuteIcon name={editingRuleId ? "check_line" : "plus_line"} size={14} />
                {editingRuleId ? "Save" : "Add Rule"}
              </button>
              {editingRuleId && (
                <button
                  type="button"
                  onClick={handleCancelEditRule}
                  className="px-2 py-1.5 bg-surface hover:bg-neutral-subtle border border-border rounded text-muted-foreground hover:text-foreground transition-colors cursor-pointer text-xs"
                  title="Cancel editing"
                >
                  <MingCuteIcon name="close_line" size={14} />
                </button>
              )}
            </div>
          </div>
          <input
            type="text"
            placeholder="Match pattern value (e.g. api.example.com or /v1/auth)..."
            value={newRuleValue}
            onChange={(e) => setNewRuleValue(e.target.value)}
            className="w-full bg-background border border-border focus:border-primary rounded px-3 py-1.5 text-xs text-foreground outline-none font-mono"
          />
        </form>

        {/* Filter Tabs & Counter */}
        <div className="flex items-center justify-between pt-1 text-xs border-b border-border pb-2">
          <div className="flex items-center gap-1 bg-surface p-0.5 rounded-lg border border-border">
            <button
              type="button"
              onClick={() => setRuleFilterTab("all")}
              className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors cursor-pointer ${
                ruleFilterTab === "all"
                  ? "bg-primary text-primary-foreground font-bold shadow-2xs"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              All ({rules.length})
            </button>
            <button
              type="button"
              onClick={() => setRuleFilterTab("intercept")}
              className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors cursor-pointer ${
                ruleFilterTab === "intercept"
                  ? "bg-amber-500/20 text-amber-600 dark:text-amber-300 font-bold border border-amber-500/40 shadow-2xs"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              Whitelist ({rules.filter((r) => r.action !== "pass").length})
            </button>
            <button
              type="button"
              onClick={() => setRuleFilterTab("pass")}
              className={`px-2.5 py-1 rounded text-[11px] font-medium transition-colors cursor-pointer ${
                ruleFilterTab === "pass"
                  ? "bg-sky-500/20 text-sky-600 dark:text-sky-300 font-bold border border-sky-500/40 shadow-2xs"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              Blacklist ({rules.filter((r) => r.action === "pass").length})
            </button>
          </div>
          <span className="text-[11px] text-muted-foreground font-mono">
            {rules.filter((r) => r.isEnabled).length} active rule{rules.filter((r) => r.isEnabled).length === 1 ? "" : "s"}
          </span>
        </div>

        {/* Existing Rules List */}
        <div className="overflow-y-auto py-1 space-y-2 text-xs max-h-80">
          {rules.length === 0 ? (
            <div className="text-center py-6 text-muted-foreground italic">
              No custom rules added. All traffic will be paused when interceptor is active.
            </div>
          ) : filteredRules.length === 0 ? (
            <div className="text-center py-6 text-muted-foreground italic">
              No rules in the selected filter tab.
            </div>
          ) : (
            filteredRules.map((rule) => {
              const isWhitelist = rule.action !== "pass";
              return (
                <div
                  key={rule.id}
                  className={`flex items-center justify-between p-2.5 rounded-lg border transition-colors ${
                    editingRuleId === rule.id
                      ? "bg-sky-500/10 border-sky-500/40"
                      : "bg-background border-border hover:border-border/80"
                  }`}
                >
                  <div className="flex items-center gap-2 flex-wrap min-w-0">
                    <input
                      type="checkbox"
                      checked={rule.isEnabled}
                      onChange={() => toggleRule(rule.id)}
                      className="rounded accent-amber-500 cursor-pointer"
                      title={rule.isEnabled ? "Disable rule" : "Enable rule"}
                    />
                    <button
                      type="button"
                      onClick={() => updateRuleAction(rule.id, isWhitelist ? "pass" : "intercept")}
                      title="Click to toggle between Whitelist and Blacklist"
                      className={`px-1.5 py-0.5 rounded text-[10px] font-mono font-bold tracking-wide uppercase transition-colors cursor-pointer ${
                        isWhitelist
                          ? "bg-amber-500/20 text-amber-600 dark:text-amber-300 border border-amber-500/40 hover:bg-amber-500/30"
                          : "bg-sky-500/20 text-sky-600 dark:text-sky-300 border border-sky-500/40 hover:bg-sky-500/30"
                      }`}
                    >
                      {isWhitelist ? "WHITELIST (PAUSE)" : "BLACKLIST (PASS)"}
                    </button>
                    <span className="font-mono text-muted-foreground font-semibold uppercase text-[10px] bg-surface px-1.5 py-0.5 rounded border border-border">
                      {rule.targetPhase}
                    </span>
                    <span className="font-mono text-foreground text-xs truncate">
                      <span className="font-semibold text-primary">{rule.matchField}</span> {rule.operator}{" "}
                      <span className="bg-surface px-1.5 py-0.5 rounded border border-border font-mono text-amber-600 dark:text-amber-400">
                        "{rule.matchValue}"
                      </span>
                    </span>
                  </div>
                  <div className="flex items-center gap-1 shrink-0 ml-2">
                    <button
                      type="button"
                      onClick={() => handleStartEditRule(rule)}
                      className={`transition-colors p-1 cursor-pointer rounded ${
                        editingRuleId === rule.id
                          ? "text-sky-500 bg-sky-500/15"
                          : "text-muted-foreground hover:text-foreground hover:bg-neutral-subtle"
                      }`}
                      title="Edit rule"
                    >
                      <MingCuteIcon name="edit_line" size={14} />
                    </button>
                    <button
                      type="button"
                      onClick={() => deleteRule(rule.id)}
                      className="text-muted-foreground hover:text-rose-500 hover:bg-rose-500/15 transition-colors p-1 cursor-pointer rounded"
                      title="Delete rule"
                    >
                      <MingCuteIcon name="delete_2_line" size={14} />
                    </button>
                  </div>
                </div>
              );
            })
          )}
        </div>
      </Dialog>
    </div>
  );
};
