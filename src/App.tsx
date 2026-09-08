import { useState } from 'react';
import { AboutDialog } from '@/components/AboutDialog';
import DropZone from '@/components/DropZone';
import { GlobalSettingsDialog } from '@/components/GlobalSettingsDialog';
import TabBar from '@/components/TabBar';
import TabPanel from '@/components/TabPanel';
import { Tabs } from '@/components/ui/tabs';
import { useTabsStore } from '@/lib/store';
import { usePrimaryColor } from '@/lib/usePrimaryColor';
import { useTheme } from '@/lib/useTheme';

function App() {
  const [showDrop, setShowDrop] = useState(false);
  const { tabs, activeTabId, addTab, closeTab, selectTab, renameTab, addDownload } = useTabsStore();
  const { theme, toggleTheme } = useTheme();
  const { colorName, setColorName } = usePrimaryColor();
  const resolvedActiveTabId = activeTabId || tabs[0]?.id || '';

  const handleDrop = (value: string) => {
    setShowDrop(false);
    if (resolvedActiveTabId) {
      addDownload(resolvedActiveTabId, value);
    }
  };

  const handleDragEnter = () => {
    setShowDrop(true);
  };

  const handleDragLeave = () => {
    setShowDrop(false);
  };

  return (
    <div className="relative flex min-h-0 flex-1 flex-col" onDragEnter={handleDragEnter}>
      <DropZone show={showDrop} onDrop={handleDrop} onLeave={handleDragLeave} />
      <div className="absolute top-0 right-0 z-20 flex gap-1">
        <GlobalSettingsDialog
          theme={theme}
          onToggleTheme={toggleTheme}
          colorName={colorName}
          onColorChange={setColorName}
        />
        <AboutDialog />
      </div>
      <Tabs value={resolvedActiveTabId} className="flex min-h-0 flex-1 flex-col">
        <TabBar
          tabs={tabs}
          onTabSelect={selectTab}
          onTabClose={closeTab}
          onTabAdd={addTab}
          onTabRename={renameTab}
        />
        {tabs.map(tab => (
          <TabPanel key={tab.id} tabId={tab.id} />
        ))}
      </Tabs>
    </div>
  );
}

export default App;
