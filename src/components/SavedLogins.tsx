import { KeyRound } from 'lucide-react';
import { useEffect, useId, useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import { type SavedLogin, useLoginsStore } from '@/lib/logins';
import { premiumHost } from '@/lib/premium';

const emptyLogin: SavedLogin = { domain: '', username: '' };

export default function SavedLogins() {
  const { logins, loading, busy, error, load, save, remove } = useLoginsStore();
  const [draft, setDraft] = useState<SavedLogin | null>(null);
  const [password, setPassword] = useState('');
  const [editing, setEditing] = useState(false);
  const [notice, setNotice] = useState('');
  const id = useId();

  useEffect(() => {
    void load();
  }, [load]);

  function closeForm() {
    setDraft(null);
    setPassword('');
    setEditing(false);
  }

  const loginForm = draft ? (
    <form
      className="flex flex-col gap-4 rounded border border-border bg-card p-4"
      onSubmit={async event => {
        event.preventDefault();
        if (await save(draft, password, editing)) {
          closeForm();
        }
      }}
    >
      <h4 className="font-medium text-sm">{editing ? 'Edit login' : 'Add login'}</h4>
      <div className="flex flex-col gap-1">
        <Label htmlFor={`${id}-domain`}>Domain</Label>
        <Input
          id={`${id}-domain`}
          value={draft.domain}
          required
          disabled={busy || editing}
          placeholder="example.com"
          autoCapitalize="none"
          autoCorrect="off"
          spellCheck={false}
          onChange={event => setDraft({ ...draft, domain: event.target.value })}
        />
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor={`${id}-username`}>Username or email</Label>
        <Input
          id={`${id}-username`}
          value={draft.username}
          required
          disabled={busy}
          autoComplete="username"
          autoCapitalize="none"
          spellCheck={false}
          onChange={event => setDraft({ ...draft, username: event.target.value })}
        />
      </div>
      <div className="flex flex-col gap-1">
        <Label htmlFor={`${id}-password`}>Password</Label>
        <Input
          id={`${id}-password`}
          type="password"
          value={password}
          required={!editing}
          disabled={busy}
          autoComplete="new-password"
          placeholder={editing ? 'Leave blank to keep the saved password' : 'Enter password'}
          onChange={event => setPassword(event.target.value)}
        />
      </div>
      <div className="flex gap-2">
        <Button type="submit" size="sm" disabled={busy}>
          {busy ? 'Saving…' : 'Save login'}
        </Button>
        <Button type="button" variant="outline" size="sm" disabled={busy} onClick={closeForm}>
          Cancel
        </Button>
      </div>
    </form>
  ) : null;

  return (
    <section className="flex flex-col gap-2">
      <div className="flex items-center justify-between gap-4">
        <h3 className="flex items-center gap-2 font-medium text-neutral-600 text-sm dark:text-neutral-400">
          <KeyRound size={15} /> Saved Logins
        </h3>
        {!draft && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={loading || busy}
            onClick={() => {
              setDraft({ ...emptyLogin });
              setEditing(false);
              setNotice('');
            }}
          >
            Add login
          </Button>
        )}
      </div>
      <div className="flex flex-col gap-4 border-neutral-300 border-l pl-4 dark:border-neutral-700">
        {loading ? (
          <p className="text-muted-foreground text-sm">Loading saved logins…</p>
        ) : logins.length === 0 ? (
          <p className="text-muted-foreground text-sm">No saved logins yet.</p>
        ) : (
          <ul className="divide-y divide-border border-border border-y">
            {logins.map(login =>
              editing && draft?.domain === login.domain ? (
                <li key={login.domain} className="py-3">
                  {loginForm}
                </li>
              ) : (
                <li key={login.domain} className="flex items-center justify-between gap-3 py-3">
                  <div className="min-w-0">
                    <div className="truncate font-medium text-sm" title={login.domain}>
                      {login.domain}
                    </div>
                    <div className="truncate text-muted-foreground text-sm" title={login.username}>
                      {login.username}
                      {premiumHost(login.domain) ? ' · Premium only' : ''}
                    </div>
                    {login.storageWarning && (
                      <p role="status" className="mt-1 text-destructive text-sm">
                        {login.storageWarning}
                      </p>
                    )}
                  </div>
                  <div className="flex shrink-0 gap-1">
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      disabled={busy || draft !== null}
                      aria-label={`Edit login for ${login.domain}`}
                      onClick={() => {
                        setDraft({ ...login });
                        setPassword('');
                        setEditing(true);
                        setNotice('');
                      }}
                    >
                      Edit
                    </Button>
                    <Button
                      type="button"
                      variant="ghost"
                      size="sm"
                      disabled={busy || draft !== null}
                      aria-label={`Remove login for ${login.domain}`}
                      onClick={async () => {
                        setNotice('');
                        if (await remove(login.domain))
                          setNotice(`Login removed for ${login.domain}.`);
                      }}
                    >
                      Remove
                    </Button>
                  </div>
                </li>
              ),
            )}
          </ul>
        )}
        {!editing && loginForm}
        {error && (
          <div role="alert" className="text-destructive text-sm">
            {error}
          </div>
        )}
        {!loading && error && !draft && (
          <Button
            type="button"
            variant="outline"
            size="sm"
            className="self-start"
            disabled={busy}
            onClick={() => void load()}
          >
            Reload logins
          </Button>
        )}
        {notice && (
          <p role="status" className="text-muted-foreground text-sm">
            {notice}
          </p>
        )}
      </div>
    </section>
  );
}
