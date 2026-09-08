import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';
import { getDb } from './db';
import { premiumHost, premiumHostForUrl } from './premium';

export interface SavedLogin {
  domain: string;
  username: string;
  includeSubdomains: boolean;
}

interface Credentials {
  username: string;
  password: string;
}

export function normalizeDomain(input: string): string {
  const value = input.trim();
  try {
    const url = new URL(value.includes('://') ? value : `https://${value}`);
    const domain = url.hostname.toLowerCase().replace(/\.$/, '');
    if (
      !value ||
      !domain ||
      !['http:', 'https:'].includes(url.protocol) ||
      url.username ||
      url.password ||
      url.port ||
      (!domain.includes('.') && domain !== 'localhost') ||
      domain.split('.').some(label => !/^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$/.test(label))
    ) {
      throw new Error();
    }
    return domain;
  } catch {
    throw new Error('Enter a valid domain or website URL, such as example.com.');
  }
}

export async function listLogins(): Promise<SavedLogin[]> {
  const db = await getDb();
  const rows = await db.select<{ domain: string; username: string; include_subdomains: number }[]>(
    'SELECT domain, username, include_subdomains FROM saved_logins ORDER BY domain',
  );
  return rows.map(row => ({
    domain: row.domain,
    username: row.username,
    includeSubdomains: Boolean(row.include_subdomains),
  }));
}

export async function findLoginForUrl(url: string): Promise<SavedLogin | null> {
  let host: string;
  const premium = premiumHostForUrl(url);
  try {
    const parsed = new URL(url);
    // Never send saved passwords over an unencrypted connection.
    if (parsed.protocol !== 'https:' && !premium) return null;
    host = parsed.hostname.toLowerCase().replace(/\.$/, '');
  } catch {
    return null;
  }
  const logins = await listLogins();
  if (premium) {
    // Host handlers explicitly recognize www and K2S's keep2share.cc alias.
    // TezFiles accounts always stay separate from K2S accounts.
    return (
      logins.find(login => login.domain === host) ??
      logins.find(login => login.domain === premium) ??
      logins.find(login => premiumHost(login.domain) === premium) ??
      null
    );
  }
  const login = logins
    .filter(
      login =>
        host === login.domain || (login.includeSubdomains && host.endsWith(`.${login.domain}`)),
    )
    .sort((a, b) => b.domain.length - a.domain.length)[0];
  return login ?? null;
}

export async function getLoginForUrl(url: string): Promise<Credentials | null> {
  const login = await findLoginForUrl(url);
  return login ? invoke<Credentials>('get_login_password', { domain: login.domain }) : null;
}

interface LoginsState {
  logins: SavedLogin[];
  loading: boolean;
  busy: boolean;
  error: string;
  load: () => Promise<void>;
  save: (login: SavedLogin, password: string, editing: boolean) => Promise<boolean>;
  remove: (domain: string) => Promise<boolean>;
}

const errorMessage = (error: unknown) => (error instanceof Error ? error.message : String(error));

export const useLoginsStore = create<LoginsState>()((set, get) => ({
  logins: [],
  loading: true,
  busy: false,
  error: '',
  load: async () => {
    set({ loading: true, error: '' });
    try {
      set({ logins: await listLogins() });
    } catch (error) {
      set({ error: errorMessage(error) });
    } finally {
      set({ loading: false });
    }
  },
  save: async (login, password, editing) => {
    if (get().busy) return false;
    set({ busy: true, error: '' });
    try {
      const domain = normalizeDomain(login.domain);
      const username = login.username.trim();
      if (!username) throw new Error('Enter a username.');
      const existing = (await listLogins()).find(item => item.domain === domain);
      if (existing && !editing)
        throw new Error('A login already exists for this domain. Use Edit to update it.');
      const previous = existing
        ? await invoke<Credentials>('get_login_password', { domain }).catch(() => null)
        : null;
      const nextPassword = password || previous?.password;
      if (!nextPassword) throw new Error('Enter a password.');
      await invoke('save_login_password', { domain, username, password: nextPassword });
      const db = await getDb();
      try {
        await db.execute(
          'INSERT INTO saved_logins (domain, username, include_subdomains) VALUES ($1, $2, $3) ON CONFLICT(domain) DO UPDATE SET username = $2, include_subdomains = $3',
          [domain, username, login.includeSubdomains ? 1 : 0],
        );
      } catch (error) {
        // Restore the old credential if the metadata write fails.
        if (previous) {
          await invoke('save_login_password', { domain, ...previous });
        } else if (!existing) {
          await invoke('delete_login_password', { domain });
        }
        throw error;
      }
      set({
        logins: [
          ...get().logins.filter(item => item.domain !== domain),
          { ...login, domain, username },
        ].sort((a, b) => a.domain.localeCompare(b.domain)),
      });
      return true;
    } catch (error) {
      set({ error: errorMessage(error) });
      return false;
    } finally {
      set({ busy: false });
    }
  },
  remove: async domain => {
    if (get().busy) return false;
    set({ busy: true, error: '' });
    try {
      await invoke('delete_login_password', { domain });
      const db = await getDb();
      await db.execute('DELETE FROM saved_logins WHERE domain = $1', [domain]);
      set({ logins: get().logins.filter(login => login.domain !== domain) });
      return true;
    } catch (error) {
      set({ error: errorMessage(error) });
      return false;
    } finally {
      set({ busy: false });
    }
  },
}));
