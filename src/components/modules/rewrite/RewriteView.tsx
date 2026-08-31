import React, { useEffect, useState } from "react";
import { useRewriteStore } from "../../../stores/useRewriteStore";
import { MingCuteIcon } from "../../common/MingCuteIcon";
import { Button, Input } from "../../common/ui";
import { RewriteRuleModal } from "./RewriteRuleModal";

export const RewriteView: React.FC = () => {
  const {
    rules,
    logs,
    isRewriteEnabled,
    selectedLogId,
    searchQuery,
    initStore,
    toggleEnabled,
    toggleRule,
    deleteRule,
    duplicateRule,
    moveRule,
    clearLogs,
    selectLog,
    setSearchQuery,
    openRuleModal,
  } = useRewriteStore();

  const [activeTab, setActiveTab] = useState<"rules" | "history">("rules");

  useEffect(() => {
    initStore();
  }, []);

  const filteredRules = rules.filter((r) => {
    if (!searchQuery.trim()) return true;
    const q = searchQuery.toLowerCase();
    return (
      r.name.toLowerCase().includes(q) ||
      r.matchValue.toLowerCase().includes(q) ||
      r.matchPattern.toLowerCase().includes(q) ||
      r.replacementValue.toLowerCase().includes(q)
    );
  });

  const filteredLogs = logs.filter((l) => {
    if (!searchQuery.trim()) return true;
    const q = searchQuery.toLowerCase();
    return (
      l.ruleName.toLowerCase().includes(q) ||
      l.originalUrl.toLowerCase().includes(q) ||
      l.rewrittenUrl.toLowerCase().includes(q) ||
      l.method.toLowerCase().includes(q)
    );
  });

  const selectedLog = logs.find((l) => l.id === selectedLogId) || (logs.length > 0 ? logs[0] : null);

  const getActionBadge = (type: string) => {
    switch (type) {
      case "partial_request":
        return <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 border border-emerald-500/30">PARTIAL REQ</span>;
      case "full_request":
        return <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-sky-500/15 text-sky-600 dark:text-sky-400 border border-sky-500/30">FULL REQ</span>;
      case "partial_response":
        return <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-purple-500/15 text-purple-600 dark:text-purple-400 border border-purple-500/30">PARTIAL RES</span>;
      case "full_response":
      case "full_response (Mock)":
        return <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-amber-500/15 text-amber-600 dark:text-amber-400 border border-amber-500/30">MOCK RES</span>;
      default:
        return <span className="px-2 py-0.5 rounded text-[10px] font-bold bg-neutral-subtle text-muted-foreground">{type}</span>;
    }
  };

  const handleCreateTemplate = (templateType: string) => {
    if (templateType === "typo") {
      openRuleModal({
        id: "",
        name: "Fix URL typo (/v1/v1/ ➔ /v1/)",
        enabled: true,
        actionType: "partial_request",
        matchField: "url",
        matchOperator: "contains",
        matchValue: "/v1/v1/",
        targetPart: "url",
        matchPattern: "/v1/v1/",
        replacementValue: "/v1/",
        isRegex: false,
        orderIndex: rules.length,
        createdAtMs: Date.now(),
      });
    } else if (templateType === "redirect") {
      openRuleModal({
        id: "",
        name: "Redirect Production to Localhost",
        enabled: true,
        actionType: "full_request",
        matchField: "host",
        matchOperator: "contains",
        matchValue: "api.production.com",
        targetPart: "url",
        matchPattern: "https://api.production.com",
        replacementValue: "http://localhost:3000",
        isRegex: false,
        orderIndex: rules.length,
        createdAtMs: Date.now(),
      });
    } else if (templateType === "mock") {
      openRuleModal({
        id: "",
        name: "Mock 200 OK Response",
        enabled: true,
        actionType: "full_response",
        matchField: "path",
        matchOperator: "contains",
        matchValue: "/api/test",
        targetPart: "status",
        matchPattern: "",
        replacementValue: "",
        isRegex: false,
        mockStatusCode: 200,
        mockHeadersJson: '[\n  ["Content-Type", "application/json"],\n  ["Access-Control-Allow-Origin", "*"]\n]',
        mockBody: '{\n  "status": "success",\n  "message": "Mocked response from Rewrite Engine"\n}',
        orderIndex: rules.length,
        createdAtMs: Date.now(),
      });
    }
  };

  return (
    <div className="flex flex-col h-full w-full bg-background text-foreground font-sans antialiased overflow-hidden select-none">
      {/* Top Header Toolbar */}
      <header className="flex items-center justify-between px-4 py-2 bg-header border-b border-border shrink-0">
        <div className="flex items-center gap-4">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="swap_line" size={18} className="text-emerald-500" />
            <h1 className="text-sm font-bold tracking-tight text-foreground">
              Rewrite Engine
            </h1>
          </div>

          <div className="h-4 w-px bg-border" />

          {/* Master Toggle */}
          <button
            onClick={toggleEnabled}
            className={`flex items-center gap-2 px-3 py-1 rounded-lg text-xs font-semibold tracking-wide transition-all shadow-2xs cursor-pointer ${
              isRewriteEnabled
                ? "bg-emerald-500/20 text-emerald-600 dark:text-emerald-300 border border-emerald-500/40"
                : "bg-surface text-muted-foreground border border-border hover:bg-neutral-subtle hover:text-foreground"
            }`}
          >
            <MingCuteIcon name="swap_line" size={14} className={isRewriteEnabled ? "text-emerald-500 animate-pulse" : ""} />
            <span>Rewrite is {isRewriteEnabled ? "ON" : "OFF"}</span>
          </button>

          {/* View Tab Switcher */}
          <div className="flex items-center bg-surface p-0.5 rounded-lg border border-border text-xs font-mono">
            <button
              onClick={() => setActiveTab("rules")}
              className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs transition-colors cursor-pointer ${
                activeTab === "rules"
                  ? "bg-primary text-primary-foreground font-bold shadow-2xs"
                  : "text-muted-foreground hover:text-foreground hover:bg-neutral-subtle font-medium"
              }`}
            >
              <MingCuteIcon name="tool_line" size={13} />
              <span>Rules ({rules.length})</span>
            </button>
            <button
              onClick={() => setActiveTab("history")}
              className={`flex items-center gap-1.5 px-3 py-1 rounded text-xs transition-colors cursor-pointer ${
                activeTab === "history"
                  ? "bg-primary text-primary-foreground font-bold shadow-2xs"
                  : "text-muted-foreground hover:text-foreground hover:bg-neutral-subtle font-medium"
              }`}
            >
              <MingCuteIcon name="history_line" size={13} />
              <span>Rewrite History ({logs.length})</span>
            </button>
          </div>
        </div>

        {/* Right Actions */}
        <div className="flex items-center gap-2">
          {/* Search Input */}
          <div className="w-52">
            <Input
              leftIcon="search_line"
              sizeVariant="xs"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search rules or logs..."
            />
          </div>

          {activeTab === "history" ? (
            <Button
              variant="destructive"
              sizeVariant="sm"
              icon="delete_2_line"
              disabled={logs.length === 0}
              onClick={clearLogs}
            >
              Clear All Logs
            </Button>
          ) : (
            <Button
              variant="primary"
              sizeVariant="sm"
              icon="plus_line"
              onClick={() => openRuleModal()}
            >
              Add Rewrite Rule
            </Button>
          )}
        </div>
      </header>

      {/* Main Content Workspace */}
      <div className="flex-1 overflow-hidden">
        {activeTab === "rules" ? (
          /* Rules List Table */
          <div className="h-full flex flex-col p-4 overflow-y-auto font-sans text-xs">
            {filteredRules.length === 0 ? (
              <div className="flex flex-col items-center justify-center flex-1 text-center p-8 border border-dashed border-border rounded-xl bg-surface/40">
                <div className="p-3 bg-emerald-500/10 rounded-full mb-3">
                  <MingCuteIcon name="swap_line" size={32} className="text-emerald-500" />
                </div>
                <h3 className="text-sm font-bold text-foreground mb-1">No Rewrite Rules Configured</h3>
                <p className="text-xs text-muted-foreground max-w-md mb-5">
                  Automate URL typo fixing, request/response modification, domain redirection, or mock API responses in the background without pausing traffic.
                </p>

                {/* Quick Start Templates */}
                <div className="flex items-center gap-2">
                  <button
                    onClick={() => handleCreateTemplate("typo")}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-surface hover:bg-neutral-subtle border border-border text-foreground font-medium text-xs transition-colors cursor-pointer"
                  >
                    <MingCuteIcon name="edit_3_line" size={14} className="text-primary" />
                    <span>Fix URL typo (/v1/v1/ ➔ /v1/)</span>
                  </button>
                  <button
                    onClick={() => handleCreateTemplate("redirect")}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-surface hover:bg-neutral-subtle border border-border text-foreground font-medium text-xs transition-colors cursor-pointer"
                  >
                    <MingCuteIcon name="external_link_line" size={14} className="text-sky-500" />
                    <span>Redirect Host</span>
                  </button>
                  <button
                    onClick={() => handleCreateTemplate("mock")}
                    className="flex items-center gap-1.5 px-3 py-1.5 rounded-lg bg-surface hover:bg-neutral-subtle border border-border text-foreground font-medium text-xs transition-colors cursor-pointer"
                  >
                    <MingCuteIcon name="box_3_line" size={14} className="text-amber-500" />
                    <span>Mock API Response</span>
                  </button>
                </div>
              </div>
            ) : (
              <div className="border border-border rounded-lg overflow-hidden bg-surface shadow-2xs font-mono">
                <table className="w-full text-left border-collapse">
                  <thead>
                    <tr className="bg-header border-b border-border text-muted-foreground text-[11px] font-sans font-semibold">
                      <th className="w-10 px-3 py-2 text-center">Active</th>
                      <th className="w-12 px-2 py-2 text-center">Order</th>
                      <th className="px-3 py-2">Rule Name</th>
                      <th className="px-3 py-2">Type</th>
                      <th className="px-3 py-2">Match Filter</th>
                      <th className="px-3 py-2">Rewrite Action</th>
                      <th className="w-32 px-3 py-2 text-right">Actions</th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border text-xs">
                    {filteredRules.map((rule, idx) => (
                      <tr key={rule.id} className="hover:bg-neutral-subtle/50 transition-colors">
                        {/* Enable Checkbox */}
                        <td className="px-3 py-2 text-center">
                          <input
                            type="checkbox"
                            checked={rule.enabled}
                            onChange={() => toggleRule(rule.id)}
                            className="rounded border-border text-primary focus:ring-primary h-3.5 w-3.5 cursor-pointer"
                          />
                        </td>

                        {/* Order & Move buttons */}
                        <td className="px-2 py-2 text-center">
                          <div className="flex items-center justify-center gap-0.5 text-muted-foreground">
                            <button
                              disabled={idx === 0}
                              onClick={() => moveRule(idx, "up")}
                              className="p-0.5 hover:text-foreground disabled:opacity-20 cursor-pointer"
                              title="Move Up"
                            >
                              <MingCuteIcon name="up_line" size={12} />
                            </button>
                            <button
                              disabled={idx === filteredRules.length - 1}
                              onClick={() => moveRule(idx, "down")}
                              className="p-0.5 hover:text-foreground disabled:opacity-20 cursor-pointer"
                              title="Move Down"
                            >
                              <MingCuteIcon name="down_line" size={12} />
                            </button>
                          </div>
                        </td>

                        {/* Rule Name */}
                        <td className="px-3 py-2 font-sans font-bold text-foreground">
                          {rule.name}
                        </td>

                        {/* Action Type Badge */}
                        <td className="px-3 py-2">
                          {getActionBadge(rule.actionType)}
                        </td>

                        {/* Match Criteria */}
                        <td className="px-3 py-2 text-muted-foreground truncate max-w-xs">
                          {rule.matchField === "all" ? (
                            <span className="text-foreground italic">All Traffic (*)</span>
                          ) : (
                            <span>
                              <span className="font-semibold text-foreground">{rule.matchField}</span> {rule.matchOperator} <span className="text-amber-500 font-bold">"{rule.matchValue}"</span>
                            </span>
                          )}
                        </td>

                        {/* Transformation Summary */}
                        <td className="px-3 py-2 truncate max-w-sm">
                          {rule.actionType === "full_response" ? (
                            <span className="text-amber-500 font-bold">Mock HTTP {rule.mockStatusCode || 200}</span>
                          ) : rule.actionType === "full_request" ? (
                            <span>
                              <span className="text-muted-foreground">{rule.targetPart}: </span>
                              <span className="text-emerald-500 font-bold">{rule.replacementValue}</span>
                            </span>
                          ) : (
                            <div className="flex items-center gap-1.5 text-xs truncate">
                              <span className="text-rose-500 font-bold line-through truncate max-w-[120px]">{rule.matchPattern}</span>
                              <MingCuteIcon name="arrow_right_line" size={11} className="text-muted-foreground shrink-0" />
                              <span className="text-emerald-500 font-bold truncate max-w-[120px]">{rule.replacementValue || "(empty)"}</span>
                              {rule.isRegex && <span className="text-[9px] px-1 bg-neutral-subtle rounded text-muted-foreground">regex</span>}
                            </div>
                          )}
                        </td>

                        {/* Row Actions */}
                        <td className="px-3 py-2 text-right">
                          <div className="flex items-center justify-end gap-1">
                            <button
                              onClick={() => openRuleModal(rule)}
                              className="p-1 text-muted-foreground hover:text-foreground hover:bg-neutral-subtle rounded transition-colors cursor-pointer"
                              title="Edit Rule"
                            >
                              <MingCuteIcon name="edit_line" size={14} />
                            </button>
                            <button
                              onClick={() => duplicateRule(rule.id)}
                              className="p-1 text-muted-foreground hover:text-foreground hover:bg-neutral-subtle rounded transition-colors cursor-pointer"
                              title="Duplicate Rule"
                            >
                              <MingCuteIcon name="copy_line" size={14} />
                            </button>
                            <button
                              onClick={() => deleteRule(rule.id)}
                              className="p-1 text-muted-foreground hover:text-rose-500 hover:bg-rose-500/10 rounded transition-colors cursor-pointer"
                              title="Delete Rule"
                            >
                              <MingCuteIcon name="delete_2_line" size={14} />
                            </button>
                          </div>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            )}
          </div>
        ) : (
          /* Execution History View (Split Table & Diff Inspector) */
          <div className="h-full flex overflow-hidden">
            {/* Left Pane: Log List */}
            <div className="w-1/2 flex flex-col border-r border-border bg-surface shrink-0">
              <div className="px-3 py-2 bg-header border-b border-border flex items-center justify-between text-xs font-semibold text-muted-foreground uppercase font-mono">
                <span>Captured Rewrites</span>
                <span className="px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-600 dark:text-emerald-400 font-bold text-[10px]">
                  {filteredLogs.length}
                </span>
              </div>

              <div className="flex-1 overflow-y-auto divide-y divide-border/60">
                {filteredLogs.length === 0 ? (
                  <div className="flex flex-col items-center justify-center h-full p-6 text-center text-muted-foreground font-sans">
                    <MingCuteIcon name="history_line" size={32} className="mb-2 opacity-30" />
                    <p className="text-xs font-semibold">No rewrite history captured yet</p>
                    <p className="text-[11px] text-muted-foreground mt-1">
                      When requests or responses match active rewrite rules, they are automatically logged here.
                    </p>
                  </div>
                ) : (
                  filteredLogs.map((log) => {
                    const isSelected = selectedLog?.id === log.id;
                    return (
                      <div
                        key={log.id}
                        onClick={() => selectLog(log.id)}
                        className={`p-3 cursor-pointer transition-colors border-l-2 font-mono text-xs ${
                          isSelected
                            ? "bg-emerald-500/10 border-emerald-500"
                            : "border-transparent hover:bg-neutral-subtle"
                        }`}
                      >
                        <div className="flex items-center justify-between mb-1">
                          <div className="flex items-center gap-1.5">
                            <span className="px-1.5 py-0.5 rounded bg-header border border-border text-emerald-600 dark:text-emerald-400 font-bold text-[10px]">
                              {log.method}
                            </span>
                            <span className="font-sans font-bold text-foreground text-xs truncate max-w-[160px]">
                              {log.ruleName}
                            </span>
                          </div>
                          <span className="text-[10px] text-muted-foreground">
                            {log.createdAt.split(" ")[1] || log.createdAt}
                          </span>
                        </div>

                        {/* URL Summary */}
                        <div className="text-xs truncate">
                          <span className="text-muted-foreground line-through opacity-75">{log.originalUrl}</span>
                        </div>
                        <div className="text-xs text-emerald-600 dark:text-emerald-400 font-bold truncate">
                          {log.rewrittenUrl}
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </div>

            {/* Right Pane: Diff Inspector */}
            <div className="flex-1 flex flex-col bg-background min-w-0 overflow-y-auto p-4 font-mono text-xs space-y-4">
              {selectedLog ? (
                <>
                  {/* Top Details */}
                  <div className="p-3 bg-header border border-border rounded-lg flex items-center justify-between">
                    <div>
                      <div className="flex items-center gap-2">
                        <span className="px-2 py-0.5 rounded bg-emerald-500/20 text-emerald-600 dark:text-emerald-400 font-bold">
                          {selectedLog.method}
                        </span>
                        <span className="font-sans font-bold text-sm text-foreground">
                          {selectedLog.ruleName}
                        </span>
                        {getActionBadge(selectedLog.actionType)}
                      </div>
                      <div className="text-[11px] text-muted-foreground mt-1">
                        Executed at: {selectedLog.createdAt}
                      </div>
                    </div>

                    {selectedLog.statusCode && (
                      <div className="px-3 py-1 rounded bg-surface border border-border text-center">
                        <div className="text-[10px] text-muted-foreground uppercase font-sans">Status</div>
                        <div className="font-bold text-emerald-500">{selectedLog.statusCode}</div>
                      </div>
                    )}
                  </div>

                  {/* URL Comparison */}
                  <div className="space-y-1.5 p-3 rounded-lg bg-surface border border-border">
                    <div className="text-[11px] font-bold text-muted-foreground font-sans uppercase">
                      URL Transformation
                    </div>
                    <div className="space-y-1">
                      <div className="p-2 rounded bg-rose-500/10 border border-rose-500/20 text-rose-600 dark:text-rose-400 break-all select-all">
                        <span className="font-sans font-semibold text-[10px] text-muted-foreground block">ORIGINAL:</span>
                        {selectedLog.originalUrl}
                      </div>
                      <div className="p-2 rounded bg-emerald-500/10 border border-emerald-500/20 text-emerald-600 dark:text-emerald-400 break-all select-all font-bold">
                        <span className="font-sans font-semibold text-[10px] text-muted-foreground block">REWRITTEN:</span>
                        {selectedLog.rewrittenUrl}
                      </div>
                    </div>
                  </div>

                  {/* Headers Comparison if modified */}
                  {JSON.stringify(selectedLog.originalHeaders) !== JSON.stringify(selectedLog.rewrittenHeaders) && (
                    <div className="space-y-1.5 p-3 rounded-lg bg-surface border border-border">
                      <div className="text-[11px] font-bold text-muted-foreground font-sans uppercase">
                        Headers Transformation
                      </div>
                      <div className="grid grid-cols-2 gap-2">
                        <div className="p-2 rounded bg-background border border-border">
                          <span className="font-sans font-semibold text-[10px] text-muted-foreground block mb-1">ORIGINAL HEADERS:</span>
                          <pre className="text-[11px] text-muted-foreground overflow-x-auto whitespace-pre-wrap">
                            {selectedLog.originalHeaders.map(([k, v]) => `${k}: ${v}`).join("\n") || "(none)"}
                          </pre>
                        </div>
                        <div className="p-2 rounded bg-background border border-emerald-500/30">
                          <span className="font-sans font-semibold text-[10px] text-emerald-500 block mb-1">REWRITTEN HEADERS:</span>
                          <pre className="text-[11px] text-emerald-600 dark:text-emerald-400 overflow-x-auto whitespace-pre-wrap">
                            {selectedLog.rewrittenHeaders.map(([k, v]) => `${k}: ${v}`).join("\n") || "(none)"}
                          </pre>
                        </div>
                      </div>
                    </div>
                  )}

                  {/* Body Comparison if modified */}
                  {(selectedLog.originalBody || selectedLog.rewrittenBody) && (
                    <div className="space-y-1.5 p-3 rounded-lg bg-surface border border-border flex-1 flex flex-col min-h-[220px]">
                      <div className="text-[11px] font-bold text-muted-foreground font-sans uppercase">
                        Body Payload Transformation
                      </div>
                      <div className="grid grid-cols-2 gap-2 flex-1">
                        <div className="p-2 rounded bg-background border border-border overflow-y-auto">
                          <span className="font-sans font-semibold text-[10px] text-muted-foreground block mb-1">ORIGINAL BODY:</span>
                          <pre className="text-[11px] text-muted-foreground whitespace-pre-wrap">
                            {selectedLog.originalBody || "(empty)"}
                          </pre>
                        </div>
                        <div className="p-2 rounded bg-background border border-emerald-500/30 overflow-y-auto">
                          <span className="font-sans font-semibold text-[10px] text-emerald-500 block mb-1">REWRITTEN BODY:</span>
                          <pre className="text-[11px] text-emerald-600 dark:text-emerald-400 whitespace-pre-wrap">
                            {selectedLog.rewrittenBody || "(empty)"}
                          </pre>
                        </div>
                      </div>
                    </div>
                  )}
                </>
              ) : (
                <div className="flex flex-col items-center justify-center h-full text-muted-foreground p-6 text-center font-sans">
                  <MingCuteIcon name="file_search_line" size={36} className="mb-2 opacity-30" />
                  <p className="text-xs font-semibold">Select a log entry from the list to view rewrite details</p>
                </div>
              )}
            </div>
          </div>
        )}
      </div>

      {/* Add / Edit Rule Modal */}
      <RewriteRuleModal />
    </div>
  );
};
