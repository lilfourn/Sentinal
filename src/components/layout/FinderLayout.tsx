import { useState } from 'react';
import { Sidebar } from './Sidebar';
import { Toolbar } from './Toolbar';
import { MainView } from './MainView';
import { QuickLook } from '../preview/QuickLook';
import { SettingsPanel } from '../Settings/SettingsPanel';
import { ChangesPanel } from '../ChangesPanel/ChangesPanel';
import { useOrganizeStore } from '../../stores/organize-store';

export function FinderLayout() {
  const [settingsOpen, setSettingsOpen] = useState(false);
  const { isOpen: isOrganizing } = useOrganizeStore();

  return (
    <div className="h-screen flex flex-col overflow-hidden">
      {/* Toolbar */}
      <Toolbar onOpenSettings={() => setSettingsOpen(true)} />

      {/* Main content area */}
      <div className="flex-1 flex overflow-hidden">
        {/* Sidebar */}
        <Sidebar />

        {/* Main file view */}
        <MainView />

        {/* Quick Look preview panel (hidden when organizing) */}
        {!isOrganizing && <QuickLook />}

        {/* AI Changes panel (shown when organizing) */}
        {isOrganizing && <ChangesPanel />}
      </div>

      {/* Settings panel */}
      <SettingsPanel
        isOpen={settingsOpen}
        onClose={() => setSettingsOpen(false)}
      />
    </div>
  );
}
