import { Channel, invoke } from '@tauri-apps/api/core';

export const PREMIUM_HOSTS = ['rapidgator.net', 'k2s.cc', 'tezfiles.com'] as const;

export function premiumHost(hostname: string): string | null {
  const host = hostname
    .toLowerCase()
    .replace(/\.$/, '')
    .replace(/^www\./, '');
  if (host === 'keep2share.cc') return 'k2s.cc';
  return PREMIUM_HOSTS.find(domain => domain === host) ?? null;
}

export function premiumHostForUrl(input: string): string | null {
  try {
    const url = new URL(input);
    return ['http:', 'https:'].includes(url.protocol) ? premiumHost(url.hostname) : null;
  } catch {
    return null;
  }
}

export type PremiumDownloadEvent =
  | { kind: 'verification'; downloadId: string; verificationId: string; url: string }
  | { kind: 'log'; message: string }
  | { kind: 'progress'; name: string; downloaded: number; total: number | null; speed: number }
  | { kind: 'complete'; name: string }
  | { kind: 'error'; message: string };

export async function startPremiumDownload(
  url: string,
  credentialDomain: string,
  savePath: string,
  rateLimit: string,
  onEvent: (event: PremiumDownloadEvent) => void,
): Promise<string> {
  const events = new Channel<PremiumDownloadEvent>();
  events.onmessage = onEvent;
  return invoke<string>('start_premium_download', {
    url,
    credentialDomain,
    savePath,
    rateLimit,
    events,
  });
}

export function cancelPremiumDownload(id: string): Promise<void> {
  return invoke('cancel_premium_download', { id });
}

export function submitLoginVerification(
  id: string,
  verificationId: string,
  code: string,
): Promise<void> {
  return invoke('submit_login_verification', { id, verificationId, code });
}

export function formatBytes(bytes: number): string {
  const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB'];
  const index =
    bytes > 0 ? Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1) : 0;
  return `${(bytes / 1024 ** index).toFixed(index ? 1 : 0)} ${units[index]}`;
}
