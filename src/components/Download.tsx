import type { Child } from '@tauri-apps/plugin-shell';
import { Command, open as openShell } from '@tauri-apps/plugin-shell';
import debug from 'debug';
import { Terminal, X } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Progress } from '@/components/ui/progress';
import { TableCell, TableRow } from '@/components/ui/table';
import { CANCELLED, COMPLETE, DOWNLOADING, ERROR, LOADING } from '@/lib/constants';
import { findLoginForUrl, getLoginForUrl } from '@/lib/logins';
import {
  cancelPremiumDownload,
  formatBytes,
  premiumHostForUrl,
  startPremiumDownload,
} from '@/lib/premium';
import { buildYtDlpArgs } from '@/lib/settingsStore';
import type { Settings } from '@/lib/store';
import { cn } from '@/lib/utils';

const log = debug('ui:download');
log.log = console.log.bind(console);

function toOutputLines(lines: string[] = []) {
  return lines.map((text, id) => ({ id, text }));
}

function extractName(lines: string[] = []) {
  for (let i = lines.length - 1; i >= 0; i--) {
    const destination = lines[i].match(/Destination:\s+(.*)/);
    if (destination) {
      return destination[1];
    }
    const already = lines[i].match(/\[download\]\s+(.+?)\s+has already been downloaded/);
    if (already) {
      return already[1];
    }
  }
  return undefined;
}

export default function Download({
  url,
  initialName,
  initialStatus,
  initialOutput,
  settings,
  savePath,
  onChange,
  onRemove,
}: {
  url: string;
  initialName?: string;
  initialStatus?: string;
  initialOutput?: string[];
  settings: Settings;
  savePath: string;
  onChange: (value: string, output?: string[], name?: string) => void;
  onRemove: () => void;
}) {
  const savedName = initialName || extractName(initialOutput);
  const [state, setState] = useState({
    name: savedName || url,
    status: initialStatus || LOADING,
    size: '--',
    speed: '--',
    progress: initialStatus === COMPLETE ? '100' : '0',
  });
  const nameRef = useRef(savedName || url);
  const [output, setOutput] = useState(() => toOutputLines(initialOutput));
  const [expanded, setExpanded] = useState(false);
  const { name, status, size, speed, progress } = state;
  const path = savePath || '.';
  const pid = useRef(0);
  const childRef = useRef<Child | null>(null);
  const nativeIdRef = useRef<string | null>(null);
  const outputId = useRef(initialOutput?.length ?? 0);
  const outputLinesRef = useRef<string[]>(initialOutput ? [...initialOutput] : []);
  const outputRef = useRef<HTMLDivElement | null>(null);
  const cancelledRef = useRef(false);
  const secretsRef = useRef<string[]>([]);

  function redact(text: string) {
    return secretsRef.current.reduce((line, secret) => line.split(secret).join('[redacted]'), text);
  }

  function appendOutput(text: string) {
    outputLinesRef.current.push(text);
    setOutput(prev => [...prev, { id: outputId.current++, text }]);
  }

  function persistStatus(nextStatus: string) {
    onChange(nextStatus, outputLinesRef.current.slice(), nameRef.current);
  }

  function stdout(line: string) {
    line = redact(line);
    const name = line.match(/Destination:\s+(.*)/);
    if (name) {
      nameRef.current = name[1];
      setState(state => ({ ...state, name: name[1] }));
    }

    const already = line.match(/\[download\]\s+(.+?)\s+has already been downloaded/);
    if (already) {
      nameRef.current = already[1];
      setState(state => ({ ...state, name: already[1] }));
    }

    const progress = line.match(/([\d.]+)% of/);
    if (progress) {
      setState(state => ({ ...state, progress: progress[1] }));
    }

    const size = line.match(/([\d.]+[KMG]iB)/);
    if (size) {
      setState(state => ({ ...state, size: size[1] }));
    }

    const speed = line.match(/at\s+([\d.]+[\w/]+)/);
    if (speed) {
      setState(state => ({ ...state, speed: speed[1], status: DOWNLOADING }));
    }

    log(`stdout: ${line}`);
    if (!/^\[download\]\s+[\d.]+%/.test(line)) {
      appendOutput(line);
    }
  }

  function stderr(line: string) {
    line = redact(line);
    log(`stderr: ${line}`);
    appendOutput(line);
  }

  useEffect(() => {
    async function spawn() {
      const premium = premiumHostForUrl(url);
      if (premium) {
        const login = await findLoginForUrl(url);
        if (cancelledRef.current) return;
        if (!login)
          throw new Error(
            `Save a premium login for ${premium} in Settings → Saved Logins. Free downloads are not supported.`,
          );
        const id = await startPremiumDownload(
          url,
          login.domain,
          path,
          settings.rateLimit,
          event => {
            if (cancelledRef.current) return;
            if (event.kind === 'log') {
              appendOutput(event.message);
            } else if (event.kind === 'progress') {
              nameRef.current = event.name;
              setState(state => ({
                ...state,
                name: event.name,
                status: DOWNLOADING,
                progress: event.total
                  ? Math.min(100, (event.downloaded / event.total) * 100).toFixed(1)
                  : '0',
                size: formatBytes(event.total ?? event.downloaded),
                speed: `${formatBytes(event.speed)}/s`,
              }));
            } else if (event.kind === 'complete') {
              nativeIdRef.current = null;
              nameRef.current = event.name;
              setState(state => ({
                ...state,
                name: event.name,
                status: COMPLETE,
                progress: '100',
                speed: '--',
              }));
              persistStatus(COMPLETE);
            } else {
              nativeIdRef.current = null;
              stderr(event.message);
              setState(state => ({ ...state, status: ERROR, speed: '--' }));
              persistStatus(ERROR);
            }
          },
        );
        nativeIdRef.current = id;
        if (cancelledRef.current) await cancelPremiumDownload(id);
        return;
      }
      const args = buildYtDlpArgs(settings, url, path);
      const login = await getLoginForUrl(url);
      if (cancelledRef.current) return;
      if (login) {
        secretsRef.current = [
          login.password,
          login.username,
          JSON.stringify(login.password).slice(1, -1),
          encodeURIComponent(login.password),
        ]
          .filter(Boolean)
          .sort((a, b) => b.length - a.length);
        // Place credentials last so tab custom arguments cannot replace them.
        // A custom '--' separator would turn these into URLs instead of options.
        if (args.includes('--')) {
          throw new Error('Remove the -- separator from Custom Arguments to use a saved login.');
        }
        args.push('--username', login.username, '--password', login.password);
      }
      const command = Command.sidecar('binaries/yt-dlp', args);

      command.on('close', data => {
        if (cancelledRef.current) return;
        log({ close: data });
        const status = data?.code ? ERROR : COMPLETE;
        setState(state => ({ ...state, status }));
        persistStatus(status);
      });

      command.on('error', error => {
        if (cancelledRef.current) return;
        stderr(String(error));
        setState(state => ({ ...state, status: ERROR }));
        persistStatus(ERROR);
      });

      command.stdout.on('data', stdout);
      command.stderr.on('data', stderr);

      const child = await command.spawn();
      if (cancelledRef.current) {
        await child.kill();
        return;
      }
      log(`pid: ${child.pid}`);

      pid.current = child.pid;
      childRef.current = child;
    }

    if (url && !pid.current && initialStatus !== COMPLETE && initialStatus !== ERROR) {
      pid.current = 1;
      void spawn().catch(error => {
        if (cancelledRef.current) return;
        stderr(error instanceof Error ? error.message : String(error));
        setState(state => ({ ...state, status: ERROR }));
        persistStatus(ERROR);
      });
    }
  }, [url, initialStatus]);

  useEffect(() => {
    if (!expanded || !outputRef.current) {
      return;
    }

    outputRef.current.scrollTop = outputRef.current.scrollHeight;
  }, [expanded, output.length]);

  const handleRemove = async () => {
    const isInProgress = status === LOADING || status === DOWNLOADING;
    cancelledRef.current = true;
    if (isInProgress && nativeIdRef.current) {
      await cancelPremiumDownload(nativeIdRef.current);
      nativeIdRef.current = null;
    }
    if (isInProgress && childRef.current) {
      log('Cancelled manually');
      await childRef.current.kill();
      setState(state => ({ ...state, status: CANCELLED }));
      persistStatus(CANCELLED);
    }
    onRemove();
  };

  const handleOpenLink = async () => {
    try {
      await openShell(url);
    } catch (error) {
      log({ open: error });
    }
  };

  return (
    <>
      <TableRow>
        <TableCell className="truncate py-2">
          <button
            type="button"
            onClick={handleOpenLink}
            className="block w-full truncate text-left hover:text-primary hover:underline"
            title={name}
          >
            {name}
          </button>
        </TableCell>
        <TableCell className="py-2">
          <span>{status}</span>
        </TableCell>
        <TableCell className="py-2">
          {+progress > 0 ? (
            <div className="flex items-center gap-2.5">
              <Progress value={Number(progress)} className="w-[50px]" />
              <div>{progress}%</div>
            </div>
          ) : (
            '--'
          )}
        </TableCell>
        <TableCell className="py-2">{speed}</TableCell>
        <TableCell className="py-2">{size}</TableCell>
        <TableCell className="py-2">
          <div className="flex justify-end gap-1">
            {output.length > 0 && (
              <Button
                variant="ghost"
                size="icon"
                onClick={() => setExpanded(prev => !prev)}
                title="Toggle output"
                className={cn(
                  'h-6 w-6 text-neutral-500 dark:text-neutral-400',
                  expanded && 'text-neutral-700 dark:text-neutral-200',
                )}
              >
                <Terminal size={16} />
              </Button>
            )}
            <Button
              variant="ghost"
              size="icon"
              onClick={handleRemove}
              title={status === LOADING || status === DOWNLOADING ? 'Cancel download' : 'Remove'}
              className="h-6 w-6 text-neutral-500 hover:text-primary dark:text-neutral-400 dark:hover:text-primary"
            >
              <X size={16} />
            </Button>
          </div>
        </TableCell>
      </TableRow>
      {expanded && (
        <TableRow>
          <TableCell colSpan={6} className="p-0">
            <div
              ref={outputRef}
              className="max-h-[200px] overflow-y-auto whitespace-pre-wrap break-all rounded-b-md border-white/6 border-t bg-terminal p-3 font-mono text-terminal-text"
            >
              {output.map(line => (
                <div key={line.id} className="leading-[1.4]">
                  {line.text}
                </div>
              ))}
            </div>
          </TableCell>
        </TableRow>
      )}
    </>
  );
}
