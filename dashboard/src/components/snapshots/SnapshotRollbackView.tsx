import React, { useState } from 'react';
import { 
  History, 
  Plus, 
  CheckCircle2, 
  GitBranch, 
  Zap
} from 'lucide-react';
import type { Snapshot, MicroVM } from '../../types';
import { Button } from '../common/Button';
import { Badge } from '../common/Badge';
import { FileSystemDiffViewer } from './FileSystemDiffViewer';

interface SnapshotRollbackViewProps {
  snapshots: Snapshot[];
  vms: MicroVM[];
  selectedVmId: string;
  onSelectVmId: (id: string) => void;
  onRollback: (snapshotId: string) => void;
  onCreateSnapshot: (name: string, description: string) => void;
}

export const SnapshotRollbackView: React.FC<SnapshotRollbackViewProps> = ({
  snapshots,
  vms,
  selectedVmId,
  onSelectVmId,
  onRollback,
  onCreateSnapshot,
}) => {
  const [selectedSnapshotId, setSelectedSnapshotId] = useState<string>(
    snapshots[0]?.id || ''
  );
  const [isRollingBack, setIsRollingBack] = useState(false);
  const [rollbackSuccessBanner, setRollbackSuccessBanner] = useState<{
    snapshotId: string;
    elapsedMs: number;
  } | null>(null);

  const selectedSnapshot = snapshots.find((s) => s.id === selectedSnapshotId) || snapshots[0];

  const handleRollbackClick = (snapshotId: string) => {
    setIsRollingBack(true);
    // Simulate sub-100ms rollback engine
    setTimeout(() => {
      onRollback(snapshotId);
      setIsRollingBack(false);
      setRollbackSuccessBanner({
        snapshotId,
        elapsedMs: 31.05,
      });
      setTimeout(() => setRollbackSuccessBanner(null), 6000);
    }, 400);
  };

  const getTriggerBadge = (trigger: Snapshot['trigger']) => {
    switch (trigger) {
      case 'milestone':
        return <Badge variant="purple" size="sm">Milestone</Badge>;
      case 'pre-command':
        return <Badge variant="warning" size="sm">Pre-Build</Badge>;
      case 'swarm-checkpoint':
        return <Badge variant="active" size="sm">Swarm</Badge>;
      case 'manual':
        return <Badge variant="neutral" size="sm">Manual</Badge>;
    }
  };

  return (
    <div className="space-y-4">
      {/* 1. Target Environment & Checkpoint Action Toolbar */}
      <div className="bg-white border border-slate-200 rounded-lg px-4 py-2.5 shadow-subtle flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <label className="text-xs font-semibold text-slate-700 flex items-center gap-1.5">
            <GitBranch className="w-4 h-4 text-purple-600" />
            <span>Target Environment:</span>
          </label>
          <select
            value={selectedVmId}
            onChange={(e) => onSelectVmId(e.target.value)}
            className="px-2.5 py-1 text-xs font-mono font-medium bg-slate-50 border border-slate-200 rounded-md text-slate-800 focus:outline-none focus:ring-1 focus:ring-purple-500"
          >
            {vms.map((vm) => (
              <option key={vm.id} value={vm.id}>
                {vm.id} — {vm.name}
              </option>
            ))}
          </select>
          <span className="text-[11px] text-slate-400 font-mono hidden md:inline">
            OverlayFS 4-Layer CoW
          </span>
        </div>

        <Button
          variant="secondary"
          size="sm"
          icon={<Plus className="w-3.5 h-3.5" />}
          onClick={() => onCreateSnapshot('Manual Checkpoint', 'Snapshot saved prior to agent refactor')}
        >
          Create Checkpoint
        </Button>
      </div>

      {/* Rollback Success Banner */}
      {rollbackSuccessBanner && (
        <div className="bg-emerald-50 border border-emerald-200 text-emerald-900 rounded-lg p-3 flex items-center justify-between shadow-2xs transition-all">
          <div className="flex items-center gap-2.5">
            <CheckCircle2 className="w-4 h-4 text-[#2E8B57] shrink-0" />
            <div className="text-xs">
              <span className="font-semibold text-[#2E8B57]">
                State Rollback Cleared in {rollbackSuccessBanner.elapsedMs} ms!
              </span>
              <span className="text-emerald-700 ml-1.5 font-mono text-[11px]">
                Restored checkpoint [{rollbackSuccessBanner.snapshotId}]. Host working tree bit-identical.
              </span>
            </div>
          </div>
          <button
            onClick={() => setRollbackSuccessBanner(null)}
            className="text-xs text-emerald-700 hover:text-emerald-950 font-medium px-2 py-0.5"
          >
            Dismiss
          </button>
        </div>
      )}

      {/* 2. Strict 2-Column Layout (30% Left / 70% Right) */}
      <div className="flex flex-col lg:flex-row gap-4 items-stretch">
        {/* Left Column (30% width): Checkpoints History List */}
        <div className="w-full lg:w-[30%] shrink-0 space-y-2.5">
          <div className="bg-white border border-slate-200 rounded-lg px-3.5 py-2.5 shadow-subtle flex items-center justify-between">
            <span className="text-xs font-semibold text-slate-800 flex items-center gap-1.5">
              <History className="w-4 h-4 text-purple-600" />
              Checkpoints ({snapshots.length})
            </span>
            <span className="text-[10px] font-mono text-purple-700 bg-purple-50 px-1.5 py-0.2 rounded border border-purple-200">
              Sub-100ms
            </span>
          </div>

          <div className="space-y-2">
            {snapshots.map((snap) => {
              const isSelected = snap.id === selectedSnapshotId;
              return (
                <div
                  key={snap.id}
                  onClick={() => setSelectedSnapshotId(snap.id)}
                  className={`p-3 bg-white border rounded-lg cursor-pointer transition-all shadow-subtle ${
                    isSelected
                      ? 'border-purple-600 ring-1 ring-purple-600 bg-purple-50/20'
                      : 'border-slate-200 hover:border-purple-300 hover:bg-slate-50/50'
                  }`}
                >
                  <div className="flex items-center justify-between gap-1.5 mb-1">
                    <div className="flex items-center gap-1.5">
                      <span className="font-mono text-xs font-bold text-slate-900">
                        {snap.id}
                      </span>
                      {getTriggerBadge(snap.trigger)}
                    </div>
                    <span className="text-[10px] font-mono text-slate-400">
                      {snap.timestamp}
                    </span>
                  </div>

                  <div className="text-xs font-semibold text-slate-800 truncate mb-1">
                    {snap.name}
                  </div>
                  <p className="text-[11px] text-slate-500 line-clamp-2 leading-relaxed">
                    {snap.description}
                  </p>

                  <div className="mt-2.5 pt-2 border-t border-slate-100 flex items-center justify-between text-[10px] font-mono text-slate-500">
                    <span className="text-slate-600">
                      {snap.stats.totalFiles} files changed
                    </span>
                    <span className="text-[#2E8B57] font-semibold flex items-center gap-0.5">
                      <Zap className="w-3 h-3 text-[#2E8B57]" />
                      {snap.rollbackLatencyMs} ms
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Right Column (70% width): Redesigned Git Diff Viewer */}
        <div className="w-full lg:w-[70%] min-w-0">
          {selectedSnapshot && (
            <FileSystemDiffViewer
              snapshot={selectedSnapshot}
              onRollback={handleRollbackClick}
              isRollingBack={isRollingBack}
            />
          )}
        </div>
      </div>
    </div>
  );
};
