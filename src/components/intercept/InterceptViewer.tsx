import React, { useEffect, useState } from "react";
import { useInterceptStore } from "../../stores/useInterceptStore";
import { MingCuteIcon } from "../common/MingCuteIcon";

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
    <div className="flex flex-col h-screen w-full bg-zinc-950 text-zinc-100 font-sans antialiased overflow-hidden">
      {/* Top Header Controls */}
      <header className="flex items-center justify-between px-4 py-3 bg-zinc-900/90 border-b border-zinc-800 backdrop-blur-md">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="shield_line" size={18} className="text-amber-400" />
            <h1 className="text-lg font-bold tracking-tight bg-gradient-to-r from-amber-400 to-orange-400 bg-clip-text text-transparent">
              Traffic Interceptor
            </h1>
          </div>

          <div className="h-4 w-px bg-zinc-800" />

          {/* Master Intercept Toggle Button */}
          <button
            onClick={() => setInterceptEnabled(!interceptEnabled)}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold tracking-wide transition-all shadow-sm ${
              interceptEnabled
                ? "bg-amber-500/20 text-amber-300 border border-amber-500/30 hover:bg-amber-500/30 shadow-amber-950/20"
                : "bg-zinc-800 text-zinc-400 border border-zinc-700 hover:bg-zinc-700"
            }`}
          >
            <MingCuteIcon name="shield_line" size={14} className={interceptEnabled ? "text-amber-400 animate-pulse" : ""} />
            <span>Intercept is {interceptEnabled ? "ON" : "OFF"}</span>
          </button>

          {/* Intercept Phase Selector */}
          <div className="flex items-center bg-zinc-950 p-1 rounded-lg border border-zinc-800 text-xs">
            <button
              onClick={() => setInterceptMode("request")}
              className={`px-2.5 py-1 rounded-md transition-colors ${
                interceptMode === "request"
                  ? "bg-amber-500/20 text-amber-300 font-medium"
                  : "text-zinc-400 hover:text-zinc-200"
              }`}
            >
              Requests
            </button>
            <button
              onClick={() => setInterceptMode("response")}
              className={`px-2.5 py-1 rounded-md transition-colors ${
                interceptMode === "response"
                  ? "bg-amber-500/20 text-amber-300 font-medium"
                  : "text-zinc-400 hover:text-zinc-200"
              }`}
            >
              Responses
            </button>
            <button
              onClick={() => setInterceptMode("both")}
              className={`px-2.5 py-1 rounded-md transition-colors ${
                interceptMode === "both"
                  ? "bg-amber-500/20 text-amber-300 font-medium"
                  : "text-zinc-400 hover:text-zinc-200"
              }`}
            >
              Both
            </button>
          </div>
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-2">
          <button
            onClick={() => setRulesModalOpen(true)}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-xs rounded-lg transition-colors border border-zinc-700 font-medium"
          >
            <MingCuteIcon name="tool_line" size={14} /> Rules ({rules.length})
          </button>

          <div className="h-4 w-px bg-zinc-800 mx-1" />

          <button
            disabled={!selectedFlow}
            onClick={forwardCurrentFlow}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-emerald-500/20 border border-emerald-500/30 hover:bg-emerald-500/30 text-emerald-300 text-xs rounded-lg transition-colors font-semibold disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <MingCuteIcon name="play_line" size={14} /> Forward
          </button>

          <button
            disabled={!selectedFlow}
            onClick={dropCurrentFlow}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-rose-500/20 border border-rose-500/30 hover:bg-rose-500/30 text-rose-300 text-xs rounded-lg transition-colors font-semibold disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <MingCuteIcon name="close_line" size={14} /> Drop
          </button>

          <button
            disabled={pendingFlows.length === 0}
            onClick={forwardAllFlows}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-sky-500/20 border border-sky-500/30 hover:bg-sky-500/30 text-sky-300 text-xs rounded-lg transition-colors font-medium disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <MingCuteIcon name="fast_forward_line" size={14} /> Forward All
          </button>

          <button
            disabled={pendingFlows.length === 0}
            onClick={dropAllFlows}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-400 text-xs rounded-lg transition-colors font-medium disabled:opacity-40 disabled:cursor-not-allowed"
          >
            <MingCuteIcon name="delete_2_line" size={14} /> Drop All
          </button>
        </div>
      </header>

      {/* Main Split Pane Layout */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left Drawer: Paused Flow Queue */}
        <div className="w-80 flex flex-col border-r border-zinc-800 bg-zinc-950/60 shrink-0">
          <div className="px-3 py-2 bg-zinc-900/60 border-b border-zinc-800 flex items-center justify-between text-xs font-semibold text-zinc-400 uppercase tracking-wider">
            <span>Paused Queue</span>
            <span className="px-2 py-0.5 rounded-full bg-amber-500/20 text-amber-300 text-[10px] font-mono">
              {pendingFlows.length}
            </span>
          </div>

          <div className="flex-1 overflow-y-auto divide-y divide-zinc-900/80">
            {pendingFlows.length === 0 ? (
              <div className="flex flex-col items-center justify-center h-full p-6 text-center text-zinc-500">
                <MingCuteIcon name="shield_line" size={32} className="mb-2 text-zinc-700" />
                <p className="text-xs">No traffic paused</p>
                <p className="text-[11px] text-zinc-600 mt-1">
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
                        : "border-transparent hover:bg-zinc-900/40"
                    }`}
                  >
                    <div className="flex items-center justify-between mb-1">
                      <span className="px-1.5 py-0.5 rounded bg-zinc-800 text-amber-400 font-mono text-[10px] font-bold">
                        {flow.method}
                      </span>
                      <span className="text-[10px] font-mono uppercase text-zinc-500">
                        {flow.phase}
                      </span>
                    </div>
                    <div className="text-xs font-mono text-zinc-300 truncate">{flow.url}</div>
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* Right Pane: Headers & Body Payload Editor */}
        <div className="flex-1 flex flex-col bg-zinc-900/30 min-w-0">
          {selectedFlow ? (
            <div className="flex flex-col h-full">
              {/* Flow Detail Bar */}
              <div className="px-4 py-2.5 bg-zinc-900 border-b border-zinc-800 flex items-center justify-between text-xs font-mono">
                <div className="flex items-center gap-2 truncate">
                  <span className="px-2 py-0.5 rounded bg-amber-500/20 text-amber-300 font-bold">
                    {selectedFlow.method}
                  </span>
                  <span className="text-zinc-200 truncate">{selectedFlow.url}</span>
                </div>
                <div className="text-zinc-500 shrink-0 text-[11px]">
                  Phase: <span className="uppercase text-amber-400">{selectedFlow.phase}</span>
                </div>
              </div>

              {/* Payload Editor Tabs */}
              <div className="flex items-center gap-2 px-4 py-2 bg-zinc-950 border-b border-zinc-800">
                <button
                  onClick={() => setActiveTab("headers")}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
                    activeTab === "headers"
                      ? "bg-amber-500/20 text-amber-300 border border-amber-500/30"
                      : "text-zinc-400 hover:text-zinc-200"
                  }`}
                >
                  Headers ({editedHeaders.length})
                </button>
                <button
                  onClick={() => setActiveTab("body")}
                  className={`px-3 py-1 rounded-md text-xs font-medium transition-colors ${
                    activeTab === "body"
                      ? "bg-amber-500/20 text-amber-300 border border-amber-500/30"
                      : "text-zinc-400 hover:text-zinc-200"
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
                      <span className="text-zinc-400 font-sans text-xs font-semibold">
                        Edit HTTP Headers
                      </span>
                      <button
                        onClick={handleAddHeader}
                        className="flex items-center gap-1 px-2.5 py-1 bg-zinc-800 hover:bg-zinc-700 text-amber-400 rounded text-xs transition-colors"
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
                          className="flex-1 bg-zinc-950 border border-zinc-800 focus:border-amber-500/60 rounded px-2.5 py-1.5 text-zinc-200 outline-none"
                        />
                        <input
                          type="text"
                          placeholder="Header Value"
                          value={val}
                          onChange={(e) => handleHeaderChange(idx, key, e.target.value)}
                          className="flex-2 bg-zinc-950 border border-zinc-800 focus:border-amber-500/60 rounded px-2.5 py-1.5 text-zinc-200 outline-none"
                        />
                        <button
                          onClick={() => handleRemoveHeader(idx)}
                          className="p-1.5 text-zinc-500 hover:text-rose-400 rounded transition-colors"
                        >
                          <MingCuteIcon name="close_line" size={14} />
                        </button>
                      </div>
                    ))}
                  </div>
                ) : (
                  <div className="flex flex-col h-full space-y-2">
                    <span className="text-zinc-400 font-sans text-xs font-semibold">
                      Edit Body Content
                    </span>
                    <textarea
                      value={editedBodyText}
                      onChange={(e) => setEditedBodyText(e.target.value)}
                      placeholder="Enter raw request/response body payload..."
                      className="flex-1 min-h-[350px] w-full bg-zinc-950 border border-zinc-800 focus:border-amber-500/60 rounded-lg p-3 text-zinc-200 outline-none resize-none font-mono text-xs leading-relaxed"
                    />
                  </div>
                )}
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-zinc-500 p-6 text-center">
              <MingCuteIcon name="code_line" size={36} className="mb-2 text-zinc-700" />
              <p className="text-sm font-medium">Select a paused flow from the left queue</p>
            </div>
          )}
        </div>
      </div>

      {/* Intercept Rules Modal */}
      {rulesModalOpen && (
        <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4">
          <div className="bg-zinc-900 border border-zinc-800 rounded-xl max-w-2xl w-full p-5 shadow-2xl flex flex-col max-h-[85vh]">
            <div className="flex items-center justify-between pb-3 border-b border-zinc-800">
              <div className="flex items-center gap-2">
                <MingCuteIcon name="tool_line" size={18} className="text-amber-400" />
                <h2 className="text-sm font-bold text-zinc-100">Intercept Rules Engine</h2>
              </div>
              <button onClick={() => setRulesModalOpen(false)} className="text-zinc-400 hover:text-zinc-200">
                <MingCuteIcon name="close_line" size={18} />
              </button>
            </div>

            {/* Add New Rule Form */}
            <form onSubmit={handleCreateRule} className="py-4 border-b border-zinc-800 space-y-3">
              <div className="text-xs font-semibold text-zinc-400">Add New Matching Rule</div>
              <div className="grid grid-cols-4 gap-2 text-xs">
                <select
                  value={newRulePhase}
                  onChange={(e: any) => setNewRulePhase(e.target.value)}
                  className="bg-zinc-950 border border-zinc-800 rounded px-2 py-1.5 text-zinc-300 outline-none"
                >
                  <option value="both">Both Phases</option>
                  <option value="request">Request</option>
                  <option value="response">Response</option>
                </select>
                <select
                  value={newRuleField}
                  onChange={(e: any) => setNewRuleField(e.target.value)}
                  className="bg-zinc-950 border border-zinc-800 rounded px-2 py-1.5 text-zinc-300 outline-none"
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
                  className="bg-zinc-950 border border-zinc-800 rounded px-2 py-1.5 text-zinc-300 outline-none"
                >
                  <option value="contains">Contains</option>
                  <option value="equals">Equals</option>
                  <option value="regex">Regex</option>
                </select>
                <button
                  type="submit"
                  className="flex items-center justify-center gap-1 bg-amber-500 hover:bg-amber-600 text-zinc-950 font-semibold rounded px-3 py-1.5 transition-colors"
                >
                  <MingCuteIcon name="plus_line" size={14} /> Add Rule
                </button>
              </div>
              <input
                type="text"
                placeholder="Match pattern value (e.g. api.example.com or /v1/auth)..."
                value={newRuleValue}
                onChange={(e) => setNewRuleValue(e.target.value)}
                className="w-full bg-zinc-950 border border-zinc-800 focus:border-amber-500/60 rounded px-3 py-1.5 text-xs text-zinc-200 outline-none"
              />
            </form>

            {/* Existing Rules List */}
            <div className="flex-1 overflow-y-auto py-3 space-y-2 text-xs">
              {rules.length === 0 ? (
                <div className="text-center py-6 text-zinc-500">
                  No custom rules added. All traffic will be paused when interceptor is active.
                </div>
              ) : (
                rules.map((rule) => (
                  <div
                    key={rule.id}
                    className="flex items-center justify-between p-2.5 bg-zinc-950 rounded-lg border border-zinc-800"
                  >
                    <div className="flex items-center gap-2">
                      <input
                        type="checkbox"
                        checked={rule.isEnabled}
                        onChange={() => toggleRule(rule.id)}
                        className="rounded accent-amber-500 cursor-pointer"
                      />
                      <span className="font-mono text-amber-400 font-semibold uppercase text-[10px]">
                        [{rule.targetPhase}]
                      </span>
                      <span className="font-mono text-zinc-300">
                        {rule.matchField} {rule.operator} "{rule.matchValue}"
                      </span>
                    </div>
                    <button
                      onClick={() => deleteRule(rule.id)}
                      className="text-zinc-500 hover:text-rose-400 transition-colors p-1"
                    >
                      <MingCuteIcon name="delete_2_line" size={14} />
                    </button>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
