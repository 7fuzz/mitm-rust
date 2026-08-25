import React, { useEffect, useState } from "react";
import { useInterceptStore } from "../../stores/useInterceptStore";
import {
  ShieldAlert,
  Play,
  X,
  FastForward,
  Trash2,
  Plus,
  Check,
  Code,
} from "lucide-react";


export const InterceptViewer: React.FC = () => {
  const {
    interceptEnabled,
    interceptMode,
    rules,
    pendingFlows,
    selectedFlowId,
    editedHeaders,
    editedBodyText,
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
    toggleRule,
    deleteRule,
    addRule,
  } = useInterceptStore();

  const [activeTab, setActiveTab] = useState<"inspector" | "rules">("inspector");
  const [showAddRuleModal, setShowAddRuleModal] = useState(false);

  // New rule form state
  const [newMatchField, setNewMatchField] = useState<"url" | "host" | "path" | "method" | "header">("url");
  const [newOperator, setNewOperator] = useState<"contains" | "equals" | "regex">("contains");
  const [newMatchValue, setNewMatchValue] = useState("");
  const [newTargetPhase, setNewTargetPhase] = useState<"request" | "response" | "both">("both");

  useEffect(() => {
    initInterceptStore();
  }, []);

  const selectedFlow = pendingFlows.find((f) => f.flowId === selectedFlowId);

  const handleHeaderChange = (index: number, key: string, value: string) => {
    const updated = [...editedHeaders];
    updated[index] = [key, value];
    setEditedHeaders(updated);
  };

  const handleAddHeader = () => {
    setEditedHeaders([...editedHeaders, ["", ""]]);
  };

  const handleRemoveHeader = (index: number) => {
    setEditedHeaders(editedHeaders.filter((_, i) => i !== index));
  };

  const handleAddRuleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newMatchValue.trim()) return;
    await addRule({
      isEnabled: true,
      targetPhase: newTargetPhase,
      matchField: newMatchField,
      operator: newOperator,
      matchValue: newMatchValue.trim(),
      orderIndex: rules.length,
    });
    setNewMatchValue("");
    setShowAddRuleModal(false);
  };

  return (
    <div className="flex flex-col h-screen w-full bg-zinc-950 text-zinc-100 font-sans antialiased overflow-hidden">
      {/* Intercept Control Header */}
      <header className="flex items-center justify-between px-4 py-3 bg-zinc-900/90 border-b border-zinc-800 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <ShieldAlert className={`w-5 h-5 ${interceptEnabled ? "text-amber-400 animate-pulse" : "text-zinc-500"}`} />
            <h1 className="text-lg font-bold tracking-tight bg-gradient-to-r from-amber-400 to-orange-400 bg-clip-text text-transparent">
              Traffic Interceptor
            </h1>
          </div>

          <div className="h-4 w-px bg-zinc-800 mx-1" />

          {/* Master Intercept Toggle Button */}
          <button
            onClick={() => setInterceptEnabled(!interceptEnabled)}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold tracking-wide transition-all shadow-sm ${
              interceptEnabled
                ? "bg-amber-500/20 text-amber-300 border border-amber-500/40 hover:bg-amber-500/30 shadow-amber-950/20"
                : "bg-zinc-800 text-zinc-400 border border-zinc-700 hover:bg-zinc-700/60"
            }`}
          >
            {interceptEnabled ? "Intercept ON" : "Intercept OFF"}
          </button>

          {/* Intercept Mode Dropdown */}
          <select
            value={interceptMode}
            onChange={(e) => setInterceptMode(e.target.value as "request" | "response" | "both")}
            disabled={!interceptEnabled}
            className="bg-zinc-950 border border-zinc-800 focus:border-amber-500/60 rounded-lg text-xs px-2.5 py-1.5 text-zinc-300 outline-none cursor-pointer disabled:opacity-50"
          >
            <option value="both font-sans">Intercept Requests & Responses</option>
            <option value="request">Intercept Requests Only</option>
            <option value="response">Intercept Responses Only</option>
          </select>
        </div>

        {/* Action Controls */}
        <div className="flex items-center gap-2">
          <button
            onClick={forwardCurrentFlow}
            disabled={!selectedFlowId}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-emerald-500/20 border border-emerald-500/30 text-emerald-300 hover:bg-emerald-500/30 disabled:opacity-40 disabled:cursor-not-allowed rounded-lg text-xs font-medium transition-all"
            title="Forward modified flow"
          >
            <Play className="w-3.5 h-3.5 fill-current" /> Forward
          </button>

          <button
            onClick={dropCurrentFlow}
            disabled={!selectedFlowId}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-rose-500/20 border border-rose-500/30 text-rose-300 hover:bg-rose-500/30 disabled:opacity-40 disabled:cursor-not-allowed rounded-lg text-xs font-medium transition-all"
            title="Drop flow"
          >
            <X className="w-3.5 h-3.5 stroke-[2.5]" /> Drop
          </button>

          <div className="h-4 w-px bg-zinc-800 mx-1" />

          <button
            onClick={forwardAllFlows}
            disabled={pendingFlows.length === 0}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700/60 disabled:opacity-40 disabled:cursor-not-allowed rounded-lg text-xs font-medium transition-all"
            title="Forward all queued flows"
          >
            <FastForward className="w-3.5 h-3.5 fill-current" /> Forward All ({pendingFlows.length})
          </button>

          <button
            onClick={dropAllFlows}
            disabled={pendingFlows.length === 0}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-rose-400 border border-zinc-700/60 disabled:opacity-40 disabled:cursor-not-allowed rounded-lg text-xs font-medium transition-all"
            title="Drop all queued flows"
          >
            <Trash2 className="w-3.5 h-3.5" /> Drop All
          </button>

          <div className="h-4 w-px bg-zinc-800 mx-1" />

          {/* View Tab Switcher */}
          <div className="flex bg-zinc-950 p-1 rounded-lg border border-zinc-800">
            <button
              onClick={() => setActiveTab("inspector")}
              className={`px-3 py-1 text-xs font-medium rounded-md transition-colors ${
                activeTab === "inspector" ? "bg-zinc-800 text-amber-300" : "text-zinc-400 hover:text-zinc-200"
              }`}
            >
              Inspector ({pendingFlows.length})
            </button>
            <button
              onClick={() => setActiveTab("rules")}
              className={`px-3 py-1 text-xs font-medium rounded-md transition-colors ${
                activeTab === "rules" ? "bg-zinc-800 text-amber-300" : "text-zinc-400 hover:text-zinc-200"
              }`}
            >
              Rules ({rules.length})
            </button>
          </div>
        </div>
      </header>

      {/* Main Body */}
      {activeTab === "inspector" ? (
        <div className="flex flex-1 overflow-hidden">
          {/* Flow Queue Drawer / List */}
          <div className="w-72 lg:w-80 flex flex-col border-r border-zinc-800 bg-zinc-950/60">
            <div className="p-3 border-b border-zinc-800 text-xs font-semibold text-zinc-400 uppercase tracking-wider flex justify-between items-center">
              <span>Paused Flow Queue</span>
              <span className="bg-amber-500/20 text-amber-300 px-2 py-0.5 rounded-full text-[10px] font-mono">
                {pendingFlows.length}
              </span>
            </div>

            <div className="flex-1 overflow-y-auto divide-y divide-zinc-900/60">
              {pendingFlows.length === 0 ? (
                <div className="p-8 text-center text-zinc-500 text-xs italic">
                  No paused flows in queue. When requests match rules, they will appear here.
                </div>
              ) : (
                pendingFlows.map((flow) => {
                  const isSelected = flow.flowId === selectedFlowId;
                  return (
                    <div
                      key={flow.flowId}
                      onClick={() => selectFlow(flow.flowId)}
                      className={`p-3 cursor-pointer transition-colors ${
                        isSelected
                          ? "bg-amber-500/10 border-l-2 border-amber-400"
                          : "hover:bg-zinc-900/40"
                      }`}
                    >
                      <div className="flex items-center justify-between mb-1">
                        <span className="font-mono text-[10px] font-bold px-1.5 py-0.5 rounded bg-zinc-800 text-amber-300 uppercase">
                          {flow.phase}
                        </span>
                        <span className="font-mono text-[10px] font-bold px-1.5 py-0.5 rounded bg-zinc-900 text-zinc-400">
                          {flow.method}
                        </span>
                      </div>
                      <div className="text-xs text-zinc-200 font-mono truncate">{flow.url}</div>
                    </div>
                  );
                })
              )}
            </div>
          </div>

          {/* Payload Editor Drawer */}
          <div className="flex-1 flex flex-col bg-zinc-900/40 min-w-0">
            {selectedFlow ? (
              <div className="flex flex-col h-full overflow-hidden p-4 space-y-4">
                {/* Flow Metadata Banner */}
                <div className="bg-zinc-950 p-3 rounded-lg border border-zinc-800 font-mono text-xs flex justify-between items-center">
                  <div>
                    <span className="text-amber-400 font-bold mr-2">[{selectedFlow.phase.toUpperCase()}]</span>
                    <span className="text-zinc-300 font-semibold mr-2">{selectedFlow.method}</span>
                    <span className="text-zinc-400">{selectedFlow.url}</span>
                  </div>
                  <span className="text-zinc-500 text-[10px] font-mono">ID: {selectedFlow.flowId.substring(0, 8)}</span>
                </div>

                {/* Headers Editor */}
                <div className="flex-1 flex flex-col bg-zinc-950 p-3 rounded-lg border border-zinc-800 overflow-hidden min-h-[180px]">
                  <div className="flex items-center justify-between mb-2">
                    <span className="text-xs font-semibold uppercase text-zinc-400 tracking-wider">
                      Headers Editor ({editedHeaders.length})
                    </span>
                    <button
                      onClick={handleAddHeader}
                      className="flex items-center gap-1 text-[11px] bg-zinc-800 hover:bg-zinc-700 text-zinc-300 px-2.5 py-1 rounded transition-colors"
                    >
                      <Plus className="w-3 h-3" /> Add Header
                    </button>
                  </div>

                  <div className="flex-1 overflow-y-auto space-y-2 pr-1">
                    {editedHeaders.map(([key, val], idx) => (
                      <div key={idx} className="flex gap-2 items-center">
                        <input
                          type="text"
                          placeholder="Header Key"
                          value={key}
                          onChange={(e) => handleHeaderChange(idx, e.target.value, val)}
                          className="bg-zinc-900 border border-zinc-800 text-xs px-2.5 py-1.5 rounded font-mono text-indigo-300 w-1/3 outline-none focus:border-amber-500/50"
                        />
                        <input
                          type="text"
                          placeholder="Header Value"
                          value={val}
                          onChange={(e) => handleHeaderChange(idx, key, e.target.value)}
                          className="bg-zinc-900 border border-zinc-800 text-xs px-2.5 py-1.5 rounded font-mono text-zinc-200 flex-1 outline-none focus:border-amber-500/50"
                        />
                        <button
                          onClick={() => handleRemoveHeader(idx)}
                          className="p-1 text-zinc-500 hover:text-rose-400 transition-colors"
                        >
                          <X className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    ))}
                  </div>
                </div>

                {/* Body Editor */}
                <div className="flex-1 flex flex-col bg-zinc-950 p-3 rounded-lg border border-zinc-800 overflow-hidden min-h-[200px]">
                  <div className="text-xs font-semibold uppercase text-zinc-400 tracking-wider mb-2">
                    Body Content Payload Editor
                  </div>
                  <textarea
                    value={editedBodyText}
                    onChange={(e) => setEditedBodyText(e.target.value)}
                    placeholder="Payload body content..."
                    className="flex-1 bg-zinc-900 border border-zinc-800 rounded p-3 text-xs font-mono text-zinc-200 outline-none focus:border-amber-500/50 resize-none"
                  />
                </div>
              </div>
            ) : (
              <div className="flex-1 flex flex-col items-center justify-center text-zinc-500 p-6 text-center">
                <Code className="w-12 h-12 mb-3 stroke-[1.2] text-zinc-600" />
                <p className="text-sm font-medium">Select a paused flow from the left queue to inspect and modify</p>
              </div>
            )}
          </div>
        </div>
      ) : (
        /* Rules Configuration View */
        <div className="flex-1 p-6 overflow-y-auto max-w-5xl mx-auto w-full">
          <div className="flex justify-between items-center mb-6">
            <div>
              <h2 className="text-lg font-bold text-zinc-100">Intercept Rules Engine</h2>
              <p className="text-xs text-zinc-400">Configure matching rules for automatic traffic interception</p>
            </div>
            <button
              onClick={() => setShowAddRuleModal(true)}
              className="flex items-center gap-1.5 px-3.5 py-2 bg-amber-500/20 text-amber-300 border border-amber-500/40 rounded-lg text-xs font-semibold hover:bg-amber-500/30 transition-all"
            >
              <Plus className="w-4 h-4" /> Add Rule
            </button>
          </div>

          {/* Rules List */}
          <div className="bg-zinc-900/60 rounded-xl border border-zinc-800 overflow-hidden divide-y divide-zinc-800">
            {rules.length === 0 ? (
              <div className="p-12 text-center text-zinc-500 text-sm">
                No active rules defined. When Intercept is enabled with no rules, all traffic is paused by default.
              </div>
            ) : (
              rules.map((rule) => (
                <div key={rule.id} className="p-4 flex items-center justify-between hover:bg-zinc-900/40 transition-colors">
                  <div className="flex items-center gap-4">
                    <button
                      onClick={() => toggleRule(rule.id)}
                      className={`w-5 h-5 rounded flex items-center justify-center border transition-colors ${
                        rule.isEnabled ? "bg-amber-500 border-amber-400 text-zinc-950" : "border-zinc-700 bg-zinc-950"
                      }`}
                    >
                      {rule.isEnabled && <Check className="w-3.5 h-3.5 stroke-[3]" />}
                    </button>
                    <div>
                      <div className="flex items-center gap-2 font-mono text-xs">
                        <span className="px-2 py-0.5 rounded bg-zinc-800 text-amber-300 uppercase text-[10px]">
                          {rule.targetPhase}
                        </span>
                        <span className="text-zinc-400">{rule.matchField}</span>
                        <span className="text-amber-400 font-bold">{rule.operator}</span>
                        <span className="text-zinc-200 bg-zinc-950 px-2 py-0.5 rounded border border-zinc-800">
                          {rule.matchValue}
                        </span>
                      </div>
                    </div>
                  </div>
                  <button
                    onClick={() => deleteRule(rule.id)}
                    className="p-1.5 text-zinc-500 hover:text-rose-400 transition-colors"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>
              ))
            )}
          </div>

          {/* Add Rule Modal */}
          {showAddRuleModal && (
            <div className="fixed inset-0 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4 z-50">
              <form onSubmit={handleAddRuleSubmit} className="bg-zinc-900 border border-zinc-800 rounded-xl p-6 w-full max-w-md space-y-4">
                <h3 className="text-base font-bold text-zinc-100">Add Intercept Rule</h3>

                <div>
                  <label className="block text-xs text-zinc-400 mb-1">Target Phase</label>
                  <select
                    value={newTargetPhase}
                    onChange={(e) => setNewTargetPhase(e.target.value as any)}
                    className="w-full bg-zinc-950 border border-zinc-800 rounded text-xs p-2 text-zinc-200 outline-none"
                  >
                    <option value="both">Both (Request & Response)</option>
                    <option value="request">Request</option>
                    <option value="response">Response</option>
                  </select>
                </div>

                <div>
                  <label className="block text-xs text-zinc-400 mb-1">Match Field</label>
                  <select
                    value={newMatchField}
                    onChange={(e) => setNewMatchField(e.target.value as any)}
                    className="w-full bg-zinc-950 border border-zinc-800 rounded text-xs p-2 text-zinc-200 outline-none"
                  >
                    <option value="url">URL</option>
                    <option value="host">Host</option>
                    <option value="path">Path</option>
                    <option value="method">Method</option>
                    <option value="header">Header</option>
                  </select>
                </div>

                <div>
                  <label className="block text-xs text-zinc-400 mb-1">Operator</label>
                  <select
                    value={newOperator}
                    onChange={(e) => setNewOperator(e.target.value as any)}
                    className="w-full bg-zinc-950 border border-zinc-800 rounded text-xs p-2 text-zinc-200 outline-none"
                  >
                    <option value="contains">Contains</option>
                    <option value="equals">Equals</option>
                    <option value="regex">Regex Match</option>
                  </select>
                </div>

                <div>
                  <label className="block text-xs text-zinc-400 mb-1">Match Value Pattern</label>
                  <input
                    type="text"
                    placeholder="e.g. api.example.com"
                    value={newMatchValue}
                    onChange={(e) => setNewMatchValue(e.target.value)}
                    className="w-full bg-zinc-950 border border-zinc-800 rounded text-xs p-2 text-zinc-200 outline-none font-mono"
                  />
                </div>

                <div className="flex justify-end gap-2 pt-2">
                  <button
                    type="button"
                    onClick={() => setShowAddRuleModal(false)}
                    className="px-4 py-2 bg-zinc-800 text-zinc-300 rounded-lg text-xs font-medium hover:bg-zinc-700"
                  >
                    Cancel
                  </button>
                  <button
                    type="submit"
                    className="px-4 py-2 bg-amber-500 text-zinc-950 rounded-lg text-xs font-bold hover:bg-amber-400"
                  >
                    Save Rule
                  </button>
                </div>
              </form>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
