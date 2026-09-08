import { Moon, Palette, Settings, Sun, X } from 'lucide-react';
import { useState } from 'react';
import { ColorPicker } from '@/components/ColorPicker';
import SavedLogins from '@/components/SavedLogins';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogTitle, DialogTrigger } from '@/components/ui/dialog';
import { useLoginsStore } from '@/lib/logins';
import type { PrimaryColorName } from '@/lib/usePrimaryColor';

interface GlobalSettingsDialogProps {
  theme: 'light' | 'dark';
  onToggleTheme: () => void;
  colorName: PrimaryColorName;
  onColorChange: (name: PrimaryColorName) => void;
}

export function GlobalSettingsDialog({
  theme,
  onToggleTheme,
  colorName,
  onColorChange,
}: GlobalSettingsDialogProps) {
  const [open, setOpen] = useState(false);
  const busy = useLoginsStore(state => state.busy);

  return (
    <Dialog
      open={open}
      onOpenChange={nextOpen => {
        if (nextOpen || !busy) setOpen(nextOpen);
      }}
    >
      <DialogTrigger asChild>
        <Button variant="ghost" size="icon" title="Settings" aria-label="Open settings">
          <Settings size={16} />
        </Button>
      </DialogTrigger>
      <DialogContent className="flex max-h-[calc(100dvh-2rem)] w-[calc(100%-2rem)] max-w-2xl flex-col overflow-hidden rounded-none p-0">
        <div className="flex shrink-0 items-center justify-between gap-4 border-b p-6">
          <DialogTitle>Settings</DialogTitle>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            disabled={busy}
            onClick={() => setOpen(false)}
            title="Close settings"
            aria-label="Close settings"
          >
            <X size={16} />
          </Button>
        </div>
        <div className="flex min-h-0 flex-col gap-6 overflow-y-auto p-6">
          <section className="flex flex-col gap-3 border-b pb-6">
            <h3 className="flex items-center gap-2 font-medium text-neutral-600 text-sm dark:text-neutral-400">
              <Palette size={15} /> Appearance
            </h3>
            <fieldset className="mb-3">
              <legend className="mb-3 text-sm">Theme</legend>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant={theme === 'light' ? 'secondary' : 'outline'}
                  size="sm"
                  aria-pressed={theme === 'light'}
                  onClick={() => {
                    if (theme !== 'light') onToggleTheme();
                  }}
                >
                  <Sun size={16} /> Light
                </Button>
                <Button
                  type="button"
                  variant={theme === 'dark' ? 'secondary' : 'outline'}
                  size="sm"
                  aria-pressed={theme === 'dark'}
                  onClick={() => {
                    if (theme !== 'dark') onToggleTheme();
                  }}
                >
                  <Moon size={16} /> Dark
                </Button>
              </div>
            </fieldset>
            <ColorPicker colorName={colorName} onColorChange={onColorChange} />
          </section>
          {open && <SavedLogins />}
        </div>
      </DialogContent>
    </Dialog>
  );
}
