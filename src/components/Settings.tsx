import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { type Settings as SettingsType, useTabsStore } from '@/lib/store';

const audioFormats = ['mp3', 'aac', 'flac', 'wav', 'opus', 'm4a', 'vorbis'];
const noBrowserValue = '__none__';
const browsers = ['', 'chrome', 'firefox', 'edge', 'safari', 'opera', 'brave'];
const sectionHeadingClass = 'font-medium text-sm text-neutral-600 dark:text-neutral-400';
const sectionGroupClass =
  'flex flex-col gap-4 border-l border-neutral-300 pl-4 dark:border-neutral-700';

export default function Settings({ tabId }: { tabId: string }) {
  const { tabs, updateSettings } = useTabsStore();
  const tab = tabs.find(t => t.id === tabId);
  const settings = tab?.settings;

  if (!settings) return null;

  const handleChange = (key: keyof SettingsType, value: string | boolean) => {
    updateSettings(tabId, { [key]: value });
  };

  return (
    <div className="relative min-h-0 flex-1">
      <div className="absolute inset-0 overflow-y-auto py-4">
        <div className="flex w-full max-w-2xl flex-col gap-6">
          <h2 className="font-bold text-lg">yt-dlp Settings</h2>

          <section className="flex flex-col gap-2">
            <h3 className={sectionHeadingClass}>Format Options</h3>

            <div className={sectionGroupClass}>
              <div className="flex flex-col gap-1">
                <Label htmlFor="format">Format Selection (-f)</Label>
                <Input
                  id="format"
                  value={settings.format}
                  onChange={e => handleChange('format', e.target.value)}
                  placeholder="e.g., bestvideo+bestaudio, best, 720p"
                />
                <span className="text-neutral-500 text-sm">
                  Specify video format. Leave empty for default (best quality).
                </span>
              </div>

              <div className="flex items-center gap-2">
                <Checkbox
                  id="extractAudio"
                  checked={settings.extractAudio}
                  onCheckedChange={checked => handleChange('extractAudio', checked === true)}
                />
                <Label htmlFor="extractAudio">Extract Audio Only (-x)</Label>
              </div>

              {settings.extractAudio && (
                <div className="flex flex-col gap-1">
                  <Label htmlFor="audioFormat">Audio Format (--audio-format)</Label>
                  <Select
                    value={settings.audioFormat}
                    onValueChange={value => {
                      if (typeof value === 'string' && value) {
                        handleChange('audioFormat', value);
                      }
                    }}
                  >
                    <SelectTrigger id="audioFormat">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      {audioFormats.map(format => (
                        <SelectItem key={format} value={format}>
                          {format.toUpperCase()}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                </div>
              )}
            </div>
          </section>

          <section className="flex flex-col gap-2">
            <h3 className={sectionHeadingClass}>Download Options</h3>

            <div className={sectionGroupClass}>
              <div className="flex flex-col gap-1">
                <Label htmlFor="outputTemplate">Output Template (-o)</Label>
                <Input
                  id="outputTemplate"
                  value={settings.outputTemplate}
                  onChange={e => handleChange('outputTemplate', e.target.value)}
                  placeholder="%(title)s.%(ext)s"
                />
                <span className="text-neutral-500 text-sm">
                  Filename template, saved under this tab's folder. Leave empty for
                  %(title)s.%(ext)s. Examples: %(uploader)s/%(title)s.%(ext)s or %(upload_date)s -
                  %(title)s.%(ext)s
                </span>
              </div>

              <div className="flex flex-col gap-1">
                <Label htmlFor="rateLimit">Rate Limit (--rate-limit)</Label>
                <Input
                  id="rateLimit"
                  value={settings.rateLimit}
                  onChange={e => handleChange('rateLimit', e.target.value)}
                  placeholder="e.g., 100K, 1M, 500K"
                />
                <span className="text-neutral-500 text-sm">
                  Limit download speed. Use K for KB/s, M for MB/s.
                </span>
              </div>

              <div className="flex flex-col gap-1">
                <Label htmlFor="concurrentFragments">Concurrent Fragments (-N)</Label>
                <Input
                  id="concurrentFragments"
                  inputMode="numeric"
                  value={settings.concurrentFragments}
                  onChange={e => handleChange('concurrentFragments', e.target.value)}
                  placeholder="e.g., 4"
                />
                <span className="text-neutral-500 text-sm">
                  Number of fragments to download in parallel.
                </span>
              </div>

              <div className="flex items-center gap-2">
                <Checkbox
                  id="writeSubs"
                  checked={settings.writeSubs}
                  onCheckedChange={checked => handleChange('writeSubs', checked === true)}
                />
                <Label htmlFor="writeSubs">Download Subtitles (--write-subs)</Label>
              </div>

              {settings.writeSubs && (
                <div className="flex flex-col gap-1">
                  <Label htmlFor="subLangs">Subtitle Languages (--sub-langs)</Label>
                  <Input
                    id="subLangs"
                    value={settings.subLangs}
                    onChange={e => handleChange('subLangs', e.target.value)}
                    placeholder="e.g., en, en,fr,es"
                  />
                  <span className="text-neutral-500">Comma-separated language codes.</span>
                </div>
              )}
            </div>
          </section>

          <section className="flex flex-col gap-2">
            <h3 className={sectionHeadingClass}>Other Options</h3>

            <div className={sectionGroupClass}>
              <div className="flex items-center gap-2">
                <Checkbox
                  id="restrictFilenames"
                  checked={settings.restrictFilenames}
                  onCheckedChange={checked => handleChange('restrictFilenames', checked === true)}
                />
                <Label htmlFor="restrictFilenames">Restrict Filenames (--restrict-filenames)</Label>
                <span className="text-neutral-500 text-sm">ASCII only, no spaces</span>
              </div>

              <div className="flex flex-col gap-1">
                <Label htmlFor="cookiesFromBrowser">
                  Cookies from Browser (--cookies-from-browser)
                </Label>
                <Select
                  value={settings.cookiesFromBrowser || noBrowserValue}
                  onValueChange={value => {
                    if (typeof value === 'string') {
                      handleChange('cookiesFromBrowser', value === noBrowserValue ? '' : value);
                    }
                  }}
                >
                  <SelectTrigger id="cookiesFromBrowser">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {browsers.map(browser => (
                      <SelectItem key={browser || noBrowserValue} value={browser || noBrowserValue}>
                        {browser ? browser.charAt(0).toUpperCase() + browser.slice(1) : 'None'}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
                <span className="text-neutral-500 text-sm">
                  Use cookies from browser for authenticated downloads.
                </span>
              </div>

              <div className="flex flex-col gap-1">
                <Label htmlFor="customArgs">Custom Arguments</Label>
                <Input
                  id="customArgs"
                  value={settings.customArgs}
                  onChange={e => handleChange('customArgs', e.target.value)}
                  placeholder="e.g., --write-info-json --embed-thumbnail"
                />
                <span className="text-neutral-500 text-sm">
                  Additional yt-dlp arguments. Quoted values are kept together.
                </span>
              </div>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
