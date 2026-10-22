import React, { useState } from 'react';
import { 
  Plus, 
  Search, 
  Layers
} from 'lucide-react';
import { Button } from '../common/Button';
import type { ActiveTab, BackendConnectionStatus, SystemInfo } from '../../types';

interface HeaderProps {
  activeTab: ActiveTab;
  onSpawnClick: () => void;
  searchQuery: string;
  onSearchChange: (query: string) => void;
  aggregateRamMb: number;
  connectionStatus?: BackendConnectionStatus;
  systemInfo?: SystemInfo | null;
}

export const Header: React.FC<HeaderProps> = ({
  activeTab,
  onSpawnClick,
  searchQuery,
  onSearchChange,
  aggregateRamMb,
  connectionStatus = 'offline',
  systemInfo,
}) => {
  const [workspace, setWorkspace] = useState('production-swarm');

  const getTabTitle = () => {
    switch (activeTab) {
      case 'sandboxes':
        return 'Sandbox Manager';
      case 'snapshots':
        return 'Snapshot & Rollback';
      case 'mcp-console':
        return 'MCP / Swarm Console';
    }
  };

  const renderConnectionBadge = () => {
    if (connectionStatus === 'connected') {
      return (
        <div 
          className="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-medium bg-emerald-50 text-emerald-700 border border-emerald-200 rounded-md"
          title={`Connected to shadow-server daemon. Driver: ${systemInfo?.hypervisorDriver || 'Adaptive'}`}
        >
          <span className="w-2 h-2 rounded-full bg-emerald-500 animate-pulse shrink-0" />
          <span className="hidden sm:inline">Backend:</span>
          <span>Live Connected</span>
          {systemInfo?.hypervisorDriver && (
            <span className="text-[10px] font-mono px-1 py-0.2 bg-emerald-100 text-emerald-800 rounded">
              {systemInfo.hypervisorDriver}
            </span>
          )}
        </div>
      );
    }
    if (connectionStatus === 'connecting') {
      return (
        <div className="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-medium bg-amber-50 text-amber-700 border border-amber-200 rounded-md">
          <span className="w-2 h-2 rounded-full bg-amber-500 animate-ping shrink-0" />
          <span>Connecting...</span>
        </div>
      );
    }
    return (
      <div 
        className="flex items-center gap-1.5 px-2.5 py-1 text-[11px] font-medium bg-slate-100 text-slate-600 border border-slate-200 rounded-md"
        title="Running in resilient fallback mode with simulated microVM telemetry. Launch 'cargo run -p shadow-server' to connect."
      >
        <span className="w-2 h-2 rounded-full bg-slate-400 shrink-0" />
        <span className="hidden sm:inline">Backend:</span>
        <span>Fallback Mode</span>
      </div>
    );
  };

  return (
    <header className="h-16 px-6 bg-white border-b border-slate-200 flex items-center justify-between gap-4 sticky top-0 z-10 select-none">
      {/* Left: View Title & Workspace Selector */}
      <div className="flex items-center gap-4 min-w-0">
        <div>
          <h1 className="text-sm font-bold text-slate-900 tracking-tight">{getTabTitle()}</h1>
        </div>

        <div className="h-4 w-px bg-slate-200 hidden sm:block" />

        {/* Workspace Selector */}
        <div className="relative hidden sm:block">
          <div className="flex items-center gap-1.5 px-2.5 py-1 text-xs bg-slate-50 border border-slate-200 rounded-md text-slate-700">
            <Layers className="w-3.5 h-3.5 text-purple-600 shrink-0" />
            <select
              value={workspace}
              onChange={(e) => setWorkspace(e.target.value)}
              className="bg-transparent font-medium text-slate-800 pr-3 focus:outline-none cursor-pointer text-xs"
            >
              <option value="production-swarm">prod-swarm-cluster (Active)</option>
              <option value="staging-eval">staging-eval-pool</option>
              <option value="local-dev">local-libkrun-ephemeral</option>
            </select>
          </div>
        </div>

        {/* Backend Connection Indicator */}
        <div className="hidden md:block">
          {renderConnectionBadge()}
        </div>
      </div>

      {/* Right: Search, Swarm Memory, Single Primary Spawn Button */}
      <div className="flex items-center gap-3">
        {/* Search */}
        <div className="relative hidden md:block w-52 lg:w-64">
          <Search className="w-3.5 h-3.5 text-slate-400 absolute left-2.5 top-1/2 -translate-y-1/2" />
          <input
            type="text"
            value={searchQuery}
            onChange={(e) => onSearchChange(e.target.value)}
            placeholder="Search VMs, files, hunks..."
            className="w-full pl-8 pr-3 py-1.5 text-xs bg-slate-50 border border-slate-200 rounded-md text-slate-800 placeholder-slate-400 focus:outline-none focus:ring-1 focus:ring-purple-500 focus:border-purple-500 focus:bg-white transition-colors"
          />
        </div>

        {/* Swarm Memory Indicator */}
        <div className="hidden lg:flex items-center gap-2.5 px-3 py-1.5 bg-slate-50 border border-slate-200 rounded-md text-xs">
          <div className="flex flex-col">
            <div className="flex items-center justify-between gap-3 text-[10px] text-slate-500">
              <span className="font-medium text-slate-600">Swarm Memory</span>
              <span className="font-mono text-slate-700">{aggregateRamMb} MB / 2500 MB</span>
            </div>
            <div className="w-28 h-1.5 bg-slate-200 rounded-full overflow-hidden mt-1">
              <div 
                className="h-full bg-purple-600 rounded-full transition-all duration-300"
                style={{ width: `${Math.min(100, (aggregateRamMb / 2500) * 100)}%` }}
              />
            </div>
          </div>
        </div>

        {/* Single Primary Action Button in top right */}
        <Button
          variant="primary"
          size="md"
          icon={<Plus className="w-4 h-4" />}
          onClick={onSpawnClick}
          className="shadow-sm font-medium"
        >
          Spawn Sandbox
        </Button>
      </div>
    </header>
  );
};
