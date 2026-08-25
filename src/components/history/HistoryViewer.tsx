import React, { useEffect, useState } from "react";
import { useHistoryStore } from "../../stores/useHistoryStore";
import {
  Play,
  Pause,
  Trash2,
  Search,
  RefreshCw,
  FileText,
  Activity,
} from "lucide-react";

export const HistoryViewer: React.FC = () => {
  const {
    logs,
    searchTerm,
    methodFilter,
    selectedLogId,
    selectedLogDetail,
    proxyConfig,
    isLoading,
    fetchLogs,
    setSearchTerm,
    setMethodFilter,
    selectLog,
    clearLogs,
    fetchProxyStatus,
    toggleProxyServer,
    initSubscription,
  } = useHistoryStore();


  const [activeDetailTab, setActiveDetailTab] = useState<"request" | "response">("request");

  useEffect(() => {
    fetchProxyStatus();
    fetchLogs();
    initSubscription();
  }, []);

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

  return (
    <div className="flex flex-col h-screen w-full bg-zinc-950 text-zinc-100 font-sans antialiased overflow-hidden">
      {/* Top Toolbar / Control Header */}
      <header className="flex items-center justify-between px-4 py-3 bg-zinc-900/90 border-b border-zinc-800 backdrop-blur-md">
        <div className="flex items-center gap-3">
          <div className="flex items-center gap-2">
            <Activity className="w-5 h-5 text-indigo-400 animate-pulse" />
            <h1 className="text-lg font-bold tracking-tight bg-gradient-to-r from-indigo-400 to-cyan-400 bg-clip-text text-transparent">
              Traffic Inspector
            </h1>
          </div>

          <div className="h-4 w-px bg-zinc-800 mx-1" />

          {/* Proxy Control Button */}
          <button
            onClick={toggleProxyServer}
            className={`flex items-center gap-2 px-3 py-1.5 rounded-lg text-xs font-semibold tracking-wide transition-all shadow-sm ${
              proxyConfig.isRunning
                ? "bg-rose-500/20 text-rose-300 border border-rose-500/30 hover:bg-rose-500/30 shadow-rose-950/20"
                : "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30 hover:bg-emerald-500/30 shadow-emerald-950/20"
            }`}
          >
            {proxyConfig.isRunning ? (
              <>
                <Pause className="w-3.5 h-3.5 fill-current" /> Stop Proxy ({proxyConfig.port})
              </>
            ) : (
              <>
                <Play className="w-3.5 h-3.5 fill-current" /> Start Proxy ({proxyConfig.port})
              </>
            )}
          </button>
        </div>

        {/* Filter Controls & Search */}
        <div className="flex items-center gap-3">
          <div className="relative flex items-center">
            <Search className="w-4 h-4 absolute left-3 text-zinc-400 pointer-events-none" />
            <input
              type="text"
              placeholder="Search URL, Host, Method..."
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
            <RefreshCw className={`w-4 h-4 ${isLoading ? "animate-spin" : ""}`} />
          </button>

          <button
            onClick={clearLogs}
            className="flex items-center gap-1.5 px-3 py-1.5 bg-rose-500/10 border border-rose-500/20 hover:bg-rose-500/20 text-rose-400 text-xs rounded-lg transition-colors font-medium"
            title="Clear History"
          >
            <Trash2 className="w-3.5 h-3.5" /> Clear
          </button>
        </div>
      </header>

      {/* Main Split-Pane Body */}
      <div className="flex flex-1 overflow-hidden">
        {/* Left / Top: Traffic Table */}
        <div className="flex-1 flex flex-col border-r border-zinc-800 bg-zinc-950/50 min-w-0">
          <div className="overflow-y-auto flex-1 divide-y divide-zinc-800/50">
            <table className="w-full text-left text-xs border-collapse">
              <thead className="bg-zinc-900/60 sticky top-0 border-b border-zinc-800 text-zinc-400 font-medium uppercase tracking-wider backdrop-blur-sm z-10">
                <tr>
                  <th className="py-2.5 px-3 w-16">Status</th>
                  <th className="py-2.5 px-3 w-20">Method</th>
                  <th className="py-2.5 px-3 w-48 truncate">Host</th>
                  <th className="py-2.5 px-3 truncate">URL / Path</th>
                  <th className="py-2.5 px-3 w-20 text-right">Time (ms)</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-zinc-900/60">
                {logs.length === 0 ? (
                  <tr>
                    <td colSpan={5} className="text-center py-16 text-zinc-500 text-sm">
                      No traffic captured yet. Start proxy and send HTTP requests.
                    </td>
                  </tr>
                ) : (
                  logs.map((item) => {
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
                        <td className="py-2 px-3">
                          <span
                            className={`inline-block px-2 py-0.5 rounded border text-[10px] font-mono font-bold ${getStatusBadgeClass(
                              item.statusCode
                            )}`}
                          >
                            {item.statusCode}
                          </span>
                        </td>
                        <td className="py-2 px-3">
                          <span
                            className={`inline-block px-1.5 py-0.5 rounded border text-[10px] font-mono font-semibold ${getMethodBadgeClass(
                              item.method
                            )}`}
                          >
                            {item.method}
                          </span>
                        </td>
                        <td className="py-2 px-3 font-mono text-zinc-300 truncate max-w-[12rem]">
                          {item.host}
                        </td>
                        <td className="py-2 px-3 font-mono text-zinc-400 truncate max-w-[20rem]">
                          {item.url}
                        </td>
                        <td className="py-2 px-3 text-right font-mono text-zinc-400">
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
                <div className="text-[11px] font-mono text-zinc-400">
                  ID: #{selectedLogDetail.id}
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
                      <div className="text-zinc-500 text-[11px]">Host: {selectedLogDetail.host}</div>
                    </div>

                    {/* Request Headers */}
                    <div className="bg-zinc-950 p-3 rounded-lg border border-zinc-800">
                      <div className="text-zinc-400 font-semibold mb-2 uppercase text-[10px] tracking-wider">
                        Headers ({selectedLogDetail.requestHeaders.length})
                      </div>
                      <div className="space-y-1 divide-y divide-zinc-900/50">
                        {selectedLogDetail.requestHeaders.map(([key, val], idx) => (
                          <div key={idx} className="pt-1 flex gap-2">
                            <span className="text-indigo-300 font-medium">{key}:</span>
                            <span className="text-zinc-300 break-all">{val}</span>
                          </div>
                        ))}
                      </div>
                    </div>

                    {/* Request Body */}
                    <div className="bg-zinc-950 p-3 rounded-lg border border-zinc-800">
                      <div className="text-zinc-400 font-semibold mb-2 uppercase text-[10px] tracking-wider">
                        Body
                      </div>
                      <pre className="text-zinc-200 bg-zinc-900 p-2.5 rounded overflow-x-auto text-[11px]">
                        {selectedLogDetail.requestBody || "(empty)"}
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
                      <div className="text-zinc-400 font-semibold mb-2 uppercase text-[10px] tracking-wider">
                        Headers ({selectedLogDetail.responseHeaders.length})
                      </div>
                      <div className="space-y-1 divide-y divide-zinc-900/50">
                        {selectedLogDetail.responseHeaders.map(([key, val], idx) => (
                          <div key={idx} className="pt-1 flex gap-2">
                            <span className="text-indigo-300 font-medium">{key}:</span>
                            <span className="text-zinc-300 break-all">{val}</span>
                          </div>
                        ))}
                      </div>
                    </div>

                    {/* Response Body */}
                    <div className="bg-zinc-950 p-3 rounded-lg border border-zinc-800">
                      <div className="text-zinc-400 font-semibold mb-2 uppercase text-[10px] tracking-wider">
                        Body
                      </div>
                      <pre className="text-zinc-200 bg-zinc-900 p-2.5 rounded overflow-x-auto text-[11px]">
                        {selectedLogDetail.responseBody || "(empty)"}
                      </pre>
                    </div>
                  </>
                )}
              </div>
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center h-full text-zinc-500 p-6 text-center">
              <FileText className="w-10 h-10 mb-2 stroke-[1.5] text-zinc-600" />
              <p className="text-sm font-medium">Select a request from the table to view details</p>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
