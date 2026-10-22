import React from 'react';
import { Sidebar } from './Sidebar';
import { Header } from './Header';
import type { ActiveTab, BackendConnectionStatus, SystemInfo } from '../../types';

interface LayoutProps {
  activeTab: ActiveTab;
  onTabChange: (tab: ActiveTab) => void;
  activeVmCount: number;
  snapshotCount: number;
  onSpawnClick: () => void;
  searchQuery: string;
  onSearchChange: (query: string) => void;
  aggregateRamMb: number;
  connectionStatus?: BackendConnectionStatus;
  systemInfo?: SystemInfo | null;
  children: React.ReactNode;
}

export const Layout: React.FC<LayoutProps> = ({
  activeTab,
  onTabChange,
  activeVmCount,
  snapshotCount,
  onSpawnClick,
  searchQuery,
  onSearchChange,
  aggregateRamMb,
  connectionStatus,
  systemInfo,
  children,
}) => {
  return (
    <div className="flex h-screen bg-slate-50 overflow-hidden font-sans text-slate-800">
      {/* Sidebar Navigation */}
      <Sidebar
        activeTab={activeTab}
        onTabChange={onTabChange}
        activeVmCount={activeVmCount}
        snapshotCount={snapshotCount}
      />

      {/* Main Content Area */}
      <div className="flex flex-col flex-1 min-w-0 overflow-hidden">
        {/* Header */}
        <Header
          activeTab={activeTab}
          onSpawnClick={onSpawnClick}
          searchQuery={searchQuery}
          onSearchChange={onSearchChange}
          aggregateRamMb={aggregateRamMb}
          connectionStatus={connectionStatus}
          systemInfo={systemInfo}
        />

        {/* Scrollable Dashboard View */}
        <main className="flex-1 overflow-y-auto p-4 sm:p-6 bg-slate-50">
          <div className="max-w-7xl mx-auto space-y-5">
            {children}
          </div>
        </main>
      </div>
    </div>
  );
};
