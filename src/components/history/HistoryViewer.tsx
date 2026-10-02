import React, { useEffect, useState } from "react";
import { useHistoryStore } from "../../stores/useHistoryStore";
import { MingCuteIcon } from "../common/MingCuteIcon";
import { ProxyPowerButton } from "../common/ProxyPowerButton";

export const HistoryViewer: React.FC = () => {
  const {
    logs,
    searchTerm,
    methodFilter,
    selectedLogId,
    limiterEnabled,
    maxRows,
    settingsModalOpen,
    isLoading,
    fetchLogs,
    setSearchTerm,
    setMethodFilter,
    selectLog,
    clearLogs,
    fetchProxyStatus,
    fetchHistorySettings,
    setHistoryLimiter,
    setSettingsModalOpen,
    initSubscription,
  } = useHistoryStore();

  const selectedLogDetail = logs.find((l) => l.id === selectedLogId) || null;

  const [activeDetailTab, setActiveDetailTab] = useState<"request" | "response">("request");
  const [tempLimiterEnabled, setTempLimiterEnabled] = useState(limiterEnabled);
  const [tempMaxRows, setTempMaxRows] = useState(maxRows);

  useEffect(() => {
    fetchProxyStatus();
    fetchHistorySettings();
    fetchLogs();
    initSubscription();
  }, []);

  useEffect(() => {
    setTempLimiterEnabled(limiterEnabled);
    setTempMaxRows(maxRows);
  }, [limiterEnabled, maxRows]);

  const handleSaveSettings = async (e: React.FormEvent) => {
    e.preventDefault();
    await setHistoryLimiter(tempLimiterEnabled, tempMaxRows);
    setSettingsModalOpen(false);
  };

  const formatBytes = (bytes: number) => {
    if (!bytes || bytes === 0) return "0 B";
    const k = 1024;
    const sizes = ["B", "KB", "MB", "GB"];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(1))} ${sizes[i]}`;
  };

  const getStatusBadgeClass = (status: number) => {
    if (status >= 200 && status < 300)
      return "bg-emerald-500/10 text-emerald-400 border-emerald-500/20";
    if (status >= 300 && status < 400)
      return "bg-blue-500/10 text-blue-400 border-blue-500/20";
    if (status >= 400 && status < 500)
      return "bg-amber-500/10 text-amber-400 border-amber-500/20";
    return "bg-rose-500/10 text-rose-400 border-rose-500/20";
  };

  const getMethodBadgeClass = (method: string) => {
    switch (method.toUpperCase()) {
      case "GET":
        return "bg-sky-500/10 text-sky-400 border-sky-500/20";
      case "POST":
        return "bg-emerald-500/10 text-emerald-400 border-emerald-500/20";
      case "PUT":
        return "bg-amber-500/10 text-amber-400 border-amber-500/20";
      case "DELETE":
        return "bg-rose-500/10 text-rose-400 border-rose-500/20";
      case "PATCH":
        return "bg-purple-500/10 text-purple-400 border-purple-500/20";
      default:
        return "bg-zinc-500/10 text-zinc-400 border-zinc-500/20";
    }
  };

  const renderBodyContent = (body: string, contentType?: string) => {
    if (!body || body.trim() === "") return "(empty)";

    if (body.startsWith("base64:")) {
      const rawB64 = body.substring(7);
      const mime = contentType || "image/png";
      if (mime.includes("image/")) {
        return (
          <div className="flex flex-col items-center gap-2 py-2">
            <img
              src={`data:${mime};base64,${rawB64}`}
              alt="Response Preview"
              className="max-h-64 rounded border border-zinc-800 object-contain"
            />
            <span className="text-3xs text-zinc-500 font-mono">Base64 Image Preview ({mime})</span>
          </div>
        );
      }
      return (
        <div className="p-2 bg-zinc-900 rounded text-zinc-400 font-mono text-2xs">
          [Binary Content ({mime}): {rawB64.length} chars base64]
        </div>
      );
    }

    try {
      const parsed = JSON.parse(body);
      return JSON.stringify(parsed, null, 2);
    } catch {
      return body;
    }
  };

  return (
    <div className="flex flex-col h-screen w-full bg-zinc-950 text-zinc-100 font-sans antialiased overflow-hidden">
      {/* Top Toolbar / Control Header */}
      <header className="flex items-center justify-between px-4 py-3 bg-zinc-900/90 border-b border-zinc-800 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <MingCuteIcon name="history_line" size={18} className="text-indigo-400" />
            <h1 className="text-lg font-bold tracking-tight bg-gradient-to-r from-indigo-400 to-cyan-400 bg-clip-text text-transparent">
              Traffic Inspector
            </h1>
          </div>

          <div className="h-4 w-px bg-zinc-800 mx-1" />

          {/* Proxy Power Control Button with Overlay */}
          <ProxyPowerButton />

          {/* History Rotation Limiter Badge & Button */}
          <button
            onClick={() => setSettingsModalOpen(true)}
            className={`flex items-center gap-1.5 px-2.5 py-1 rounded-md text-2xs font-semibold transition-all border ${
              limiterEnabled
                ? "bg-indigo-500/10 text-indigo-300 border-indigo-500/30 hover:bg-indigo-500/20"
                : "bg-zinc-800 text-zinc-400 border-zinc-700 hover:bg-zinc-700"
            }`}
            title="Configure History Log Rotation Limiter"
          >
            <MingCuteIcon name="storage_line" size={13} className={limiterEnabled ? "text-indigo-400" : "text-zinc-500"} />
            <span>Limit: {limiterEnabled ? `${maxRows} max` : "Disabled"}</span>
          </button>
        </div>

        {/* Filter Controls & Search */}
        <div className="flex items-center gap-3">
          <div className="relative flex items-center">
            <MingCuteIcon name="search_line" size={14} className="absolute left-3 text-zinc-400 pointer-events-none" />
            <input
              type="text"
              placeholder="Search URL, Host, Path..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="bg-zinc-950 border border-zinc-800 focus:border-indigo-500/60 focus:ring-1 focus:ring-indigo-500/60 rounded-lg text-xs pl-9 pr-3 py-1.5 w-64 text-zinc-200 placeholder-zinc-500 outline-none transition-all"
            />
          </div>

          {/* Method Filter Dropdown */}
          <select
            value={methodFilter}
            onChange={(e) => setMethodFilter(e.target.value)}
            className="bg-zinc-950 border border-zinc-800 focus:border-indigo-500/60 rounded-lg text-xs px-2.5 py-1.5 text-zinc-300 outline-none cursor-pointer"
          >
            <option value="ALL">All Methods</option>
            <option value="GET">GET</option>
            <option value="POST">POST</option>
            <option value="PUT">PUT</option>
            <option value="DELETE">DELETE</option>
            <option value="PATCH">PATCH</option>
          </select>

          <button
            onClick={fetchLogs}
            className="p-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 rounded-lg transition-colors"
            title="Refresh Logs"
          >
            <MingCuteIcon name="refresh_line" size={15} className={isLoading ? "animate-spin" : ""} />
          </button>

          <button
            onClick={clearLogs}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-rose-500/10 border border-rose-500/20 hover:bg-rose-500/20 text-rose-400 text-xs rounded-lg transition-colors font-medium"
            title="Clear History"
          >
            <MingCuteIcon name="delete_2_line" size={14} /> Clear
          </button>
        </div>
      </header>

      {/* Main Split-Pane Body */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left / Top: Traffic Table */}
        <div className="flex-1 flex flex-col border-r border-zinc-800 bg-zinc-950/50 min-w-0">
          <div className="overflow-y-auto flex-1">
            <table className="w-full text-left text-xs border-collapse table-fixed">
              <thead className="bg-zinc-900 sticky top-0 border-b border-zinc-800 text-zinc-400 font-medium uppercase tracking-wider backdrop-blur-sm z-10">
                <tr>
                  <th className="py-2.5 px-3 w-10 text-zinc-500 font-mono">#</th>
                  <th className="py-2.5 px-3 w-16">Status</th>
                  <th className="py-2.5 px-3 w-20">Method</th>
                  <th className="py-2.5 px-3 w-48">Host</th>
                  <th className="py-2.5 px-3">Path</th>
                  <th className="py-2.5 px-3 w-36">Content Type</th>
                  <th className="py-2.5 px-3 w-20 text-right">Size</th>
                  <th className="py-2.5 px-3 w-20 text-right">Time</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-zinc-900/60 font-mono text-2xs">
                {logs.length === 0 ? (
                  <tr>
                    <td colSpan={8} className="text-center py-16 text-zinc-500 text-sm">
                      No traffic captured yet. Start proxy and send HTTP requests.
                    </td>
                  </tr>
                ) : (
                  logs.map((item, index) => {
                    const isSelected = selectedLogId === item.id;
                    return (
                      <tr
                        key={item.id}
                        onClick={() => selectLog(item.id)}
                        className={`cursor-pointer transition-colors ${
                          isSelected
                            ? "bg-indigo-600/15 border-l-2 border-indigo-500"
                            : "hover:bg-zinc-900/40"
                        }`}
                      >
                        <td className="py-2 px-3 text-zinc-600 font-mono">{logs.length - index}</td>
                        <td className="py-2 px-3">
                          <span
                            className={`inline-block px-2 py-0.5 rounded border text-3xs font-mono font-bold ${getStatusBadgeClass(
                              item.statusCode
                            )}`}
                          >
                            {item.statusCode}
                          </span>
                        </td>
                        <td className="py-2 px-3">
                          <span
                            className={`inline-block px-1.5 py-0.5 rounded border text-3xs font-mono font-semibold ${getMethodBadgeClass(
                              item.method
                            )}`}
                          >
                            {item.method}
                          </span>
                        </td>
                        <td className="py-2 px-3 text-indigo-300 overflow-hidden">
                          <div className="truncate" title={item.host}>{item.host}</div>
                        </td>
                        <td className="py-2 px-3 text-zinc-300 overflow-hidden">
                          <div className="truncate" title={item.path}>{item.path}</div>
                        </td>
                        <td className="py-2 px-3 text-zinc-400 overflow-hidden">
                          <div className="truncate" title={item.contentType}>{item.contentType || "-"}</div>
                        </td>
                        <td className="py-2 px-3 text-right text-zinc-400">
                          {formatBytes(item.responseSize)}
                        </td>
                        <td className="py-2 px-3 text-right text-zinc-400">
                          {item.durationMs != null ? `${item.durationMs}ms` : "-"}
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
        </div>

        {/* Right / Bottom: Detail Inspector Pane */}
        <div className="w-[450px] lg:w-[550px] xl:w-[650px] flex flex-col bg-zinc-900/40 border-l border-zinc-800">
          {selectedLogDetail ? (
            <div className="flex flex-col h-full">
              {/* Detail Header Tabs */}
              <div className="flex items-center justify-between px-4 py-2 bg-zinc-900 border-b border-zinc-800">
                <div className="flex items-center gap-1">
                  <button
                    onClick={() => setActiveDetailTab("request")}
                    className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
                      activeDetailTab === "request"
                        ? "bg-indigo-600/20 text-indigo-300 border border-indigo-500/30"
                        : "text-zinc-400 hover:text-zinc-200"
                    }`}
                  >
                    Request
                  </button>
                  <button
                    onClick={() => setActiveDetailTab("response")}
                    className={`px-3 py-1.5 rounded-md text-xs font-medium transition-colors ${
                      activeDetailTab === "response"
                        ? "bg-indigo-600/20 text-indigo-300 border border-indigo-500/30"
                        : "text-zinc-400 hover:text-zinc-200"
                    }`}
                  >
                    Response
                  </button>
                </div>
                <div className="text-2xs font-mono text-zinc-400">
                  ID: {selectedLogDetail.id.slice(0, 8)}...
                </div>
              </div>

              {/* Detail Content Viewer */}
              <div className="flex-1 overflow-y-auto p-4 space-y-4 font-mono text-xs">
                {activeDetailTab === "request" ? (
                  <>
                    <div className="bg-zinc-950 p-3 rounded-lg border border-zinc-800">
                      <div className="text-indigo-400 font-semibold mb-1">
                        {selectedLogDetail.method} {selectedLogDetail.url}
                      </div>
                      <div className="text-zinc-400 text-2xs">Host: {selectedLogDetail.host}</div>
                    </div>

                    {/* Request Headers */}
                    <div className="bg-zinc-950 p-3 rounded-lg border border-zinc-800">
                      <div className="text-zinc-400 font-semibold mb-2 uppercase text-3xs tracking-wider">
                        Headers ({selectedLogDetail.requestHeaders?.length || 0})
                      </div>
                      <div className="space-y-1 divide-y divide-zinc-900/50">
                        {(selectedLogDetail.requestHeaders || []).map(([key, val], idx) => (
                          <div key={idx} className="pt-1 flex gap-2">
                            <span className="text-indigo-300 font-medium">{key}:</span>
                            <span className="text-zinc-300 break-all">{val}</span>
                          </div>
                        ))}
                      </div>
                    </div>

                    {/* Request Body */}
                    <div className="bg-zinc-950 p-3 rounded-lg border border-zinc-800">
                      <div className="text-zinc-400 font-semibold mb-2 uppercase text-3xs tracking-wider">
                        Body
                      </div>
                      <pre className="text-zinc-200 bg-zinc-900 p-2.5 rounded overflow-x-auto text-2xs whitespace-pre-wrap break-all">
                        {renderBodyContent(selectedLogDetail.requestBody)}
                      </pre>
                    </div>
                  </>
                ) : (
                  <>
                    <div className="bg-zinc-950 p-3 rounded-lg border border-zinc-800 flex items-center justify-between">
                      <div>
                        <span className="text-zinc-400 font-semibold">Status Code: </span>
                        <span className={`font-bold ${getStatusBadgeClass(selectedLogDetail.statusCode)} px-2 py-0.5 rounded border`}>
                          {selectedLogDetail.statusCode}
                        </span>
                      </div>
                      {selectedLogDetail.durationMs && (
                        <span className="text-zinc-400">Duration: {selectedLogDetail.durationMs}ms</span>
                      )}
                    </div>

                    {/* Response Headers */}
                    <div className="bg-zinc-950 p-3 rounded-lg border border-zinc-800">
                      <div className="text-zinc-400 font-semibold mb-2 uppercase text-3xs tracking-wider">
                        Headers ({selectedLogDetail.responseHeaders?.length || 0})
                      </div>
                      <div className="space-y-1 divide-y divide-zinc-900/50">
                        {(selectedLogDetail.responseHeaders || []).map(([key, val], idx) => (
                          <div key={idx} className="pt-1 flex gap-2">
                            <span className="text-indigo-300 font-medium">{key}:</span>
                            <span className="text-zinc-300 break-all">{val}</span>
                          </div>
                        ))}
                      </div>
                    </div>

                    {/* Response Body */}
                    <div className="bg-zinc-950 p-3 rounded-lg border border-zinc-800">
                      <div className="text-zinc-400 font-semibold mb-2 uppercase text-3xs tracking-wider">
                        Body
                      </div>
                      <pre className="text-zinc-200 bg-zinc-900 p-2.5 rounded overflow-x-auto text-2xs whitespace-pre-wrap break-all">
                        {renderBodyContent(selectedLogDetail.responseBody, selectedLogDetail.contentType)}
                      </pre>
                    </div>
                  </>
                )}
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-zinc-500 p-6 text-center">
              <MingCuteIcon name="file_code_line" size={36} className="mb-2 text-zinc-700" />
              <p className="text-sm font-medium">Select a request from the table to view details</p>
            </div>
          )}
        </div>
      </div>

      {/* History Log Rotation Limiter Settings Modal */}
      {settingsModalOpen && (
        <div
          onClick={(e) => {
            if (e.target === e.currentTarget) setSettingsModalOpen(false);
          }}
          className="fixed inset-0 z-50 bg-black/70 backdrop-blur-sm flex items-center justify-center p-4 cursor-pointer"
        >
          <form
            onSubmit={handleSaveSettings}
            onClick={(e) => e.stopPropagation()}
            className="bg-zinc-900 border border-zinc-800 rounded-xl max-w-md w-full p-5 shadow-2xl flex flex-col space-y-4 cursor-default select-text"
          >
            <div className="flex items-center justify-between pb-3 border-b border-zinc-800">
              <div className="flex items-center gap-2">
                <MingCuteIcon name="storage_line" size={18} className="text-indigo-400" />
                <h2 className="text-sm font-bold text-zinc-100">History Log Rotation Settings</h2>
              </div>
              <button
                type="button"
                onClick={() => setSettingsModalOpen(false)}
                className="text-zinc-400 hover:text-zinc-200"
              >
                <MingCuteIcon name="close_line" size={18} />
              </button>
            </div>

            {/* Toggle Enable Limiter */}
            <div className="flex items-center justify-between bg-zinc-950 p-3 rounded-lg border border-zinc-800">
              <div>
                <div className="text-xs font-semibold text-zinc-200">Automatic Rotation Limiter</div>
                <div className="text-2xs text-zinc-400 mt-0.5">
                  Enforces maximum row count in SQLite database.
                </div>
              </div>
              <button
                type="button"
                onClick={() => setTempLimiterEnabled(!tempLimiterEnabled)}
                className={`relative inline-flex h-5 w-9 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                  tempLimiterEnabled ? "bg-indigo-500" : "bg-zinc-700"
                }`}
              >
                <span
                  className={`pointer-events-none inline-block h-4 w-4 transform rounded-full bg-white shadow-lg ring-0 transition duration-200 ease-in-out ${
                    tempLimiterEnabled ? "translate-x-4" : "translate-x-0"
                  }`}
                />
              </button>
            </div>

            {/* Max Records Input */}
            <div className="space-y-1.5">
              <label className="text-xs font-medium text-zinc-300 flex items-center justify-between">
                <span>Maximum Stored Records</span>
                <span className="text-2xs text-indigo-400 font-mono">{tempMaxRows} rows</span>
              </label>
              <div className="flex items-center gap-2">
                <input
                  type="number"
                  min={50}
                  max={50000}
                  step={50}
                  disabled={!tempLimiterEnabled}
                  value={tempMaxRows}
                  onChange={(e) => setTempMaxRows(Math.max(1, parseInt(e.target.value) || 500))}
                  className="flex-1 bg-zinc-950 border border-zinc-800 focus:border-indigo-500 rounded-lg px-3 py-1.5 text-xs text-zinc-200 outline-none font-mono disabled:opacity-40"
                />
                <select
                  disabled={!tempLimiterEnabled}
                  value={[100, 250, 500, 1000, 2500, 5000].includes(tempMaxRows) ? tempMaxRows : "custom"}
                  onChange={(e) => {
                    if (e.target.value !== "custom") {
                      setTempMaxRows(parseInt(e.target.value));
                    }
                  }}
                  className="bg-zinc-950 border border-zinc-800 rounded-lg px-2.5 py-1.5 text-xs text-zinc-300 outline-none cursor-pointer disabled:opacity-40"
                >
                  <option value={100}>100 rows</option>
                  <option value={250}>250 rows</option>
                  <option value={500}>500 rows (Default)</option>
                  <option value={1000}>1,000 rows</option>
                  <option value={2500}>2,500 rows</option>
                  <option value={5000}>5,000 rows</option>
                  <option value="custom">Custom</option>
                </select>
              </div>
            </div>

            <div className="pt-2 flex justify-end gap-2 border-t border-zinc-800">
              <button
                type="button"
                onClick={() => setSettingsModalOpen(false)}
                className="px-3 py-1.5 bg-zinc-800 hover:bg-zinc-700 text-zinc-300 text-xs rounded-lg transition-colors"
              >
                Cancel
              </button>
              <button
                type="submit"
                className="px-4 py-1.5 bg-indigo-600 hover:bg-indigo-500 text-white font-semibold text-xs rounded-lg transition-colors shadow-sm"
              >
                Save Settings
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
};
