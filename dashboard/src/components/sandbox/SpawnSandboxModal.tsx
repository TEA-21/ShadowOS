import React, { useState } from 'react';
import { Modal } from '../common/Modal';
import { Button } from '../common/Button';
import { HardDrive, Shield } from 'lucide-react';
import type { MicroVM } from '../../types';

interface SpawnSandboxModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSpawn: (newVm: Partial<MicroVM>) => void;
}

export const SpawnSandboxModal: React.FC<SpawnSandboxModalProps> = ({
  isOpen,
  onClose,
  onSpawn,
}) => {
  const [name, setName] = useState('worker-task-ephemeral');
  const [hypervisor, setHypervisor] = useState<'libkrun' | 'firecracker'>('libkrun');
  const [vcpus, setVcpus] = useState<number>(2);
  const [ramMb, setRamMb] = useState<number>(512);
  const [enableDax, setEnableDax] = useState<boolean>(true);
  const [workloadPreset, setWorkloadPreset] = useState<string>('agent-code-exec');

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const id = `vm-${Math.random().toString(36).substring(2, 10)}`;
    const newVm: Partial<MicroVM> = {
      id,
      name: name || 'worker-ephemeral',
      hypervisor,
      status: 'Active',
      uptime: '1s',
      ramUsage: {
        currentMb: 140,
        limitMb: ramMb,
        balloonedMb: Math.max(0, ramMb - 140),
        percent: Math.round((140 / ramMb) * 100),
      },
      cpuAllocation: {
        vcpus,
        usagePercent: 6.5,
        pinnedCore: Math.floor(Math.random() * 4),
      },
      cowUpperSize: '0.0 MB',
      vsockCid: Math.floor(Math.random() * 20) + 8,
      guestAgentStatus: 'Connected',
      workloadProfile: 
        workloadPreset === 'agent-code-exec'
          ? 'DAX Direct-Map + 4-Layer CoW'
          : workloadPreset === 'browser-cdp'
          ? 'Xvfb (DISPLAY=:99) + Raw CDP Bridge'
          : 'High-Throughput Swarm Task Runner',
      ipAddress: `192.168.127.${Math.floor(Math.random() * 200) + 10}`,
      createdAt: new Date().toLocaleTimeString(),
    };

    onSpawn(newVm);
    onClose();
  };

  return (
    <Modal
      isOpen={isOpen}
      onClose={onClose}
      title={
        <div className="flex items-center gap-2">
          <span>Provision New MicroVM Sandbox</span>
          <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-purple-100 text-purple-700 border border-purple-200">
            &lt; 25ms Cold Boot
          </span>
        </div>
      }
      subtitle="Spawn an isolated hardware microVM with bit-identical 4-layer OverlayFS and AF_VSOCK bridge."
      maxWidth="lg"
      footer={
        <>
          <Button variant="secondary" onClick={onClose}>
            Cancel
          </Button>
          <Button variant="primary" onClick={handleSubmit}>
            Spawn MicroVM
          </Button>
        </>
      }
    >
      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Sandbox Identifier */}
        <div>
          <label className="block text-xs font-semibold text-slate-700 mb-1">
            Sandbox Name / Identifier
          </label>
          <input
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            className="w-full px-3 py-2 text-xs font-mono bg-slate-50 border border-gray-200 rounded-md text-slate-800 focus:outline-none focus:ring-1 focus:ring-purple-500 focus:border-purple-500 focus:bg-white"
            placeholder="e.g. agent-workspace-lint"
            required
          />
        </div>

        {/* Hypervisor Driver Selection */}
        <div>
          <label className="block text-xs font-semibold text-slate-700 mb-1">
            Hypervisor Virtualization Driver
          </label>
          <div className="grid grid-cols-2 gap-3">
            <button
              type="button"
              onClick={() => setHypervisor('libkrun')}
              className={`p-3 rounded-lg border text-left transition-all ${
                hypervisor === 'libkrun'
                  ? 'border-purple-500 bg-purple-50/60 ring-1 ring-purple-500'
                  : 'border-gray-200 bg-white hover:bg-slate-50'
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-semibold text-xs text-slate-900">libkrun Driver</span>
                <span className="text-[10px] font-mono text-purple-700 bg-purple-100 px-1 rounded">Fastest</span>
              </div>
              <p className="text-[11px] text-slate-500 mt-1">
                TSI / Direct in-process isolation. Cold boot &lt; 22ms. Ideal for local dev & browser CDP.
              </p>
            </button>

            <button
              type="button"
              onClick={() => setHypervisor('firecracker')}
              className={`p-3 rounded-lg border text-left transition-all ${
                hypervisor === 'firecracker'
                  ? 'border-purple-500 bg-purple-50/60 ring-1 ring-purple-500'
                  : 'border-gray-200 bg-white hover:bg-slate-50'
              }`}
            >
              <div className="flex items-center justify-between">
                <span className="font-semibold text-xs text-slate-900">Firecracker VMM</span>
                <span className="text-[10px] font-mono text-slate-600 bg-slate-100 px-1 rounded">KVM Jail</span>
              </div>
              <p className="text-[11px] text-slate-500 mt-1">
                Strict seccomp jailed microVM. Sub-second boot, hardened multi-tenant boundary.
              </p>
            </button>
          </div>
        </div>

        {/* Workload Profile Preset */}
        <div>
          <label className="block text-xs font-semibold text-slate-700 mb-1">
            Workload Execution Profile
          </label>
          <select
            value={workloadPreset}
            onChange={(e) => setWorkloadPreset(e.target.value)}
            className="w-full px-3 py-2 text-xs bg-slate-50 border border-gray-200 rounded-md text-slate-800 focus:outline-none focus:ring-1 focus:ring-purple-500"
          >
            <option value="agent-code-exec">DAX Direct-Map + 4-Layer CoW (General Agent)</option>
            <option value="browser-cdp">Headless Virtual Display (Xvfb DISPLAY=:99 + Raw CDP)</option>
            <option value="swarm-runner">High-Throughput Swarm Task Runner (Pinned Core)</option>
          </select>
        </div>

        {/* Resource Allocation */}
        <div className="grid grid-cols-2 gap-4">
          <div>
            <label className="block text-xs font-semibold text-slate-700 mb-1 flex items-center justify-between">
              <span>vCPU Cores</span>
              <span className="font-mono text-purple-700 font-semibold">{vcpus} vCPUs</span>
            </label>
            <select
              value={vcpus}
              onChange={(e) => setVcpus(Number(e.target.value))}
              className="w-full px-3 py-2 text-xs bg-slate-50 border border-gray-200 rounded-md text-slate-800 focus:outline-none focus:ring-1 focus:ring-purple-500"
            >
              <option value={1}>1 vCPU (Pinned affinity)</option>
              <option value={2}>2 vCPUs (Recommended)</option>
              <option value={4}>4 vCPUs (Heavy compilation)</option>
            </select>
          </div>

          <div>
            <label className="block text-xs font-semibold text-slate-700 mb-1 flex items-center justify-between">
              <span>RAM Allocation Limit</span>
              <span className="font-mono text-purple-700 font-semibold">{ramMb} MB</span>
            </label>
            <select
              value={ramMb}
              onChange={(e) => setRamMb(Number(e.target.value))}
              className="w-full px-3 py-2 text-xs bg-slate-50 border border-gray-200 rounded-md text-slate-800 focus:outline-none focus:ring-1 focus:ring-purple-500"
            >
              <option value={256}>256 MB (Minimal worker)</option>
              <option value={512}>512 MB (Default worker)</option>
              <option value={1024}>1024 MB (Browser / Xvfb)</option>
              <option value={2048}>2048 MB (Heavy build)</option>
            </select>
          </div>
        </div>

        {/* virtio-fs DAX & CoW Config */}
        <div className="p-3 bg-slate-50 border border-gray-200 rounded-lg space-y-2 text-xs">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <HardDrive className="w-4 h-4 text-purple-600" />
              <div>
                <span className="font-medium text-slate-800">virtio-fs DAX Direct Memory Mapping</span>
                <p className="text-[10px] text-slate-500">Bypasses guest page cache for 130-229% NVMe speed</p>
              </div>
            </div>
            <input
              type="checkbox"
              checked={enableDax}
              onChange={(e) => setEnableDax(e.target.checked)}
              className="rounded text-purple-600 focus:ring-purple-500 h-4 w-4"
            />
          </div>

          <div className="pt-2 border-t border-gray-200/80 flex items-center justify-between text-[11px] text-slate-600">
            <span className="flex items-center gap-1.5">
              <Shield className="w-3.5 h-3.5 text-[#2E8B57]" />
              OverlayFS 4-Layer Isolation
            </span>
            <span className="font-mono text-[#2E8B57]">Host 100% Bit-Identical</span>
          </div>
        </div>
      </form>
    </Modal>
  );
};
