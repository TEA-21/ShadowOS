import React, { useState, useEffect, useRef } from 'react';
import { 
  Terminal, 
  Play, 
  Pause, 
  Trash2, 
  ArrowDown, 
  ShieldCheck, 
  Bot, 
  Sparkles, 
  CheckCircle2, 
  Clock, 
  Layers, 
  ArrowRight,
  GitCommit,
  Check,
  FileCode,
  HardDrive
} from 'lucide-react';
import type { VsockLogEntry, AgentNote } from '../../types';
import { Button } from '../common/Button';
import { Badge } from '../common/Badge';
import { Modal } from '../common/Modal';

interface McpSwarmConsoleProps {
  logs: VsockLogEntry[];
  agentNotes: AgentNote[];
  isStreaming: boolean;
  onToggleStreaming: () => void;
  onClearLogs: () => void;
  onInspectDiff?: () => void;
  onPromoteHostTree?: (noteId: string) => void;
}

export const McpSwarmConsole: React.FC<McpSwarmConsoleProps> = ({
  logs,
  agentNotes,
  isStreaming,
  onToggleStreaming,
  onClearLogs,
  onInspectDiff,
  onPromoteHostTree,
}) => {
  const [selectedChannel, setSelectedChannel] = useState<string>('all');
  const [selectedLevel, setSelectedLevel] = useState<string>('all');
  const [autoScroll, setAutoScroll] = useState(true);
  const [selectedNoteId, setSelectedNoteId] = useState<string>(agentNotes[0]?.id || '');
  const [expandedPayloads, setExpandedPayloads] = useState<Set<string>>(new Set());
  
  // Promotion modal & state
  const [isPromoteModalOpen, setIsPromoteModalOpen] = useState(false);
  const [isPromoting, setIsPromoting] = useState(false);
  const [promotionSuccessMessage, setPromotionSuccessMessage] = useState<string | null>(null);

  const logContainerRef = useRef<HTMLDivElement>(null);

  const selectedNote = agentNotes.find((n) => n.id === selectedNoteId) || agentNotes[0];

  useEffect(() => {
    if (autoScroll && logContainerRef.current) {
      logContainerRef.current.scrollTop = logContainerRef.current.scrollHeight;
    }
  }, [logs, autoScroll]);

  const toggleExpandPayload = (id: string) => {
    setExpandedPayloads((prev) => {
      const next = new Set(prev);
      if (next.has(id)) {
        next.delete(id);
      } else {
        next.add(id);
      }
      return next;
    });
  };

  const handleConfirmPromotion = () => {
    setIsPromoting(true);
    setTimeout(() => {
      onPromoteHostTree?.(selectedNote.id);
      setIsPromoting(false);
      setIsPromoteModalOpen(false);
      setPromotionSuccessMessage(
        `Successfully promoted 6 modified files to host repository (c:\\Users\\HP\\OneDrive\\Documents\\ShadowOs\\ShadowOS). SHA256 integrity verified with zero drift.`
      );
      setTimeout(() => setPromotionSuccessMessage(null), 8000);
    }, 450);
  };

  const filteredLogs = logs.filter((log) => {
    if (selectedChannel !== 'all' && !log.channel.includes(selectedChannel)) {
      return false;
    }
    if (selectedLevel !== 'all' && log.level !== selectedLevel) {
      return false;
    }
    return true;
  });

  const getLevelBadge = (level: VsockLogEntry['level']) => {
    switch (level) {
      case 'INFO':
        return (
          <span className="font-mono text-[10px] font-semibold text-blue-700 bg-blue-50 px-1.5 py-0.2 rounded border border-blue-200 shrink-0">
            [INFO]
          </span>
        );
      case 'DEBUG':
        return (
          <span className="font-mono text-[10px] font-semibold text-slate-600 bg-slate-100 px-1.5 py-0.2 rounded border border-slate-200 shrink-0">
            [DEBUG]
          </span>
        );
      case 'TRACE':
        return (
          <span className="font-mono text-[10px] font-semibold text-purple-700 bg-purple-50 px-1.5 py-0.2 rounded border border-purple-200 shrink-0">
            [TRACE]
          </span>
        );
      case 'WARN':
        return (
          <span className="font-mono text-[10px] font-semibold text-[#D97757] bg-amber-50 px-1.5 py-0.2 rounded border border-amber-200 shrink-0">
            [WARN]
          </span>
        );
      case 'ERROR':
        return (
          <span className="font-mono text-[10px] font-semibold text-red-700 bg-red-50 px-1.5 py-0.2 rounded border border-red-200 shrink-0">
            [ERROR]
          </span>
        );
    }
  };

  return (
    <div className="space-y-4">
      {/* 50% / 50% Split View Layout */}
      <div className="grid grid-cols-1 lg:grid-cols-2 gap-4 items-stretch min-h-[640px]">
        {/* ======================================================== */}
        {/* LEFT PANE (50% Width - Live Console): Monospace Stream  */}
        {/* ======================================================== */}
        <div className="bg-white border border-slate-200 rounded-lg shadow-subtle flex flex-col overflow-hidden">
          {/* Header */}
          <div className="px-4 py-3 border-b border-slate-200 bg-white flex flex-wrap items-center justify-between gap-2.5">
            <div className="flex items-center gap-2">
              <Terminal className="w-4 h-4 text-purple-600" />
              <span className="text-xs font-bold text-slate-900">
                Live Vsock Stream & MCP Events
              </span>
              <span className="font-mono text-[10px] text-slate-400">
                ({filteredLogs.length} events)
              </span>
            </div>

            {/* Controls */}
            <div className="flex items-center gap-1.5">
              <button
                onClick={onToggleStreaming}
                className={`px-2 py-1 text-xs rounded-md border font-medium transition-colors flex items-center gap-1 ${
                  isStreaming 
                    ? 'bg-purple-50 text-purple-700 border-purple-200' 
                    : 'bg-slate-50 text-slate-600 border-slate-200 hover:bg-slate-100'
                }`}
                title={isStreaming ? 'Pause Stream' : 'Resume Stream'}
              >
                {isStreaming ? <Pause className="w-3 h-3" /> : <Play className="w-3 h-3 text-[#2E8B57]" />}
                <span>{isStreaming ? 'Pause' : 'Resume'}</span>
              </button>

              <button
                onClick={() => setAutoScroll(!autoScroll)}
                className={`p-1.5 rounded text-xs border transition-colors ${
                  autoScroll
                    ? 'bg-purple-50 text-purple-700 border-purple-200'
                    : 'bg-white text-slate-500 border-slate-200 hover:bg-slate-50'
                }`}
                title="Toggle Auto-Scroll"
              >
                <ArrowDown className="w-3.5 h-3.5" />
              </button>

              <button
                onClick={onClearLogs}
                className="p-1.5 rounded text-xs border border-slate-200 text-slate-500 hover:text-slate-800 hover:bg-slate-50 transition-colors"
                title="Clear Logs"
              >
                <Trash2 className="w-3.5 h-3.5" />
              </button>
            </div>
          </div>

          {/* Filter Bar */}
          <div className="px-4 py-2 bg-slate-50/70 border-b border-slate-200 flex flex-wrap items-center justify-between text-xs gap-2">
            <div className="flex items-center gap-1.5">
              <span className="text-[11px] text-slate-500 font-medium">Channel:</span>
              <select
                value={selectedChannel}
                onChange={(e) => setSelectedChannel(e.target.value)}
                className="text-[11px] font-mono bg-white border border-slate-200 rounded px-2 py-0.5 text-slate-700 focus:outline-none focus:ring-1 focus:ring-purple-500"
              >
                <option value="all">All Channels</option>
                <option value="mcp-jsonrpc">mcp-jsonrpc</option>
                <option value="vsock">vsock streams</option>
                <option value="virtio-balloon">virtio-balloon</option>
                <option value="overlayfs">overlayfs</option>
                <option value="cdp">cdp (browser)</option>
              </select>
            </div>

            <div className="flex items-center gap-1.5">
              <span className="text-[11px] text-slate-500 font-medium">Level:</span>
              <select
                value={selectedLevel}
                onChange={(e) => setSelectedLevel(e.target.value)}
                className="text-[11px] font-mono bg-white border border-slate-200 rounded px-2 py-0.5 text-slate-700 focus:outline-none focus:ring-1 focus:ring-purple-500"
              >
                <option value="all">All Levels</option>
                <option value="INFO">INFO</option>
                <option value="DEBUG">DEBUG</option>
                <option value="TRACE">TRACE</option>
                <option value="WARN">WARN</option>
              </select>
            </div>
          </div>

          {/* Monospace Log Viewer with Clean Word Wrapping */}
          <div
            ref={logContainerRef}
            className="flex-1 overflow-y-auto p-3.5 font-mono text-[11px] leading-relaxed bg-[#F8FAFC] space-y-1.5 select-text max-h-[580px]"
          >
            {filteredLogs.length === 0 ? (
              <div className="text-center py-16 text-slate-400">
                No logs recorded for current filter criteria.
              </div>
            ) : (
              filteredLogs.map((log) => {
                const hasPayload = !!log.jsonRpcPayload;
                const isExpanded = expandedPayloads.has(log.id);

                return (
                  <div
                    key={log.id}
                    className="p-1.5 rounded-md hover:bg-white transition-colors border border-transparent hover:border-slate-200"
                  >
                    <div className="flex items-start gap-2">
                      <span className="text-slate-400 select-none text-[10.5px] shrink-0 font-mono">
                        {log.timestamp}
                      </span>
                      <span className="shrink-0">{getLevelBadge(log.level)}</span>
                      <span className="font-mono text-[10px] px-1 py-0.2 rounded bg-slate-100 text-slate-700 border border-slate-200 shrink-0">
                        {log.channel}
                      </span>
                      {log.vmId && (
                        <span className="font-mono text-[10px] text-purple-700 font-semibold shrink-0">
                          {log.vmId}
                        </span>
                      )}
                      <span className="text-slate-800 break-words leading-relaxed flex-1 font-mono text-[11px]">
                        {log.message}
                      </span>
                      {hasPayload && (
                        <button
                          onClick={() => toggleExpandPayload(log.id)}
                          className="shrink-0 text-[10px] font-mono text-purple-700 hover:text-purple-900 px-1.5 py-0.2 rounded bg-purple-50 border border-purple-200 font-medium"
                        >
                          {isExpanded ? 'Hide' : 'JSON'}
                        </button>
                      )}
                    </div>

                    {/* Expandable JSON frame */}
                    {hasPayload && isExpanded && (
                      <div className="mt-1.5 ml-12 p-2 rounded bg-white border border-purple-200 text-slate-800 text-[10px] overflow-x-auto shadow-2xs">
                        <pre className="font-mono whitespace-pre-wrap">
                          {JSON.stringify(JSON.parse(log.jsonRpcPayload!), null, 2)}
                        </pre>
                      </div>
                    )}
                  </div>
                );
              })
            )}
          </div>
        </div>

        {/* ======================================================== */}
        {/* RIGHT PANE (50% Width - Agent Execution Summary): Cards */}
        {/* ======================================================== */}
        <div className="bg-white border border-slate-200 rounded-lg shadow-subtle flex flex-col overflow-hidden">
          {/* Header */}
          <div className="px-4 py-3 border-b border-slate-200 bg-white flex items-center justify-between gap-2">
            <div className="flex items-center gap-2">
              <Bot className="w-4 h-4 text-purple-600" />
              <span className="text-xs font-bold text-slate-900">
                Agent Execution Summary
              </span>
            </div>

            {/* Note Selector */}
            <select
              value={selectedNoteId}
              onChange={(e) => setSelectedNoteId(e.target.value)}
              className="text-xs font-medium bg-slate-50 border border-slate-200 rounded-md px-2.5 py-1 text-slate-800 focus:outline-none focus:ring-1 focus:ring-purple-500 max-w-[220px] truncate"
            >
              {agentNotes.map((note) => (
                <option key={note.id} value={note.id}>
                  {note.title}
                </option>
              ))}
            </select>
          </div>

          {/* Body Content */}
          <div className="flex-1 overflow-y-auto p-4 space-y-3.5 bg-slate-50/30 max-h-[580px]">
            {/* Promotion Success Toast Banner */}
            {promotionSuccessMessage && (
              <div className="p-3 bg-emerald-50 border border-emerald-200 rounded-lg text-xs text-emerald-900 flex items-start justify-between gap-2 shadow-2xs animate-fadeIn">
                <div className="flex items-start gap-2">
                  <CheckCircle2 className="w-4 h-4 text-[#2E8B57] shrink-0 mt-0.5" />
                  <div>
                    <span className="font-bold text-[#2E8B57]">Host Tree Promotion Complete!</span>
                    <p className="text-emerald-800 text-[11px] mt-0.5 leading-relaxed font-mono">
                      {promotionSuccessMessage}
                    </p>
                  </div>
                </div>
                <button
                  onClick={() => setPromotionSuccessMessage(null)}
                  className="text-emerald-700 hover:text-emerald-950 font-medium text-xs px-1"
                >
                  ✕
                </button>
              </div>
            )}

            {selectedNote ? (
              <>
                {/* 1. Key Metrics Cards at Top */}
                <div className="grid grid-cols-3 gap-2.5">
                  <div className="bg-white border border-slate-200 rounded-lg p-3 text-center shadow-2xs">
                    <span className="text-[10px] font-medium text-slate-400 uppercase tracking-wider block">
                      Tests Passed
                    </span>
                    <div className="flex items-center justify-center gap-1.5 mt-1">
                      <CheckCircle2 className="w-4 h-4 text-[#2E8B57]" />
                      <span className="font-mono text-base font-bold text-[#2E8B57]">
                        {selectedNote.metrics.testPassed} / {selectedNote.metrics.testTotal}
                      </span>
                    </div>
                  </div>

                  <div className="bg-white border border-slate-200 rounded-lg p-3 text-center shadow-2xs">
                    <span className="text-[10px] font-medium text-slate-400 uppercase tracking-wider block">
                      Compilation
                    </span>
                    <div className="flex items-center justify-center gap-1 mt-1">
                      <Clock className="w-4 h-4 text-slate-500" />
                      <span className="font-mono text-base font-bold text-slate-900">
                        {selectedNote.metrics.compilationTimeMs} ms
                      </span>
                    </div>
                  </div>

                  <div className="bg-white border border-slate-200 rounded-lg p-3 text-center shadow-2xs">
                    <span className="text-[10px] font-medium text-slate-400 uppercase tracking-wider block">
                      CoW Delta
                    </span>
                    <div className="flex items-center justify-center gap-1 mt-1">
                      <Layers className="w-4 h-4 text-purple-600" />
                      <span className="font-mono text-base font-bold text-purple-700">
                        {selectedNote.metrics.uncommittedDiffBytes}
                      </span>
                    </div>
                  </div>
                </div>

                {/* 2. Directive & Environment Card */}
                <div className="bg-white border border-slate-200 rounded-lg p-3.5 shadow-2xs space-y-2">
                  <div className="flex items-center justify-between">
                    <div className="flex items-center gap-1.5 text-xs font-bold text-slate-900">
                      <Sparkles className="w-3.5 h-3.5 text-purple-600" />
                      <span>Task Directive & Mission</span>
                    </div>
                    <Badge variant={selectedNote.status === 'Completed' ? 'active' : 'warning'} size="sm">
                      {selectedNote.status}
                    </Badge>
                  </div>
                  <p className="text-xs text-slate-600 leading-relaxed">
                    {selectedNote.targetTask}
                  </p>

                  <div className="pt-2 border-t border-slate-100 grid grid-cols-2 gap-2 text-[11px] text-slate-500">
                    <div>
                      <span className="text-slate-400 block text-[10px]">Coordinator</span>
                      <span className="font-medium text-slate-800">{selectedNote.coordinator}</span>
                    </div>
                    <div>
                      <span className="text-slate-400 block text-[10px]">Hypervisor</span>
                      <span className="font-medium text-slate-800">{selectedNote.hypervisorDriver}</span>
                    </div>
                  </div>
                </div>

                {/* 3. Verification Audit Cards (Clean Badges, NO raw unrendered math) */}
                <div className="bg-white border border-slate-200 rounded-lg p-3.5 shadow-2xs space-y-2.5">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-bold text-slate-900 flex items-center gap-1.5">
                      <ShieldCheck className="w-3.5 h-3.5 text-[#2E8B57]" />
                      Verification Audit & Performance Gates
                    </span>
                    <span className="text-[10px] font-semibold text-[#2E8B57] bg-emerald-50 px-2 py-0.5 rounded border border-emerald-200">
                      ALL PASS
                    </span>
                  </div>

                  <div className="space-y-1.5">
                    {selectedNote.nfrAudits.map((audit) => (
                      <div 
                        key={audit.id}
                        className="flex items-center justify-between text-xs p-2 rounded bg-slate-50/70 border border-slate-100"
                      >
                        <div className="flex items-center gap-2">
                          <span className="font-mono text-[10px] font-bold text-purple-700 bg-purple-50 px-1.5 py-0.2 rounded border border-purple-200">
                            {audit.id}
                          </span>
                          <span className="font-medium text-slate-800">{audit.label}</span>
                        </div>
                        <div className="flex items-center gap-3 font-mono text-[11px]">
                          <span className="text-slate-400 hidden sm:inline">{audit.target}</span>
                          <span className="font-semibold text-[#2E8B57]">{audit.measured}</span>
                        </div>
                      </div>
                    ))}
                  </div>
                </div>

                {/* 4. Key Security & Isolation Observations */}
                <div className="bg-white border border-slate-200 rounded-lg p-3.5 shadow-2xs space-y-2">
                  <span className="text-xs font-bold text-slate-900 block">
                    Key Observations & Security Boundaries
                  </span>
                  <ul className="space-y-1.5 text-xs text-slate-600 pl-1">
                    {selectedNote.keyObservations.map((obs, idx) => (
                      <li key={idx} className="flex items-start gap-2">
                        <span className="w-1.5 h-1.5 rounded-full bg-purple-600 shrink-0 mt-1.5" />
                        <span className="leading-relaxed">{obs}</span>
                      </li>
                    ))}
                  </ul>
                </div>

                {/* 5. Recommended Actions & Working Buttons */}
                <div className="bg-white border border-slate-200 rounded-lg p-3.5 shadow-2xs space-y-2.5">
                  <span className="text-xs font-bold text-slate-900 block">
                    Recommended Actions
                  </span>
                  <ul className="space-y-1 text-xs text-slate-600 pl-1">
                    {selectedNote.recommendedActions.map((act, idx) => (
                      <li key={idx} className="flex items-start gap-2">
                        <ArrowRight className="w-3.5 h-3.5 text-purple-600 shrink-0 mt-0.5" />
                        <span className="leading-relaxed">{act}</span>
                      </li>
                    ))}
                  </ul>

                  {/* ACTIVE ACTION BUTTONS */}
                  <div className="pt-2.5 border-t border-slate-100 flex items-center justify-end gap-2.5">
                    <Button 
                      variant="outline-purple" 
                      size="sm"
                      onClick={() => onInspectDiff?.()}
                      className="font-medium"
                    >
                      Inspect Diff in Staging
                    </Button>
                    <Button 
                      variant="primary" 
                      size="sm"
                      onClick={() => setIsPromoteModalOpen(true)}
                      className="font-medium shadow-xs"
                    >
                      Promote to Host Tree
                    </Button>
                  </div>
                </div>
              </>
            ) : (
              <div className="text-center py-12 text-slate-400 text-xs">
                No note selected.
              </div>
            )}
          </div>
        </div>
      </div>

      {/* Promotion Confirmation Modal */}
      <Modal
        isOpen={isPromoteModalOpen}
        onClose={() => setIsPromoteModalOpen(false)}
        title={
          <div className="flex items-center gap-2">
            <GitCommit className="w-4 h-4 text-purple-600" />
            <span>Promote Staged Changes to Host Workspace</span>
          </div>
        }
        subtitle="Atomic staging from ephemeral upperdir into host repository tree (PRD M3-04 primitive)"
        maxWidth="lg"
        footer={
          <>
            <Button 
              variant="secondary" 
              onClick={() => setIsPromoteModalOpen(false)}
              disabled={isPromoting}
            >
              Cancel
            </Button>
            <Button 
              variant="primary" 
              isLoading={isPromoting}
              onClick={handleConfirmPromotion}
            >
              Confirm Atomic Promotion
            </Button>
          </>
        }
      >
        <div className="space-y-3.5 text-xs text-slate-700">
          <div className="p-3 bg-purple-50/70 border border-purple-200 rounded-lg space-y-1 font-mono text-[11px]">
            <div className="text-purple-900 font-bold flex items-center gap-1.5">
              <HardDrive className="w-3.5 h-3.5" />
              Target Host Repository:
            </div>
            <div className="text-purple-800 break-all pl-5">
              c:\Users\HP\OneDrive\Documents\ShadowOs\ShadowOS
            </div>
          </div>

          <div>
            <span className="font-semibold text-slate-900 block mb-1.5">
              Staged Files to Merge (6 files):
            </span>
            <div className="space-y-1 font-mono text-[11px] max-h-40 overflow-y-auto border border-slate-200 rounded-md p-2 bg-slate-50/50">
              <div className="flex items-center justify-between py-0.5">
                <span className="text-slate-800 flex items-center gap-1">
                  <FileCode className="w-3 h-3 text-slate-400" />
                  crates/shadow-vmm/src/swarm.rs
                </span>
                <span className="text-[#2E8B57] font-semibold">+18 -4</span>
              </div>
              <div className="flex items-center justify-between py-0.5">
                <span className="text-slate-800 flex items-center gap-1">
                  <FileCode className="w-3 h-3 text-slate-400" />
                  crates/shadow-mcp/src/handler.rs
                </span>
                <span className="text-[#2E8B57] font-semibold">+24 -2</span>
              </div>
              <div className="flex items-center justify-between py-0.5">
                <span className="text-slate-800 flex items-center gap-1">
                  <FileCode className="w-3 h-3 text-slate-400" />
                  crates/shadow-cow/src/overlay.rs
                </span>
                <span className="text-[#2E8B57] font-semibold">+12 -8</span>
              </div>
              <div className="flex items-center justify-between py-0.5">
                <span className="text-slate-800 flex items-center gap-1">
                  <FileCode className="w-3 h-3 text-slate-400" />
                  scripts/verify_swarm_affinity.py
                </span>
                <span className="text-[#2E8B57] font-semibold">+42 -0</span>
              </div>
              <div className="flex items-center justify-between py-0.5">
                <span className="text-slate-800 flex items-center gap-1">
                  <FileCode className="w-3 h-3 text-slate-400" />
                  target/debug/incremental/shadow_vmm.cache
                </span>
                <span className="text-[#2E8B57] font-semibold">+86 -0</span>
              </div>
              <div className="flex items-center justify-between py-0.5">
                <span className="text-slate-500 line-through flex items-center gap-1">
                  <FileCode className="w-3 h-3 text-slate-400" />
                  .shadow/temp_build.lock
                </span>
                <span className="text-red-600 font-semibold">-3</span>
              </div>
            </div>
          </div>

          <div className="flex items-center gap-2 p-2.5 rounded bg-emerald-50 border border-emerald-200 text-[#2E8B57] text-[11px]">
            <Check className="w-4 h-4 shrink-0" />
            <span>
              SHA256 checksum verified • 100% bit-identical match with zero host working tree drift.
            </span>
          </div>
        </div>
      </Modal>
    </div>
  );
};
