import React, { useState } from 'react';
import { 
  FileCode, 
  RotateCcw, 
  ChevronLeft, 
  ChevronRight, 
  Sparkles, 
  Copy, 
  Check, 
  ChevronDown, 
  ChevronUp,
  FileText,
  CheckCircle2,
  FolderTree
} from 'lucide-react';
import type { Snapshot, FileDiffItem, DiffFileStatus } from '../../types';
import { Button } from '../common/Button';
import { Badge } from '../common/Badge';

interface FileSystemDiffViewerProps {
  snapshot: Snapshot;
  onRollback: (snapshotId: string) => void;
  isRollingBack: boolean;
  isActiveBaseline?: boolean;
  upperdirWiped?: boolean;
  onMutateUpperdir?: () => void;
}

export const FileSystemDiffViewer: React.FC<FileSystemDiffViewerProps> = ({
  snapshot,
  onRollback,
  isRollingBack,
  isActiveBaseline = false,
  upperdirWiped = false,
  onMutateUpperdir,
}) => {
  const [selectedFileIndex, setSelectedFileIndex] = useState<number>(0);
  const [showAiExplanation, setShowAiExplanation] = useState<boolean>(true);
  const [copied, setCopied] = useState<boolean>(false);
  const [viewTab, setViewTab] = useState<'diff' | 'live-tree'>('diff');

  const files = snapshot.files;
  const selectedFile: FileDiffItem | undefined = files[selectedFileIndex];

  const handleNextFile = () => {
    if (files.length === 0) return;
    setSelectedFileIndex((prev) => (prev + 1) % files.length);
  };

  const handlePrevFile = () => {
    if (files.length === 0) return;
    setSelectedFileIndex((prev) => (prev - 1 + files.length) % files.length);
  };

  const handleCopy = () => {
    if (!selectedFile) return;
    const diffText = selectedFile.diffLines.map((l) => l.text).join('\n');
    navigator.clipboard.writeText(diffText);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const getStatusBadge = (status: DiffFileStatus) => {
    switch (status) {
      case 'added':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold bg-emerald-50 text-[#2E8B57] border border-emerald-200">
            Added
          </span>
        );
      case 'modified':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold bg-amber-50 text-[#D97757] border border-amber-200">
            Modified
          </span>
        );
      case 'deleted':
        return (
          <span className="px-1.5 py-0.5 rounded text-[10px] font-mono font-semibold bg-red-50 text-red-700 border border-red-200">
            Deleted
          </span>
        );
    }
  };

  return (
    <div className="bg-white border border-slate-200 rounded-lg shadow-subtle flex flex-col h-full overflow-hidden">
      {/* 1. Top Toolbar */}
      <div className="px-4 py-3 border-b border-slate-200 bg-white flex flex-wrap items-center justify-between gap-3">
        {/* Left: File Selector & Change Overview */}
        <div className="flex flex-wrap items-center gap-3 min-w-0">
          {/* File Selector Dropdown */}
          <div className="flex items-center gap-1.5">
            <FileCode className="w-4 h-4 text-purple-600 shrink-0" />
            <select
              value={selectedFileIndex}
              onChange={(e) => setSelectedFileIndex(Number(e.target.value))}
              className="text-xs font-mono font-semibold bg-slate-50 border border-slate-200 rounded-md px-2.5 py-1 text-slate-800 focus:outline-none focus:ring-1 focus:ring-purple-500 max-w-[280px] sm:max-w-[340px] truncate"
            >
              {files.map((file, idx) => (
                <option key={file.path} value={idx}>
                  {file.path} {upperdirWiped ? '(Clean / Reverted)' : `(${file.status})`}
                </option>
              ))}
            </select>
          </div>

          {upperdirWiped ? (
            <span className="px-2 py-0.5 rounded text-[10px] font-mono font-bold bg-emerald-50 text-emerald-800 border border-emerald-300 flex items-center gap-1 shadow-2xs">
              <CheckCircle2 className="w-3 h-3 text-emerald-600" />
              Reverted Clean
            </span>
          ) : (
            selectedFile && getStatusBadge(selectedFile.status)
          )}

          {/* Change Overview Pill */}
          {upperdirWiped ? (
            <div className="hidden sm:flex items-center gap-2 text-xs font-mono">
              <span className="text-emerald-800 font-semibold bg-emerald-50 px-2 py-0.5 rounded border border-emerald-300 flex items-center gap-1">
                <Check className="w-3 h-3 text-emerald-600" />
                0 dirty hunks (Bit-Identical)
              </span>
            </div>
          ) : (
            selectedFile && (
              <div className="hidden sm:flex items-center gap-2 text-xs font-mono">
                <span className="text-[#2E8B57] font-semibold bg-emerald-50 px-2 py-0.5 rounded border border-emerald-200">
                  +{selectedFile.additions} lines
                </span>
                <span className="text-red-700 font-semibold bg-red-50 px-2 py-0.5 rounded border border-red-200">
                  -{selectedFile.deletions} lines
                </span>
              </div>
            )
          )}
        </div>

        {/* Right: Prev/Next File, AI Panel Toggle, Copy & Rollback */}
        <div className="flex items-center gap-2">
          {/* Prev / Next Toggles */}
          <div className="flex items-center border border-slate-200 rounded-md overflow-hidden bg-slate-50">
            <button
              onClick={handlePrevFile}
              className="p-1 hover:bg-slate-100 text-slate-600 border-r border-slate-200 transition-colors"
              title="Previous file"
            >
              <ChevronLeft className="w-3.5 h-3.5" />
            </button>
            <span className="px-2 text-[11px] font-mono text-slate-500 select-none">
              {files.length > 0 ? `${selectedFileIndex + 1}/${files.length}` : '0/0'}
            </span>
            <button
              onClick={handleNextFile}
              className="p-1 hover:bg-slate-100 text-slate-600 transition-colors"
              title="Next file"
            >
              <ChevronRight className="w-3.5 h-3.5" />
            </button>
          </div>

          {/* Toggle AI Diff Explanation */}
          <button
            onClick={() => setShowAiExplanation(!showAiExplanation)}
            className={`px-2.5 py-1 text-xs rounded-md border font-medium transition-colors flex items-center gap-1.5 ${
              showAiExplanation
                ? 'bg-purple-50 text-purple-700 border-purple-200'
                : 'bg-white text-slate-600 border-slate-200 hover:bg-slate-50'
            }`}
            title="Toggle AI Diff Explanation Panel"
          >
            <Sparkles className="w-3.5 h-3.5 text-purple-600" />
            <span className="hidden md:inline">AI Explanation</span>
            {showAiExplanation ? <ChevronUp className="w-3 h-3" /> : <ChevronDown className="w-3 h-3" />}
          </button>

          {/* Copy Diff */}
          <button
            onClick={handleCopy}
            className="p-1.5 text-xs rounded-md border border-slate-200 text-slate-600 hover:bg-slate-50 transition-colors"
            title="Copy Diff"
          >
            {copied ? <Check className="w-3.5 h-3.5 text-[#2E8B57]" /> : <Copy className="w-3.5 h-3.5" />}
          </button>

          {/* Active Baseline vs Rollback to State Button */}
          {isActiveBaseline && upperdirWiped ? (
            <div className="flex items-center gap-2">
              <span className="inline-flex items-center gap-1.5 px-3 py-1 rounded-md text-xs font-semibold bg-emerald-50 text-emerald-800 border border-emerald-300 shadow-2xs">
                <CheckCircle2 className="w-3.5 h-3.5 text-emerald-600 shrink-0" />
                Active Baseline (Restored)
              </span>
              {onMutateUpperdir && (
                <button
                  onClick={onMutateUpperdir}
                  className="px-2.5 py-1 text-xs rounded-md bg-amber-50 hover:bg-amber-100 border border-amber-300 text-amber-900 font-medium transition-colors shadow-2xs"
                  title="Simulate agent modifying code to demo rollback again"
                >
                  Mutate (+18.4MB)
                </button>
              )}
            </div>
          ) : (
            <Button
              variant="primary"
              size="sm"
              icon={<RotateCcw className="w-3.5 h-3.5" />}
              isLoading={isRollingBack}
              onClick={() => onRollback(snapshot.id)}
              className="font-semibold shadow-xs"
            >
              Rollback to State
            </Button>
          )}
        </div>
      </div>

      {/* 2. Optional Contextual AI Panel: Collapsible Callout Box */}
      {showAiExplanation && (
        <div className="p-3.5 bg-purple-50/80 border-b border-purple-200 text-xs transition-all">
          <div className="flex items-center justify-between mb-1.5">
            <div className="flex items-center gap-1.5 font-semibold text-purple-900">
              <Sparkles className="w-4 h-4 text-purple-600" />
              <span>AI Diff Explanation & Context</span>
              <span className="text-[10px] font-mono text-purple-700 bg-purple-100 px-1.5 py-0.2 rounded border border-purple-200">
                {selectedFile?.path.split('/').pop()}
              </span>
            </div>
            <span className="text-[10px] text-purple-700 font-mono">
              Auto-generated by shadow-mcp
            </span>
          </div>

          <ul className="space-y-1 text-slate-700 pl-4 list-disc text-[11px] leading-relaxed">
            {selectedFile?.aiExplanation ? (
              selectedFile.aiExplanation.map((point, idx) => (
                <li key={idx}>{point}</li>
              ))
            ) : snapshot.overallAiSummary ? (
              snapshot.overallAiSummary.map((point, idx) => (
                <li key={idx}>{point}</li>
              ))
            ) : (
              <li>Code alterations recorded in ephemeral upperdir tmpfs layer.</li>
            )}
          </ul>
        </div>
      )}

      {/* View Switcher: Checkpoint Diff vs Live Upperdir State */}
      <div className="bg-slate-100/90 px-4 py-1.5 border-b border-slate-200 flex items-center justify-between text-xs">
        <div className="flex items-center gap-2">
          <button
            onClick={() => setViewTab('diff')}
            className={`px-2.5 py-1 rounded text-xs font-medium transition-all ${
              viewTab === 'diff'
                ? 'bg-white text-purple-700 shadow-xs font-semibold'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            Checkpoint Snapshot Delta ({files.length} files)
          </button>
          <button
            onClick={() => setViewTab('live-tree')}
            className={`px-2.5 py-1 rounded text-xs font-medium transition-all flex items-center gap-1.5 ${
              viewTab === 'live-tree'
                ? 'bg-white text-purple-700 shadow-xs font-semibold'
                : 'text-slate-600 hover:text-slate-900'
            }`}
          >
            <FolderTree className="w-3.5 h-3.5" />
            Live Working Tree ({upperdirWiped ? '0 uncommitted' : '18.4 MB dirty'})
          </button>
        </div>
        <span className="text-[11px] font-mono text-slate-500 hidden sm:inline">
          OverlayFS 4-Layer CoW
        </span>
      </div>

      {/* Restored Banner if active baseline */}
      {isActiveBaseline && upperdirWiped && viewTab === 'diff' && (
        <div className="bg-emerald-50/90 border-b border-emerald-200 px-4 py-2 text-xs flex items-center justify-between text-emerald-900">
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0" />
            <span className="font-semibold">Current State Restored:</span>
            <span className="text-emerald-700 font-mono text-[11px]">
              Upperdir scratch layer wiped in {snapshot.rollbackLatencyMs} ms. Working tree matches this baseline.
            </span>
          </div>
          <Badge variant="active" size="sm">Bit-Identical</Badge>
        </div>
      )}

      {/* 3. Live Tree Mode vs Code Diff Display */}
      {viewTab === 'live-tree' ? (
        <div className="p-8 text-center bg-slate-900 text-slate-100 flex-1 min-h-[460px] flex flex-col items-center justify-center">
          {upperdirWiped ? (
            <>
              <div className="w-12 h-12 rounded-full bg-emerald-500/20 text-emerald-400 flex items-center justify-center mx-auto mb-3">
                <CheckCircle2 className="w-6 h-6" />
              </div>
              <h4 className="text-sm font-bold text-white mb-1">
                Ephemeral Scratch Layer Wiped — 100% Bit-Identical
              </h4>
              <p className="text-xs text-slate-400 max-w-lg mx-auto mb-4 leading-relaxed">
                The instant rollback primitive (<code className="text-purple-300">rollback_state</code>) cleared in <span className="text-emerald-400 font-semibold">{snapshot.rollbackLatencyMs} ms</span>. Ephemeral memory modifications were discarded without touching the host lowerdir.
              </p>
              <div className="inline-flex flex-wrap items-center justify-center gap-4 bg-slate-800/90 px-4 py-2.5 rounded-lg border border-slate-700 text-xs font-mono text-slate-300">
                <span>Ephemeral upperdir: <b className="text-emerald-400">0.0 MB</b></span>
                <span>·</span>
                <span>Dirty hunks: <b className="text-emerald-400">0 files</b></span>
                <span>·</span>
                <span>Host SHA-256: <b className="text-purple-300">a2f51b41... (Match)</b></span>
              </div>
              {onMutateUpperdir && (
                <div className="mt-5">
                  <Button
                    variant="secondary"
                    size="sm"
                    icon={<Sparkles className="w-3.5 h-3.5 text-amber-400" />}
                    onClick={onMutateUpperdir}
                  >
                    Simulate Agent Code Mutation (+18.4 MB)
                  </Button>
                </div>
              )}
            </>
          ) : (
            <>
              <div className="w-12 h-12 rounded-full bg-amber-500/20 text-amber-400 flex items-center justify-center mx-auto mb-3">
                <FolderTree className="w-6 h-6" />
              </div>
              <h4 className="text-sm font-bold text-white mb-1">
                Dirty Scratch State Detected (18.4 MB Uncommitted)
              </h4>
              <p className="text-xs text-slate-400 max-w-lg mx-auto mb-4 leading-relaxed">
                Autonomous agent has modified files in tmpfs upperdir scratch layer. Host lowerdir remains clean. Click "Rollback to State" to discard all uncommitted changes in under 100 ms.
              </p>
              <div className="inline-flex flex-wrap items-center justify-center gap-4 bg-slate-800/90 px-4 py-2.5 rounded-lg border border-slate-700 text-xs font-mono text-slate-300">
                <span>Ephemeral upperdir: <b className="text-amber-400">18.4 MB</b></span>
                <span>·</span>
                <span>Dirty files: <b className="text-amber-400">{files.length} modified</b></span>
              </div>
              <div className="mt-5">
                <Button
                  variant="primary"
                  size="sm"
                  icon={<RotateCcw className="w-3.5 h-3.5" />}
                  isLoading={isRollingBack}
                  onClick={() => onRollback(snapshot.id)}
                >
                  Rollback to State (Wipe Scratch Layer)
                </Button>
              </div>
            </>
          )}
        </div>
      ) : (
      <div className="flex-1 overflow-y-auto bg-[#F8FAFC] font-mono text-[11.5px] leading-relaxed select-text min-h-[460px]">
        {selectedFile ? (
          <div className="divide-y divide-slate-100">
            {upperdirWiped && (
              <div className="bg-emerald-500/10 border-b border-emerald-500/20 px-4 py-2.5 text-xs text-emerald-950 flex items-center justify-between select-none">
                <div className="flex items-center gap-2">
                  <CheckCircle2 className="w-4 h-4 text-emerald-600 shrink-0" />
                  <span className="font-semibold text-emerald-900">Pristine Working Tree Restored:</span>
                  <span className="text-emerald-800 font-mono text-[11px]">
                    All uncommitted additions were wiped in {snapshot.rollbackLatencyMs} ms. Working tree matches host repository bit-identically.
                  </span>
                </div>
                <span className="text-[10px] font-mono font-bold bg-emerald-100 text-emerald-800 px-2 py-0.5 rounded border border-emerald-300">
                  CLEAN (0 diffs)
                </span>
              </div>
            )}
            {(upperdirWiped
              ? selectedFile.diffLines
                  .filter((l) => l.type !== 'add')
                  .map((l) => ({
                    ...l,
                    type: 'context',
                    text: l.text.replace(/^[-+]/, ' '),
                    newLineNumber: l.oldLineNumber || l.newLineNumber,
                  }))
              : selectedFile.diffLines
            ).map((line, idx) => {
              if (line.type === 'hunk') {
                return (
                  <div 
                    key={idx}
                    className="bg-slate-100 text-slate-600 px-4 py-1 text-[11px] font-semibold border-y border-slate-200 flex items-center justify-between select-none"
                  >
                    <span>{line.text}</span>
                    <span className="text-[10px] text-slate-400 font-sans">hunk header</span>
                  </div>
                );
              }

              const isAdd = line.type === 'add';
              const isDelete = line.type === 'delete';

              // Standard code review colors
              const rowBg = isAdd 
                ? 'bg-emerald-50/80 hover:bg-emerald-100/60' 
                : isDelete 
                ? 'bg-red-50/80 hover:bg-red-100/60' 
                : 'hover:bg-slate-100/60';

              const textColor = isAdd 
                ? 'text-emerald-900' 
                : isDelete 
                ? 'text-red-900' 
                : 'text-slate-800';

              const symbol = isAdd ? '+' : isDelete ? '-' : ' ';
              const symbolColor = isAdd ? 'text-[#2E8B57] font-bold' : isDelete ? 'text-red-600 font-bold' : 'text-slate-300';

              return (
                <div key={idx} className={`flex items-start transition-colors ${rowBg}`}>
                  {/* Left Line Number (old) */}
                  <span className="w-10 shrink-0 text-slate-400 text-right pr-2 select-none border-r border-slate-200/80 py-0.5 text-[10.5px]">
                    {line.oldLineNumber || ''}
                  </span>

                  {/* Right Line Number (new) */}
                  <span className="w-10 shrink-0 text-slate-400 text-right pr-2 select-none border-r border-slate-200/80 py-0.5 text-[10.5px]">
                    {line.newLineNumber || ''}
                  </span>

                  {/* Prefix Sign */}
                  <span className={`w-6 shrink-0 text-center select-none py-0.5 ${symbolColor}`}>
                    {symbol}
                  </span>

                  {/* Code Line Content */}
                  <span className={`flex-1 px-2 py-0.5 whitespace-pre-wrap break-all ${textColor}`}>
                    {line.text.replace(/^[+-]/, '')}
                  </span>
                </div>
              );
            })}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center p-12 text-slate-400 text-xs text-center">
            <FileText className="w-8 h-8 text-slate-300 mb-2" />
            <p>No modified files present in this snapshot checkpoint.</p>
          </div>
        )}
      </div>
      )}
    </div>
  );
};
