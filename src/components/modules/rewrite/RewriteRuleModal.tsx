import React, { useState, useEffect } from "react";
import { useRewriteStore } from "../../../stores/useRewriteStore";
import { MingCuteIcon } from "../../common/MingCuteIcon";
import { Select, Button, Checkbox, Dialog } from "../../common/ui";

const ACTION_TYPE_OPTIONS = [
  { value: "partial_request", label: "1. Partial Request Rewrite (Fix typo, URL/header/body replace)" },
  { value: "full_request", label: "2. Full Request Change (Forward to new URL / Override Body or Method)" },
  { value: "redirect", label: "3. HTTP Redirect (301/302/307/308 Location redirect to new URL)" },
  { value: "partial_response", label: "4. Partial Response Rewrite (Response body/header replace)" },
  { value: "full_response", label: "5. Full Response Change (Mock Status / Headers / Body)" },
] as const;

const REDIRECT_STATUS_OPTIONS = [
  { value: "307", label: "307 Temporary Redirect (Recommended - Preserves HTTP Method)" },
  { value: "302", label: "302 Found (Standard Browser Redirect)" },
  { value: "301", label: "301 Moved Permanently (Permanent Redirect)" },
  { value: "308", label: "308 Permanent Redirect (Preserves HTTP Method)" },
] as const;

const MATCH_FIELD_OPTIONS = [
  { value: "all", label: "All Requests (*)" },
  { value: "url", label: "URL" },
  { value: "host", label: "Host" },
  { value: "path", label: "Path" },
  { value: "method", label: "Method" },
  { value: "header", label: "Header" },
] as const;

const MATCH_OPERATOR_OPTIONS = [
  { value: "contains", label: "Contains" },
  { value: "equals", label: "Equals" },
  { value: "starts_with", label: "Starts With" },
  { value: "ends_with", label: "Ends With" },
  { value: "regex", label: "Regular Expression (Regex)" },
] as const;

export const RewriteRuleModal: React.FC = () => {
  const { isRuleModalOpen, closeRuleModal, editingRule, addOrUpdateRule } = useRewriteStore();

  const [name, setName] = useState("");
  const [actionType, setActionType] = useState<"partial_request" | "full_request" | "redirect" | "partial_response" | "full_response">("partial_request");
  
  const [matchField, setMatchField] = useState<"all" | "url" | "host" | "path" | "method" | "header">("url");
  const [matchOperator, setMatchOperator] = useState<"contains" | "equals" | "regex" | "starts_with">("contains");
  const [matchValue, setMatchValue] = useState("");
  
  const [targetPart, setTargetPart] = useState<"url" | "query" | "header" | "body" | "method" | "status">("url");
  const [targetHeader, setTargetHeader] = useState("");
  const [matchPattern, setMatchPattern] = useState("");
  const [replacementValue, setReplacementValue] = useState("");
  const [isRegex, setIsRegex] = useState(false);

  const [mockStatusCode, setMockStatusCode] = useState<number>(200);
  const [mockHeadersJson, setMockHeadersJson] = useState<string>('[\n  ["Content-Type", "application/json"],\n  ["Access-Control-Allow-Origin", "*"]\n]');
  const [mockBody, setMockBody] = useState<string>('{\n  "status": "success",\n  "data": []\n}');

  // Sandbox Tester State
  const [testInput, setTestInput] = useState("");
  const [testResult, setTestResult] = useState("");

  useEffect(() => {
    if (editingRule) {
      setName(editingRule.name);
      setActionType(editingRule.actionType);
      setMatchField(editingRule.matchField);
      setMatchOperator(editingRule.matchOperator);
      setMatchValue(editingRule.matchValue);
      setTargetPart(editingRule.targetPart);
      setTargetHeader(editingRule.targetHeader || "");
      setMatchPattern(editingRule.matchPattern);
      setReplacementValue(editingRule.replacementValue);
      setIsRegex(editingRule.isRegex);
      setMockStatusCode(editingRule.mockStatusCode || (editingRule.actionType === "redirect" ? 307 : 200));
      setMockHeadersJson(editingRule.mockHeadersJson || '[\n  ["Content-Type", "application/json"]\n]');
      setMockBody(editingRule.mockBody || "");
      if (editingRule.actionType === "redirect") {
        setTestInput(editingRule.matchPattern ? `${editingRule.matchPattern}/dashboard?step=1` : "https://form.duluin.com/submit?id=123");
      } else {
        setTestInput(editingRule.matchPattern ? `https://api.example.com${editingRule.matchPattern}sample` : "https://api.example.com/v1/v1/users");
      }
    } else {
      setName("");
      setActionType("partial_request");
      setMatchField("url");
      setMatchOperator("contains");
      setMatchValue("");
      setTargetPart("url");
      setTargetHeader("");
      setMatchPattern("");
      setReplacementValue("");
      setIsRegex(false);
      setMockStatusCode(200);
      setMockHeadersJson('[\n  ["Content-Type", "application/json"],\n  ["Access-Control-Allow-Origin", "*"]\n]');
      setMockBody('{\n  "status": "success",\n  "message": "Mocked response from Rewrite Engine"\n}');
      setTestInput("https://api.example.com/v1/v1/users");
    }
  }, [editingRule, isRuleModalOpen]);

  // Live Test Sandbox computation
  useEffect(() => {
    if (!testInput) {
      setTestResult("");
      return;
    }

    try {
      if (actionType === "partial_request" || actionType === "partial_response") {
        if (isRegex && matchPattern) {
          const re = new RegExp(matchPattern, "g");
          setTestResult(testInput.replace(re, replacementValue));
        } else if (matchPattern) {
          setTestResult(testInput.split(matchPattern).join(replacementValue));
        } else {
          setTestResult(testInput);
        }
      } else if (actionType === "redirect") {
        let finalUrl = testInput;
        if (isRegex && matchPattern) {
          const re = new RegExp(matchPattern, "g");
          finalUrl = testInput.replace(re, replacementValue);
        } else if (matchPattern && testInput.includes(matchPattern)) {
          finalUrl = testInput.split(matchPattern).join(replacementValue);
        } else if (replacementValue) {
          finalUrl = replacementValue;
        }
        setTestResult(`[HTTP ${mockStatusCode || 307} Redirect]\nLocation: ${finalUrl}`);
      } else if (actionType === "full_request") {
        if (targetPart === "url") {
          setTestResult(replacementValue || "Destination URL");
        } else if (targetPart === "body") {
          setTestResult(replacementValue || "Full body replaced");
        } else {
          setTestResult(replacementValue || "Method replaced");
        }
      } else if (actionType === "full_response") {
        setTestResult(`[HTTP ${mockStatusCode}]\n${mockBody}`);
      }
    } catch (e: any) {
      setTestResult(`Regex Error: ${e.message}`);
    }
  }, [testInput, actionType, targetPart, matchPattern, replacementValue, isRegex, mockStatusCode, mockBody]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    await addOrUpdateRule({
      id: editingRule?.id,
      name: name.trim(),
      enabled: editingRule?.enabled ?? true,
      actionType,
      matchField,
      matchOperator,
      matchValue: matchValue.trim(),
      targetPart,
      targetHeader: targetHeader.trim() || undefined,
      matchPattern: matchPattern.trim(),
      replacementValue,
      isRegex,
      mockStatusCode: (actionType === "full_response" || actionType === "redirect") ? (mockStatusCode || (actionType === "redirect" ? 307 : 200)) : undefined,
      mockHeadersJson: actionType === "full_response" ? mockHeadersJson : undefined,
      mockBody: actionType === "full_response" ? mockBody : undefined,
    });
  };

  const getTargetPartOptions = () => {
    if (actionType === "partial_request") {
      return [
        { value: "url", label: "URL / Path (Find & Replace Substring or Regex)" },
        { value: "query", label: "Query Parameter (Add / Replace Query Param)" },
        { value: "header", label: "Request Header (Modify / Inject / Remove Header)" },
        { value: "body", label: "Request Body (Find & Replace String/Regex in Body)" },
      ];
    } else if (actionType === "full_request") {
      return [
        { value: "url", label: "Destination URL (Redirect whole URL to new target)" },
        { value: "body", label: "Full Request Body (Override entire request body)" },
        { value: "method", label: "HTTP Method (Override method, e.g. POST -> PUT)" },
      ];
    } else if (actionType === "partial_response") {
      return [
        { value: "body", label: "Response Body (Find & Replace String/Regex in Response)" },
        { value: "header", label: "Response Header (Modify / Inject / Remove Header)" },
      ];
    } else {
      return [
        { value: "status", label: "Full Mock Response (Status + Headers + Body)" },
      ];
    }
  };

  return (
    <Dialog
      isOpen={isRuleModalOpen}
      onClose={closeRuleModal}
      title={editingRule ? "Edit Rewrite Rule" : "Create Automated Rewrite Rule"}
      size="xl"
    >
      <form onSubmit={handleSubmit} className="space-y-4 font-sans text-xs">
        {/* Rule Name & Action Type */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
          <div className="md:col-span-1 space-y-1">
            <label className="block text-2xs font-semibold text-muted-foreground uppercase tracking-wider">
              Rule Name <span className="text-rose-500">*</span>
            </label>
            <input
              type="text"
              required
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="e.g. Fix /v1/v1 typo"
              className="w-full bg-surface border border-border focus:border-primary rounded-lg px-3 py-1.5 text-foreground outline-none text-xs"
            />
          </div>

          <div className="md:col-span-2 space-y-1">
            <label className="block text-2xs font-semibold text-muted-foreground uppercase tracking-wider">
              Rewrite Type
            </label>
            <Select
              value={actionType}
              onChange={(e) => {
                const newAction = e.target.value as any;
                setActionType(newAction);
                if (newAction === "partial_request" || newAction === "full_request") {
                  setTargetPart("url");
                } else if (newAction === "redirect") {
                  setTargetPart("url");
                  setMockStatusCode(307);
                } else if (newAction === "partial_response") {
                  setTargetPart("body");
                } else {
                  setTargetPart("status");
                  setMockStatusCode(200);
                }
              }}
              options={ACTION_TYPE_OPTIONS}
              sizeVariant="sm"
            />
          </div>
        </div>

        {/* Match Condition Box */}
        <div className="p-3 rounded-lg bg-surface border border-border space-y-2">
          <div className="flex items-center gap-1.5 text-xs font-bold text-foreground">
            <MingCuteIcon name="filter_line" size={14} className="text-amber-500" />
            <span>1. Match Traffic Filter</span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-3 gap-2">
            <div>
              <label className="block text-3xs text-muted-foreground mb-1">Target Field</label>
              <Select
                value={matchField}
                onChange={(e) => setMatchField(e.target.value as any)}
                options={MATCH_FIELD_OPTIONS}
                sizeVariant="sm"
              />
            </div>

            <div>
              <label className="block text-3xs text-muted-foreground mb-1">Operator</label>
              <Select
                value={matchOperator}
                onChange={(e) => setMatchOperator(e.target.value as any)}
                options={MATCH_OPERATOR_OPTIONS}
                sizeVariant="sm"
              />
            </div>

            <div>
              <label className="block text-3xs text-muted-foreground mb-1">Filter Pattern</label>
              <input
                type="text"
                value={matchValue}
                onChange={(e) => setMatchValue(e.target.value)}
                placeholder={matchField === "all" ? "Applies to all traffic" : "e.g. api.example.com"}
                disabled={matchField === "all"}
                className="w-full bg-background border border-border focus:border-primary rounded px-2.5 py-1 text-foreground outline-none font-mono text-xs disabled:opacity-40"
              />
            </div>
          </div>
        </div>

        {/* Transformation / Replacement Box */}
        <div className="p-3 rounded-lg bg-surface border border-border space-y-3">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-1.5 text-xs font-bold text-foreground">
              <MingCuteIcon name="swap_line" size={14} className="text-primary" />
              <span>2. Rewrite Transformation</span>
            </div>

            {(actionType === "partial_request" || actionType === "partial_response" || actionType === "redirect") && (
              <Checkbox
                label="Use Regular Expression (Regex)"
                checked={isRegex}
                onChange={(e) => setIsRegex(e.target.checked)}
              />
            )}
          </div>

          {/* Target Part Selector */}
          {actionType !== "full_response" && actionType !== "redirect" && (
            <div>
              <label className="block text-3xs text-muted-foreground mb-1">Target Component</label>
              <Select
                value={targetPart}
                onChange={(e) => setTargetPart(e.target.value as any)}
                options={getTargetPartOptions()}
                sizeVariant="sm"
              />
            </div>
          )}

          {/* Header Key Field if Header action */}
          {targetPart === "header" && actionType !== "full_response" && actionType !== "redirect" && (
            <div>
              <label className="block text-3xs text-muted-foreground mb-1">Header Name</label>
              <input
                type="text"
                value={targetHeader}
                onChange={(e) => setTargetHeader(e.target.value)}
                placeholder="e.g. Authorization, Access-Control-Allow-Origin, User-Agent"
                className="w-full bg-background border border-border focus:border-primary rounded px-2.5 py-1.5 text-foreground outline-none font-mono text-xs"
              />
            </div>
          )}

          {/* HTTP Redirect Configuration */}
          {actionType === "redirect" && (
            <div className="space-y-3">
              <div>
                <label className="block text-3xs text-muted-foreground mb-1">
                  HTTP Redirect Status Code
                </label>
                <Select
                  value={String(mockStatusCode || 307)}
                  onChange={(e) => setMockStatusCode(parseInt(e.target.value) || 307)}
                  options={REDIRECT_STATUS_OPTIONS}
                  sizeVariant="sm"
                />
              </div>

              <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                <div className="space-y-1">
                  <label className="block text-3xs text-muted-foreground">
                    URL Pattern to Replace (Optional)
                  </label>
                  <input
                    type="text"
                    value={matchPattern}
                    onChange={(e) => setMatchPattern(e.target.value)}
                    placeholder="e.g. https://form.duluin.com (leave blank for exact redirect)"
                    className="w-full bg-background border border-border focus:border-primary rounded px-2.5 py-1.5 text-foreground outline-none font-mono text-xs"
                  />
                  <p className="text-3xs text-muted-foreground">
                    If set, replaces this substring in URL while preserving paths/query params.
                  </p>
                </div>

                <div className="space-y-1">
                  <label className="block text-3xs text-muted-foreground">
                    Redirect Destination URL <span className="text-rose-500">*</span>
                  </label>
                  <input
                    type="text"
                    required
                    value={replacementValue}
                    onChange={(e) => setReplacementValue(e.target.value)}
                    placeholder="e.g. https://dev-form.duluin.id"
                    className="w-full bg-background border border-border focus:border-primary rounded px-2.5 py-1.5 text-foreground outline-none font-mono text-xs"
                  />
                  <p className="text-3xs text-muted-foreground">
                    The replacement domain or full destination URL sent in the <code>Location</code> header.
                  </p>
                </div>
              </div>
            </div>
          )}

          {/* String/Regex Replacement Fields */}
          {(actionType === "partial_request" || actionType === "partial_response") && (
            <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
              <div className="space-y-1">
                <label className="block text-3xs text-muted-foreground">
                  Find String / Pattern to Replace <span className="text-rose-500">*</span>
                </label>
                <input
                  type="text"
                  required
                  value={matchPattern}
                  onChange={(e) => setMatchPattern(e.target.value)}
                  placeholder={targetPart === "url" ? "e.g. /v1/v1/" : targetPart === "query" ? "param_name" : "find_text"}
                  className="w-full bg-background border border-border focus:border-primary rounded px-2.5 py-1.5 text-foreground outline-none font-mono text-xs"
                />
              </div>

              <div className="space-y-1">
                <label className="block text-3xs text-muted-foreground">
                  Replace With
                </label>
                <input
                  type="text"
                  value={replacementValue}
                  onChange={(e) => setReplacementValue(e.target.value)}
                  placeholder={targetPart === "url" ? "e.g. /v1/" : targetPart === "header" ? "Bearer new_token (leave empty to delete)" : "replacement_text"}
                  className="w-full bg-background border border-border focus:border-primary rounded px-2.5 py-1.5 text-foreground outline-none font-mono text-xs"
                />
              </div>
            </div>
          )}

          {/* Full Request Replacement Fields */}
          {actionType === "full_request" && (
            <div className="space-y-1">
              <label className="block text-3xs text-muted-foreground">
                {targetPart === "url"
                  ? "New Destination Target URL (Full Redirect)"
                  : targetPart === "body"
                  ? "Full Replacement Request Body"
                  : "New HTTP Method (e.g. POST, PUT, GET)"}
              </label>
              {targetPart === "body" ? (
                <textarea
                  value={replacementValue}
                  onChange={(e) => setReplacementValue(e.target.value)}
                  rows={4}
                  placeholder="Enter full request body payload..."
                  className="w-full bg-background border border-border focus:border-primary rounded p-2.5 text-foreground outline-none font-mono text-xs resize-none"
                />
              ) : (
                <input
                  type="text"
                  required
                  value={replacementValue}
                  onChange={(e) => setReplacementValue(e.target.value)}
                  placeholder={targetPart === "url" ? "http://127.0.0.1:3000/api/endpoint" : "POST"}
                  className="w-full bg-background border border-border focus:border-primary rounded px-2.5 py-1.5 text-foreground outline-none font-mono text-xs"
                />
              )}
            </div>
          )}

          {/* Full Response / Mocking Fields */}
          {actionType === "full_response" && (
            <div className="space-y-3">
              <div className="w-36">
                <label className="block text-3xs text-muted-foreground mb-1">Mock HTTP Status</label>
                <input
                  type="number"
                  value={mockStatusCode}
                  onChange={(e) => setMockStatusCode(parseInt(e.target.value) || 200)}
                  className="w-full bg-background border border-border focus:border-primary rounded px-2.5 py-1 text-foreground outline-none font-mono text-xs"
                />
              </div>

              <div>
                <label className="block text-3xs text-muted-foreground mb-1">Mock Headers (JSON array)</label>
                <textarea
                  value={mockHeadersJson}
                  onChange={(e) => setMockHeadersJson(e.target.value)}
                  rows={3}
                  className="w-full bg-background border border-border focus:border-primary rounded p-2 text-foreground outline-none font-mono text-2xs resize-none"
                />
              </div>

              <div>
                <label className="block text-3xs text-muted-foreground mb-1">Mock Response Body</label>
                <textarea
                  value={mockBody}
                  onChange={(e) => setMockBody(e.target.value)}
                  rows={4}
                  placeholder='{"status": "success"}'
                  className="w-full bg-background border border-border focus:border-primary rounded p-2 text-foreground outline-none font-mono text-xs resize-none"
                />
              </div>
            </div>
          )}
        </div>

        {/* Live Interactive Sandbox Tester */}
        <div className="p-3 rounded-lg bg-neutral-subtle/50 border border-dashed border-border space-y-2 font-mono text-xs">
          <div className="flex items-center gap-1.5 text-2xs font-bold text-muted-foreground font-sans uppercase tracking-wide">
            <MingCuteIcon name="flask_line" size={13} className="text-emerald-500" />
            <span>Interactive Live Sandbox Preview</span>
          </div>

          <div className="grid grid-cols-1 md:grid-cols-2 gap-2">
            <div className="space-y-1">
              <span className="text-3xs text-muted-foreground font-sans">Sample Input:</span>
              <input
                type="text"
                value={testInput}
                onChange={(e) => setTestInput(e.target.value)}
                placeholder="Enter sample input to test replacement..."
                className="w-full bg-surface border border-border rounded px-2 py-1 text-foreground outline-none text-xs"
              />
            </div>

            <div className="space-y-1">
              <span className="text-3xs text-emerald-500 font-sans font-bold">Rewritten Result Preview:</span>
              <div className="w-full bg-surface border border-emerald-500/30 text-emerald-600 dark:text-emerald-400 rounded px-2 py-1 text-xs truncate select-all">
                {testResult || <span className="opacity-40 italic">Result preview</span>}
              </div>
            </div>
          </div>
        </div>

        {/* Footer Actions */}
        <div className="flex items-center justify-end gap-2 pt-2 border-t border-border">
          <Button
            type="button"
            variant="secondary"
            sizeVariant="sm"
            onClick={closeRuleModal}
          >
            Cancel
          </Button>
          <Button
            type="submit"
            variant="primary"
            sizeVariant="sm"
            icon="check_line"
          >
            {editingRule ? "Save Changes" : "Create Rule"}
          </Button>
        </div>
      </form>
    </Dialog>
  );
};
