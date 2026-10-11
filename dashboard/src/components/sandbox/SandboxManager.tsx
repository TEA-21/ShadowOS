import React, { useState } from 'react';
import { 
  Play, 
  Pause, 
  Trash2, 
  Terminal, 
  Cpu, 
  Table as TableIcon, 
  LayoutGrid, 
  Filter
} from 'lucide-react';
import { Card } from '../common/Card';
import { Badge } from '../common/Badge';
import type { MicroVM, VmStatus } from '../../types';
import { SpawnSandboxModal } from './SpawnSandboxModal';
import { VmTerminalModal } from './VmTerminalModal';

interface SandboxManagerProps {
  vms: MicroVM[];
  onSpawnVm: (vm: Partial<MicroVM>) => void;
  onTogglePause: (id: string) => void;
  onKillVm: (id: string) => void;
}

export const SandboxManager: React.FC<SandboxManagerProps> = ({
  vms,
  onSpawnVm,
  onTogglePause,
  onKillVm,
}) => {
  const [viewMode, setViewMode] = useState<'table' | 'grid'>('table');
  const [statusFilter, setStatusFilter] = useState<'all' | VmStatus>('all');
  const [isSpawnModalOpen, setIsSpawnModalOpen] = useState(false);
  const [terminalVm, setTerminalVm] = useState<MicroVM | null>(null);

  const filteredVms = vms.filter((vm) => {
    if (statusFilter === 'all') return true;
    return vm.status === statusFilter;
  });

  const activeCount = vms.filter((v) => v.status === 'Active').length;
  const suspendedCount = vms.filter((v) => v.status === 'Suspended').length;
  const stoppedCount = vms.filter((v) => v.status === 'Stopped').length;

  const totalAllocatedRam = vms.reduce((acc, v) => acc + (v.status !== 'Stopped' ? v.ramUsage.currentMb : 0), 0);
  const totalLimitRam = vms.reduce((acc, v) => acc + (v.status !== 'Stopped' ? v.ramUsage.limitMb : 0), 0);

  const getStatusBadge = (status: VmStatus) => {
    switch (status) {
      case 'Active':
        return <Badge variant="active" dot size="sm">Active</Badge>;
      case 'Suspended':
        return <Badge variant="warning" dot size="sm">Suspended</Badge>;
      case 'Stopped':
        return <Badge variant="stopped" dot size="sm">Stopped</Badge>;
    }
  };

  return (
    <div className="space-y-4">
      {/* 1. Concise High-Focus Metric Strip */}
      <div className="grid grid-cols-2 sm:grid-cols-4 gap-3">
        <div className="bg-white border border-slate-200 rounded-lg p-3.5 shadow-subtle">
          <span className="text-[11px] font-medium text-slate-400 uppercase tracking-wider block">Running MicroVMs</span>
          <div className="flex items-baseline gap-2 mt-1">
            <span className="text-xl font-bold font-mono text-slate-900">{activeCount}</span>
            <span className="text-xs text-slate-400 font-mono">/ {vms.length} instances</span>
          </div>
        </div>

        <div className="bg-white border border-slate-200 rounded-lg p-3.5 shadow-subtle">
          <span className="text-[11px] font-medium text-slate-400 uppercase tracking-wider block">Suspended / Idle</span>
          <div className="flex items-baseline gap-2 mt-1">
            <span className="text-xl font-bold font-mono text-amber-600">{suspendedCount}</span>
            <span className="text-xs text-slate-400 font-mono">paused</span>
          </div>
        </div>

        <div className="bg-white border border-slate-200 rounded-lg p-3.5 shadow-subtle">
          <span className="text-[11px] font-medium text-slate-400 uppercase tracking-wider block">Swarm RAM Usage</span>
          <div className="flex items-baseline gap-2 mt-1">
            <span className="text-xl font-bold font-mono text-purple-700">{totalAllocatedRam} MB</span>
            <span className="text-xs text-slate-400 font-mono">/ {totalLimitRam} MB</span>
          </div>
        </div>

        <div className="bg-white border border-slate-200 rounded-lg p-3.5 shadow-subtle">
          <span className="text-[11px] font-medium text-slate-400 uppercase tracking-wider block">Cold Boot Latency</span>
          <div className="flex items-baseline gap-2 mt-1">
            <span className="text-xl font-bold font-mono text-[#2E8B57]">22.38 ms</span>
            <span className="text-xs text-slate-400 font-sans">avg</span>
          </div>
        </div>
      </div>

      {/* 2. Clean Toolbar (Filters & View Mode) */}
      <div className="bg-white border border-slate-200 rounded-lg px-4 py-2.5 shadow-subtle flex items-center justify-between gap-3">
        {/* Status Filter Pills */}
        <div className="flex items-center gap-1.5">
          <Filter className="w-3.5 h-3.5 text-slate-400 mr-1" />
          {(['all', 'Active', 'Suspended', 'Stopped'] as const).map((status) => (
            <button
              key={status}
              onClick={() => setStatusFilter(status)}
              className={`px-2.5 py-1 text-xs rounded-md transition-colors capitalize ${
                statusFilter === status
                  ? 'bg-purple-50 text-purple-700 font-semibold border border-purple-200 shadow-xs'
                  : 'text-slate-600 hover:text-slate-900 hover:bg-slate-50 border border-transparent'
              }`}
            >
              {status}
              {status === 'all' && ` (${vms.length})`}
              {status === 'Active' && ` (${activeCount})`}
              {status === 'Suspended' && ` (${suspendedCount})`}
              {status === 'Stopped' && ` (${stoppedCount})`}
            </button>
          ))}
        </div>

        {/* View Switcher only (NO duplicate spawn button) */}
        <div className="flex items-center border border-slate-200 rounded-md p-0.5 bg-slate-50">
          <button
            onClick={() => setViewMode('table')}
            className={`p-1 rounded text-xs transition-colors ${
              viewMode === 'table' ? 'bg-white text-purple-700 shadow-xs font-medium' : 'text-slate-500 hover:text-slate-800'
            }`}
            title="Table view"
          >
            <TableIcon className="w-4 h-4" />
          </button>
          <button
            onClick={() => setViewMode('grid')}
            className={`p-1 rounded text-xs transition-colors ${
              viewMode === 'grid' ? 'bg-white text-purple-700 shadow-xs font-medium' : 'text-slate-500 hover:text-slate-800'
            }`}
            title="Grid view"
          >
            <LayoutGrid className="w-4 h-4" />
          </button>
        </div>
      </div>

      {/* 3. Clean Data Grid (Simplified Columns & Rows) */}
      {viewMode === 'table' ? (
        <Card padding="none" className="overflow-hidden border-slate-200">
          <div className="overflow-x-auto">
            <table className="w-full text-left border-collapse text-xs">
              <thead>
                <tr className="bg-slate-50/70 border-b border-slate-200 text-slate-500 uppercase tracking-wider text-[11px] font-semibold">
                  <th className="py-3 px-4">VM ID / Name</th>
                  <th className="py-3 px-4">Status</th>
                  <th className="py-3 px-4">Uptime</th>
                  <th className="py-3 px-4">RAM Usage</th>
                  <th className="py-3 px-4">CPU Allocation</th>
                  <th className="py-3 px-4 text-right">Actions</th>
                </tr>
              </thead>
              <tbody className="divide-y divide-slate-100">
                {filteredVms.length === 0 ? (
                  <tr>
                    <td colSpan={6} className="text-center py-10 text-slate-400">
                      No microVMs match the selected filter.
                    </td>
                  </tr>
                ) : (
                  filteredVms.map((vm) => {
                    const isStopped = vm.status === 'Stopped';
                    return (
                      <tr 
                        key={vm.id}
                        className={`hover:bg-slate-50/70 transition-colors ${
                          isStopped ? 'opacity-60 bg-slate-50/20' : 'bg-white'
                        }`}
                      >
                        {/* 1. VM ID / Name */}
                        <td className="py-3.5 px-4">
                          <div className="flex items-center gap-2">
                            <span className="font-mono text-xs font-bold text-slate-900 bg-slate-100 px-1.5 py-0.5 rounded border border-slate-200">
                              {vm.id}
                            </span>
                            <span className="font-semibold text-slate-800 text-xs">
                              {vm.name}
                            </span>
                            <span className="text-[10px] font-mono text-purple-700 bg-purple-50 px-1 py-0.2 rounded border border-purple-200">
                              {vm.hypervisor}
                            </span>
                          </div>
                        </td>

                        {/* 2. Status */}
                        <td className="py-3.5 px-4">
                          {getStatusBadge(vm.status)}
                        </td>

                        {/* 3. Uptime */}
                        <td className="py-3.5 px-4 font-mono text-xs text-slate-700">
                          {vm.uptime}
                        </td>

                        {/* 4. RAM Usage */}
                        <td className="py-3.5 px-4 min-w-[170px]">
                          <div className="flex items-center justify-between text-xs font-mono mb-1">
                            <span className="text-slate-800 font-medium">
                              {vm.ramUsage.currentMb} MB
                            </span>
                            <span className="text-slate-400 text-[11px]">
                              / {vm.ramUsage.limitMb} MB
                            </span>
                          </div>
                          <div className="w-full h-1.5 bg-slate-100 rounded-full overflow-hidden border border-slate-200/50">
                            <div
                              className="h-full bg-purple-600 rounded-full transition-all duration-300"
                              style={{ width: `${Math.min(100, vm.ramUsage.percent)}%` }}
                            />
                          </div>
                        </td>

                        {/* 5. CPU Allocation */}
                        <td className="py-3.5 px-4">
                          <div className="flex items-center gap-1.5 font-mono text-xs text-slate-800 font-medium">
                            <Cpu className="w-3.5 h-3.5 text-purple-600 shrink-0" />
                            <span>{vm.cpuAllocation.vcpus} vCPU</span>
                            <span className="text-slate-400 text-[11px]">
                              ({vm.cpuAllocation.usagePercent}%)
                            </span>
                            {vm.cpuAllocation.pinnedCore !== undefined && (
                              <span className="text-[10px] text-slate-500 font-sans ml-1">
                                [Core #{vm.cpuAllocation.pinnedCore}]
                              </span>
                            )}
                          </div>
                        </td>

                        {/* 6. Compact Muted Outline Actions Button Group */}
                        <td className="py-3.5 px-4 text-right">
                          <div className="inline-flex items-center rounded-md border border-slate-200 divide-x divide-slate-200 bg-white shadow-2xs overflow-hidden">
                            {/* Pause / Resume */}
                            <button
                              disabled={isStopped}
                              onClick={() => onTogglePause(vm.id)}
                              className="px-2.5 py-1 text-xs font-medium text-slate-700 hover:bg-slate-50 hover:text-slate-900 disabled:opacity-40 disabled:hover:bg-white transition-colors flex items-center gap-1"
                              title={vm.status === 'Suspended' ? 'Resume microVM' : 'Pause microVM'}
                            >
                              {vm.status === 'Suspended' ? (
                                <>
                                  <Play className="w-3 h-3 text-[#2E8B57]" />
                                  <span>Resume</span>
                                </>
                              ) : (
                                <>
                                  <Pause className="w-3 h-3 text-[#D97757]" />
                                  <span>Pause</span>
                                </>
                              )}
                            </button>

                            {/* Terminal */}
                            <button
                              disabled={isStopped}
                              onClick={() => setTerminalVm(vm)}
                              className="px-2.5 py-1 text-xs font-medium text-slate-700 hover:bg-purple-50 hover:text-purple-700 disabled:opacity-40 disabled:hover:bg-white transition-colors flex items-center gap-1"
                              title="Open Terminal"
                            >
                              <Terminal className="w-3 h-3 text-purple-600" />
                              <span>Terminal</span>
                            </button>

                            {/* Kill */}
                            <button
                              disabled={isStopped}
                              onClick={() => onKillVm(vm.id)}
                              className="px-2 py-1 text-xs font-medium text-rose-600 hover:bg-rose-50 disabled:opacity-40 disabled:hover:bg-white transition-colors flex items-center"
                              title="Kill microVM"
                            >
                              <Trash2 className="w-3 h-3" />
                            </button>
                          </div>
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
        </Card>
      ) : (
        /* Grid View Mode */
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
          {filteredVms.map((vm) => {
            const isStopped = vm.status === 'Stopped';
            return (
              <div 
                key={vm.id}
                className={`bg-white border border-slate-200 rounded-lg p-4 shadow-subtle flex flex-col justify-between transition-all ${
                  isStopped ? 'opacity-60' : 'hover:border-purple-300'
                }`}
              >
                <div>
                  <div className="flex items-center justify-between pb-3 border-b border-slate-100">
                    <div className="flex items-center gap-1.5">
                      <span className="font-mono text-xs font-bold text-slate-900 bg-slate-100 px-1.5 py-0.5 rounded border border-slate-200">
                        {vm.id}
                      </span>
                      <span className="text-[10px] font-mono text-purple-700 bg-purple-50 px-1 py-0.2 rounded border border-purple-200">
                        {vm.hypervisor}
                      </span>
                    </div>
                    {getStatusBadge(vm.status)}
                  </div>

                  <div className="py-3 space-y-2 text-xs">
                    <div className="font-semibold text-slate-900 text-sm">{vm.name}</div>

                    <div className="flex justify-between text-xs font-mono text-slate-600 pt-1">
                      <span>RAM Footprint</span>
                      <span>{vm.ramUsage.currentMb} MB / {vm.ramUsage.limitMb} MB</span>
                    </div>
                    <div className="w-full h-1.5 bg-slate-100 rounded-full overflow-hidden border border-slate-200/50">
                      <div
                        className="h-full bg-purple-600 rounded-full"
                        style={{ width: `${Math.min(100, vm.ramUsage.percent)}%` }}
                      />
                    </div>

                    <div className="flex items-center justify-between text-xs text-slate-600 pt-1 font-mono">
                      <span>Uptime: {vm.uptime}</span>
                      <span>{vm.cpuAllocation.vcpus} vCPU ({vm.cpuAllocation.usagePercent}%)</span>
                    </div>
                  </div>
                </div>

                <div className="pt-3 border-t border-slate-100 flex items-center justify-end">
                  <div className="inline-flex items-center rounded-md border border-slate-200 divide-x divide-slate-200 bg-white overflow-hidden shadow-2xs">
                    <button
                      disabled={isStopped}
                      onClick={() => onTogglePause(vm.id)}
                      className="px-2.5 py-1 text-xs text-slate-700 hover:bg-slate-50 flex items-center gap-1 disabled:opacity-40"
                    >
                      {vm.status === 'Suspended' ? <Play className="w-3 h-3 text-[#2E8B57]" /> : <Pause className="w-3 h-3 text-[#D97757]" />}
                      <span>{vm.status === 'Suspended' ? 'Resume' : 'Pause'}</span>
                    </button>
                    <button
                      disabled={isStopped}
                      onClick={() => setTerminalVm(vm)}
                      className="px-2.5 py-1 text-xs text-slate-700 hover:bg-purple-50 hover:text-purple-700 flex items-center gap-1 disabled:opacity-40"
                    >
                      <Terminal className="w-3 h-3 text-purple-600" />
                      <span>Terminal</span>
                    </button>
                    <button
                      disabled={isStopped}
                      onClick={() => onKillVm(vm.id)}
                      className="px-2 py-1 text-xs text-rose-600 hover:bg-rose-50 flex items-center disabled:opacity-40"
                    >
                      <Trash2 className="w-3 h-3" />
                    </button>
                  </div>
                </div>
              </div>
            );
          })}
        </div>
      )}

      {/* Spawn Sandbox Modal */}
      <SpawnSandboxModal
        isOpen={isSpawnModalOpen}
        onClose={() => setIsSpawnModalOpen(false)}
        onSpawn={onSpawnVm}
      />

      {/* Terminal Modal */}
      <VmTerminalModal
        vm={terminalVm}
        isOpen={!!terminalVm}
        onClose={() => setTerminalVm(null)}
      />
    </div>
  );
};
