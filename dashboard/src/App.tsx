import React, { useState, useEffect } from 'react';
import { Layout } from './components/layout/Layout';
import { SandboxManager } from './components/sandbox/SandboxManager';
import { SnapshotRollbackView } from './components/snapshots/SnapshotRollbackView';
import { McpSwarmConsole } from './components/mcp/McpSwarmConsole';
import { SpawnSandboxModal } from './components/sandbox/SpawnSandboxModal';
import { 
  INITIAL_VMS, 
  MOCK_SNAPSHOTS, 
  MOCK_VSOCK_LOGS, 
  MOCK_AGENT_NOTES 
} from './data/mockData';
import type { MicroVM, Snapshot, VsockLogEntry, AgentNote, ActiveTab } from './types';

export const App: React.FC = () => {
  const [activeTab, setActiveTab] = useState<ActiveTab>('sandboxes');
  const [vms, setVms] = useState<MicroVM[]>(INITIAL_VMS);
  const [snapshots, setSnapshots] = useState<Snapshot[]>(MOCK_SNAPSHOTS);
  const [logs, setLogs] = useState<VsockLogEntry[]>(MOCK_VSOCK_LOGS);
  const [agentNotes, setAgentNotes] = useState<AgentNote[]>(MOCK_AGENT_NOTES);
  const [selectedVmId, setSelectedVmId] = useState<string>(INITIAL_VMS[0]?.id || '');
  const [isSpawnModalOpen, setIsSpawnModalOpen] = useState(false);
  const [isStreaming, setIsStreaming] = useState(true);
  const [searchQuery, setSearchQuery] = useState('');

  // Calculate live aggregate Swarm memory footprint
  const aggregateRamMb = vms.reduce(
    (acc, vm) => acc + (vm.status !== 'Stopped' ? vm.ramUsage.currentMb : 0),
    0
  );

  // Active microVM count
  const activeVmCount = vms.filter((v) => v.status === 'Active').length;

  // Real-time vsock packet simulation stream
  useEffect(() => {
    if (!isStreaming) return;

    const streamInterval = setInterval(() => {
      const simulatedChannels = ['vsock:3', 'vsock:4', 'virtio-balloon', 'mcp-jsonrpc', 'overlayfs'];
      const channel = simulatedChannels[Math.floor(Math.random() * simulatedChannels.length)];
      const activeList = vms.filter((v) => v.status === 'Active');
      const randomVm = activeList[Math.floor(Math.random() * activeList.length)] || vms[0];

      const now = new Date();
      const timestamp = `${now.toTimeString().split(' ')[0]}.${String(now.getMilliseconds()).padStart(3, '0')}`;

      let message = '';
      let jsonRpcPayload: string | undefined;
      let level: VsockLogEntry['level'] = 'INFO';

      if (channel === 'virtio-balloon') {
        const delta = Math.floor(Math.random() * 40) - 20;
        message = `Dynamic memory balloon heartbeat on ${randomVm.id}: RSS delta ${delta > 0 ? '+' : ''}${delta} MB. Idle footprint stable at ${randomVm.ramUsage.currentMb} MB.`;
        level = 'DEBUG';
      } else if (channel === 'mcp-jsonrpc') {
        message = `MCP tool inspection invoked: "inspect_diff" on ${randomVm.id}. Output unified diff framed over stdio.`;
        jsonRpcPayload = JSON.stringify({
          jsonrpc: '2.0',
          method: 'tools/call',
          params: { name: 'inspect_diff', arguments: { sandbox_id: randomVm.id } },
          id: `req-${Math.floor(Math.random() * 9000) + 1000}`
        });
      } else if (channel === 'overlayfs') {
        message = `OverlayFS upperdir sync: 2 hunks modified in tmpfs scratch layer for ${randomVm.id}. Host repo bit-identical.`;
        level = 'DEBUG';
      } else {
        const latency = (0.28 + Math.random() * 0.15).toFixed(2);
        message = `Zero-TCP packet stream on channel #${randomVm.vsockCid}. Multiplexed latency: ${latency} µs.`;
      }

      const newLog: VsockLogEntry = {
        id: `log-${Date.now()}-${Math.random().toString(36).substring(2, 6)}`,
        timestamp,
        level,
        channel,
        vmId: randomVm.id,
        message,
        jsonRpcPayload,
      };

      setLogs((prev) => [...prev.slice(-120), newLog]);
    }, 4200);

    return () => clearInterval(streamInterval);
  }, [isStreaming, vms]);

  // Handlers
  const handleSpawnVm = (newVmPartial: Partial<MicroVM>) => {
    const newVm = newVmPartial as MicroVM;
    setVms((prev) => [newVm, ...prev]);

    const initSnapshot: Snapshot = {
      id: `snap-${Math.floor(Math.random() * 9000) + 1000}-init`,
      vmId: newVm.id,
      name: `Baseline Hardware Boot: ${newVm.name}`,
      timestamp: new Date().toTimeString().split(' ')[0],
      date: '2026-10-10',
      description: `Initial hardware microVM boot via ${newVm.hypervisor}. DAX direct memory mapping active.`,
      trigger: 'manual',
      ramFootprintMb: newVm.ramUsage.currentMb,
      cowUpperSize: '0.0 MB',
      rollbackLatencyMs: 22.38,
      stats: { added: 0, modified: 0, deleted: 0, totalFiles: 0 },
      files: [],
    };

    setSnapshots((prev) => [initSnapshot, ...prev]);
  };

  const handleTogglePause = (id: string) => {
    setVms((prev) =>
      prev.map((vm) => {
        if (vm.id === id) {
          const nextStatus = vm.status === 'Suspended' ? 'Active' : 'Suspended';
          return { ...vm, status: nextStatus };
        }
        return vm;
      })
    );
  };

  const handleKillVm = (id: string) => {
    setVms((prev) =>
      prev.map((vm) => (vm.id === id ? { ...vm, status: 'Stopped', ramUsage: { ...vm.ramUsage, currentMb: 0, percent: 0 } } : vm))
    );
  };

  const handleRollback = (snapshotId: string) => {
    const targetSnapshot = snapshots.find((s) => s.id === snapshotId);
    if (!targetSnapshot) return;

    setVms((prev) =>
      prev.map((vm) => {
        if (vm.id === targetSnapshot.vmId) {
          return {
            ...vm,
            status: 'Active',
            cowUpperSize: '0.0 MB',
            ramUsage: {
              ...vm.ramUsage,
              currentMb: targetSnapshot.ramFootprintMb,
            },
          };
        }
        return vm;
      })
    );

    setLogs((prev) => [
      ...prev,
      {
        id: `log-rollback-${Date.now()}`,
        timestamp: new Date().toTimeString().split(' ')[0],
        level: 'INFO',
        channel: 'mcp-jsonrpc',
        vmId: targetSnapshot.vmId,
        message: `Rollback primitive "rollback_state" executed. Upperdir wiped, memory restored to [${snapshotId}] in ${targetSnapshot.rollbackLatencyMs} ms.`,
      },
    ]);
  };

  const handleCreateSnapshot = (name: string, description: string) => {
    const targetVm = vms.find((v) => v.id === selectedVmId) || vms[0];
    const newSnapshot: Snapshot = {
      id: `snap-${Math.floor(Math.random() * 9000) + 1000}-manual`,
      vmId: targetVm.id,
      name,
      timestamp: new Date().toTimeString().split(' ')[0],
      date: '2026-10-10',
      description,
      trigger: 'manual',
      ramFootprintMb: targetVm.ramUsage.currentMb,
      cowUpperSize: targetVm.cowUpperSize,
      rollbackLatencyMs: 31.05,
      stats: { added: 1, modified: 2, deleted: 0, totalFiles: 3 },
      files: snapshots[0]?.files.slice(0, 3) || [],
    };

    setSnapshots((prev) => [newSnapshot, ...prev]);
  };

  const handleInspectDiff = () => {
    setSelectedVmId('vm-4a7b2e9c');
    setActiveTab('snapshots');
  };

  const handlePromoteHostTree = (noteId: string) => {
    setAgentNotes((prev) =>
      prev.map((note) =>
        note.id === noteId ? { ...note, status: 'Completed' } : note
      )
    );

    const now = new Date();
    const timestamp = `${now.toTimeString().split(' ')[0]}.${String(now.getMilliseconds()).padStart(3, '0')}`;
    const promoteLog: VsockLogEntry = {
      id: `log-promote-${Date.now()}`,
      timestamp,
      level: 'INFO',
      channel: 'mcp-jsonrpc',
      vmId: 'vm-4a7b2e9c',
      message: 'MCP primitive "promote_change" executed. 6 files atomically merged into host repository (SHA256 verified).',
      jsonRpcPayload: JSON.stringify({
        jsonrpc: '2.0',
        method: 'tools/call',
        params: {
          name: 'promote_change',
          arguments: {
            sandbox_id: 'vm-4a7b2e9c',
            target_path: 'c:\\Users\\HP\\OneDrive\\Documents\\ShadowOs\\ShadowOS',
          },
        },
        result: {
          status: 'success',
          files_merged: 6,
          host_tree_checksum: 'a2f51b41...',
        },
        id: `req-${Date.now()}`,
      }),
    };
    setLogs((prev) => [...prev, promoteLog]);
  };

  return (
    <Layout
      activeTab={activeTab}
      onTabChange={setActiveTab}
      activeVmCount={activeVmCount}
      snapshotCount={snapshots.length}
      onSpawnClick={() => setIsSpawnModalOpen(true)}
      searchQuery={searchQuery}
      onSearchChange={setSearchQuery}
      aggregateRamMb={aggregateRamMb}
    >
      {/* 1. Sandbox Manager View */}
      {activeTab === 'sandboxes' && (
        <SandboxManager
          vms={vms}
          onSpawnVm={handleSpawnVm}
          onTogglePause={handleTogglePause}
          onKillVm={handleKillVm}
        />
      )}

      {/* 2. Snapshot & Rollback View */}
      {activeTab === 'snapshots' && (
        <SnapshotRollbackView
          snapshots={snapshots}
          vms={vms}
          selectedVmId={selectedVmId}
          onSelectVmId={setSelectedVmId}
          onRollback={handleRollback}
          onCreateSnapshot={handleCreateSnapshot}
        />
      )}

      {/* 3. MCP / Swarm Console View */}
      {activeTab === 'mcp-console' && (
        <McpSwarmConsole
          logs={logs}
          agentNotes={agentNotes}
          isStreaming={isStreaming}
          onToggleStreaming={() => setIsStreaming(!isStreaming)}
          onClearLogs={() => setLogs([])}
          onInspectDiff={handleInspectDiff}
          onPromoteHostTree={handlePromoteHostTree}
        />
      )}

      {/* Spawn Sandbox Modal */}
      <SpawnSandboxModal
        isOpen={isSpawnModalOpen}
        onClose={() => setIsSpawnModalOpen(false)}
        onSpawn={handleSpawnVm}
      />
    </Layout>
  );
};

export default App;
