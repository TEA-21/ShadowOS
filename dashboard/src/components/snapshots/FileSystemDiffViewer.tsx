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
  FileText
} from 'lucide-react';
import type { Snapshot, FileDiffItem, DiffFileStatus } from '../../types';
import { Button } from '../common/Button';

interface FileSystemDiffViewerProps {
  snapshot: Snapshot;
  onRollback: (snapshotId: string) => void;
  isRollingBack: boolean;
}

export const FileSystemDiffViewer: React.FC<FileSystemDiffViewerProps> = ({
  snapshot,
  onRollback,
  isRollingBack,
}) => {
  const [selectedFileIndex, setSelectedFileIndex] = useState<number>(0);
  const [showAiExplanation, setShowAiExplanation] = useState<boolean>(true);
  const [copied, setCopied] = useState<boolean>(false);

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
                  {file.path} ({file.status})
                </option>
              ))}
            </select>
          </div>

          {selectedFile && getStatusBadge(selectedFile.status)}

          {/* Change Overview Pill */}
          {selectedFile && (
            <div className="hidden sm:flex items-center gap-2 text-xs font-mono">
              <span className="text-[#2E8B57] font-semibold bg-emerald-50 px-2 py-0.5 rounded border border-emerald-200">
                +{selectedFile.additions} lines
              </span>
              <span className="text-red-700 font-semibold bg-red-50 px-2 py-0.5 rounded border border-red-200">
                -{selectedFile.deletions} lines
              </span>
            </div>
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

          {/* Prominent Rollback to State Button */}
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

      {/* 3. Redesigned Code Diff Display (Inspired by standard code review tools) */}
      <div className="flex-1 overflow-y-auto bg-[#F8FAFC] font-mono text-[11.5px] leading-relaxed select-text min-h-[460px]">
        {selectedFile ? (
          <div className="divide-y divide-slate-100">
            {selectedFile.diffLines.map((line, idx) => {
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
    </div>
  );
};
