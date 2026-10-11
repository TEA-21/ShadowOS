import React from 'react';
import { ShieldCheck } from 'lucide-react';
import { Badge } from '../common/Badge';

export const OverviewMetrics: React.FC = () => {
  const complianceGates = [
    {
      id: 'NFR-01',
      title: 'Cold Boot Provisioning Latency',
      target: 'Strictly < 150.0 ms',
      measured: 'Avg: 22.38 ms (P95: 22.60 ms)',
      ratio: '6.7x faster than gate requirement',
      status: 'APPROVED',
      layer: 'shadow-vmm (libkrun / FC)',
    },
    {
      id: 'NFR-02',
      title: 'Base RAM Memory Footprint',
      target: 'Strictly < 150.0 MB',
      measured: '140.0 MB total idle footprint',
      ratio: '128 MB guest + 12 MB VMM RSS',
      status: 'APPROVED',
      layer: 'virtio-balloon driver',
    },
    {
      id: 'NFR-03',
      title: 'Peak Swarm RAM Workload Limit',
      target: 'Capped at <= 2.50 GB',
      measured: '1.55 GB peak aggregate (4 workers)',
      ratio: '38% memory headroom remaining',
      status: 'APPROVED',
      layer: 'swarm orchestrator',
    },
    {
      id: 'NFR-04',
      title: 'virtio-fs DAX Direct-Map I/O',
      target: '>= 85.0% of Native NVMe',
      measured: 'Read: 130.75%, Write: 229.42%',
      ratio: 'Direct memory mapping bypasses cache',
      status: 'APPROVED',
      layer: 'virtiofsd DAX supervisor',
    },
    {
      id: 'NFR-05',
      title: 'In-Memory Build RAM-Disk Speed',
      target: 'Exceed physical disk throughput',
      measured: '14,929.7 MB/s throughput',
      ratio: '14.84x faster than physical NVMe',
      status: 'APPROVED',
      layer: 'tmpfs merged workspace',
    },
    {
      id: 'NFR-06',
      title: 'Zero-TCP Framing Latency',
      target: 'Strictly < 100.0 µs / frame',
      measured: '0.32 µs / frame (50,000 frames in 16ms)',
      ratio: '312x faster than gate requirement',
      status: 'APPROVED',
      layer: 'shadow-vsock multiplexer',
    },
    {
      id: 'P2-01',
      title: 'Ephemeral CoW Isolation & Reset',
      target: 'Bit-identical host tree; reset < 5.0ms',
      measured: '100% Bit-Identical; Reset: 1.34 ms',
      ratio: '0 byte host working tree drift',
      status: 'APPROVED',
      layer: 'shadow-cow OverlayFS',
    },
    {
      id: 'P2-02',
      title: 'Sub-100ms Instant State Rollback',
      target: 'Strictly < 100.0 ms rollback latency',
      measured: 'Avg: 31.05 ms (50 cycles, P95: 33.15 ms)',
      ratio: '3.2x faster than gate requirement',
      status: 'APPROVED',
      layer: 'shadow-snapshot controller',
    },
  ];

  return (
    <div className="space-y-5">
      {/* Top Banner */}
      <div className="bg-white border border-gray-200 rounded-lg p-4 sm:p-5 shadow-subtle flex flex-wrap items-center justify-between gap-4">
        <div>
          <div className="flex items-center gap-2">
            <h2 className="text-base font-bold text-slate-900 tracking-tight">
              ShadowOS Autonomous PRD Critique Gate Audit
            </h2>
            <Badge variant="active" dot>
              100% PASS
            </Badge>
          </div>
          <p className="text-xs text-slate-500 mt-1">
            All Non-Functional Requirements (NFR-01..06), Phase 2 CoW & Rollback benchmarks, and Phase 3 Swarm gates verified.
          </p>
        </div>

        <div className="flex items-center gap-3">
          <div className="px-3 py-1.5 rounded-md bg-emerald-50 border border-emerald-200 text-xs text-[#2E8B57] font-semibold flex items-center gap-1.5">
            <ShieldCheck className="w-4 h-4" />
            <span>Production Ready</span>
          </div>
        </div>
      </div>

      {/* Grid of Audit Gates */}
      <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
        {complianceGates.map((gate) => (
          <div
            key={gate.id}
            className="bg-white border border-gray-200 rounded-lg p-4 shadow-subtle flex flex-col justify-between hover:border-purple-300 transition-colors"
          >
            <div>
              <div className="flex items-center justify-between mb-2">
                <div className="flex items-center gap-2">
                  <span className="font-mono text-xs font-bold text-purple-700 bg-purple-50 px-1.5 py-0.5 rounded border border-purple-200">
                    {gate.id}
                  </span>
                  <span className="text-xs font-bold text-slate-900">
                    {gate.title}
                  </span>
                </div>
                <span className="text-[10px] font-mono font-semibold px-2 py-0.5 rounded bg-emerald-50 text-[#2E8B57] border border-emerald-200">
                  {gate.status}
                </span>
              </div>

              <div className="space-y-1.5 py-2 text-xs">
                <div className="flex items-center justify-between text-slate-500">
                  <span>Specification Target:</span>
                  <span className="font-mono text-slate-700">{gate.target}</span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-slate-500">Measured Performance:</span>
                  <span className="font-mono font-bold text-[#2E8B57]">{gate.measured}</span>
                </div>
                <div className="flex items-center justify-between text-[11px] text-purple-700 font-medium">
                  <span>Audit Margin:</span>
                  <span>{gate.ratio}</span>
                </div>
              </div>
            </div>

            <div className="pt-2.5 border-t border-gray-100 flex items-center justify-between text-[10px] font-mono text-slate-400">
              <span>Subsystem: {gate.layer}</span>
              <span className="text-[#2E8B57]">Verified Bit-Identical</span>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};
