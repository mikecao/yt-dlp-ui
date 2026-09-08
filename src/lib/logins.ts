import { invoke } from '@tauri-apps/api/core';
import { create } from 'zustand';
import { getDb } from './db';
import { premiumHost, premiumHostForUrl } from './premium';

export interface SavedLogin {
  domain: string;
  username: string;
  storageWarning?: string | null;
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
  await getDb();
  return invoke<SavedLogin[]>('list_saved_logins');
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
  const login = logins
    .filter(login => host === login.domain || host.endsWith(`.${login.domain}`))
    .sort((a, b) => b.domain.length - a.domain.length)[0];
  if (login) return login;

  // K2S aliases may share a root-domain login. Never borrow a login from a
  // sibling or child subdomain (e.g. www.example.com for example.com).
  return premium
    ? (logins.find(
        login => !login.domain.startsWith('www.') && premiumHost(login.domain) === premium,
      ) ?? null)
    : null;
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
      const saved = await invoke<SavedLogin>('save_login_password', {
        domain,
        username,
        password,
        editing,
      });
      set({
        logins: [...get().logins.filter(item => item.domain !== domain), saved].sort((a, b) =>
          a.domain.localeCompare(b.domain),
        ),
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
