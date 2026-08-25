import React, { useState, useRef, useEffect } from "react";
import { useHistoryStore } from "../../stores/useHistoryStore";
import { MingCuteIcon } from "./MingCuteIcon";

export const ProxyPowerButton: React.FC = () => {
  const { proxyConfig, changeProxyMode } = useHistoryStore();
  const [isOpen, setIsOpen] = useState(false);
  const containerRef = useRef<HTMLDivElement>(null);

  const mode = proxyConfig.proxyMode || (proxyConfig.proxyEnabled ? "on" : "off");

  // Close overlay on outside click
  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        setIsOpen(false);
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const getModeStyles = (m: string) => {
    switch (m) {
      case "on":
        return {
          btn: "bg-emerald-500/15 text-emerald-400 border-emerald-500/40 hover:bg-emerald-500/25 shadow-[0_0_12px_rgba(52,211,153,0.35)]",
          icon: "text-emerald-400",
        };
      case "block_client":
        return {
          btn: "bg-amber-500/15 text-amber-400 border-amber-500/40 hover:bg-amber-500/25 shadow-[0_0_12px_rgba(251,191,36,0.35)]",
          icon: "text-amber-400",
        };
      case "block":
        return {
          btn: "bg-purple-500/15 text-purple-400 border-purple-500/40 hover:bg-purple-500/25 shadow-[0_0_12px_rgba(192,132,252,0.35)]",
          icon: "text-purple-400",
        };
      case "off":
      default:
        return {
          btn: "bg-rose-500/15 text-rose-400 border-rose-500/40 hover:bg-rose-500/25 shadow-[0_0_12px_rgba(244,63,94,0.35)]",
          icon: "text-rose-400",
        };
    }
  };

  const currentStyles = getModeStyles(mode);

  const handleSelectMode = async (selectedMode: "on" | "off" | "block_client" | "block") => {
    await changeProxyMode(selectedMode);
    setIsOpen(false);
  };

  return (
    <div ref={containerRef} className="relative inline-block text-left">
      {/* Power Icon Button (NO text or dot as requested) */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={`p-2 rounded-lg border transition-all duration-200 cursor-pointer flex items-center justify-center ${currentStyles.btn}`}
        title={`Proxy Mode: ${mode.toUpperCase()} (Click to change)`}
      >
        <MingCuteIcon name="power_line" size={16} className={currentStyles.icon} />
      </button>

      {/* Mode Selection Overlay Popover */}
      {isOpen && (
        <div className="fixed top-11 right-3 w-72 bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl z-[9999] p-2 text-xs divide-y divide-zinc-800/60 backdrop-blur-md animate-in fade-in zoom-in-95 duration-100">
          <div className="px-3 py-2 text-[10px] font-semibold text-zinc-400 uppercase tracking-wider">
            Select Proxy Mode ({proxyConfig.port})
          </div>


          <div className="py-1 space-y-1">
            {/* Mode 1: ON (Green) */}
            <button
              onClick={() => handleSelectMode("on")}
              className={`w-full flex items-start gap-3 p-2 rounded-lg text-left transition-colors ${
                mode === "on" ? "bg-emerald-500/10 border border-emerald-500/30" : "hover:bg-zinc-800/60"
              }`}
            >
              <div className="p-1.5 rounded-md bg-emerald-500/20 text-emerald-400 mt-0.5">
                <MingCuteIcon name="zap_line" size={14} />
              </div>
              <div className="flex-1">
                <div className="flex items-center justify-between font-bold text-emerald-400">
                  <span>ON (Green)</span>
                  {mode === "on" && <MingCuteIcon name="check_line" size={14} className="text-emerald-400" />}
                </div>
                <p className="text-[11px] text-zinc-400 mt-0.5 leading-snug">
                  Normal proxying. Sends requests to server and returns responses to client.
                </p>
              </div>
            </button>

            {/* Mode 2: OFF (Red) */}
            <button
              onClick={() => handleSelectMode("off")}
              className={`w-full flex items-start gap-3 p-2 rounded-lg text-left transition-colors ${
                mode === "off" ? "bg-rose-500/10 border border-rose-500/30" : "hover:bg-zinc-800/60"
              }`}
            >
              <div className="p-1.5 rounded-md bg-rose-500/20 text-rose-400 mt-0.5">
                <MingCuteIcon name="power_line" size={14} />
              </div>
              <div className="flex-1">
                <div className="flex items-center justify-between font-bold text-rose-400">
                  <span>OFF (Red)</span>
                  {mode === "off" && <MingCuteIcon name="check_line" size={14} className="text-rose-400" />}
                </div>
                <p className="text-[11px] text-zinc-400 mt-0.5 leading-snug">
                  Proxy disabled. Stops proxy listener server.
                </p>
              </div>
            </button>

            {/* Mode 3: Block Client (Yellow) */}
            <button
              onClick={() => handleSelectMode("block_client")}
              className={`w-full flex items-start gap-3 p-2 rounded-lg text-left transition-colors ${
                mode === "block_client" ? "bg-amber-500/10 border border-amber-500/30" : "hover:bg-zinc-800/60"
              }`}
            >
              <div className="p-1.5 rounded-md bg-amber-500/20 text-amber-400 mt-0.5">
                <MingCuteIcon name="radio_line" size={14} />
              </div>
              <div className="flex-1">
                <div className="flex items-center justify-between font-bold text-amber-400">
                  <span>Block Client (Yellow)</span>
                  {mode === "block_client" && <MingCuteIcon name="check_line" size={14} className="text-amber-400" />}
                </div>
                <p className="text-[11px] text-zinc-400 mt-0.5 leading-snug">
                  Sends request to server & logs history, but client does NOT receive response.
                </p>
              </div>
            </button>

            {/* Mode 4: Block All (Purple) */}
            <button
              onClick={() => handleSelectMode("block")}
              className={`w-full flex items-start gap-3 p-2 rounded-lg text-left transition-colors ${
                mode === "block" ? "bg-purple-500/10 border border-purple-500/30" : "hover:bg-zinc-800/60"
              }`}
            >
              <div className="p-1.5 rounded-md bg-purple-500/20 text-purple-400 mt-0.5">
                <MingCuteIcon name="ban_line" size={14} />
              </div>
              <div className="flex-1">
                <div className="flex items-center justify-between font-bold text-purple-400">
                  <span>Block All (Purple)</span>
                  {mode === "block" && <MingCuteIcon name="check_line" size={14} className="text-purple-400" />}
                </div>
                <p className="text-[11px] text-zinc-400 mt-0.5 leading-snug">
                  Logs request history, but NEVER sends to server and NEVER returns to client.
                </p>
              </div>
            </button>
          </div>
        </div>
      )}
    </div>
  );
};
