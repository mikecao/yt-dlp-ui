import { open } from '@tauri-apps/plugin-shell';
import { ExternalLink, ShieldCheck } from 'lucide-react';
import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Input } from '@/components/ui/input';
import { type PremiumDownloadEvent, submitLoginVerification } from '@/lib/premium';

export default function LoginVerification({
  challenge,
  onSubmitted,
}: {
  challenge: Extract<PremiumDownloadEvent, { kind: 'verification' }>;
  onSubmitted: () => void;
}) {
  const [code, setCode] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState('');

  return (
    <form
      className="space-y-3 rounded-md border p-4"
      onSubmit={async event => {
        event.preventDefault();
        if (busy || !code.trim()) return;
        setBusy(true);
        setError('');
        try {
          await submitLoginVerification(challenge.downloadId, challenge.verificationId, code);
          setCode('');
          onSubmitted();
        } catch (error) {
          setError(String(error));
        } finally {
          setBusy(false);
        }
      }}
    >
      <div className="flex items-center gap-2 font-medium">
        <ShieldCheck size={16} />
        Verify login
      </div>
      <p className="text-muted-foreground text-sm">
        Complete the CAPTCHA in your browser, then paste the code shown there.
      </p>
      <Button
        type="button"
        variant="outline"
        onClick={async () => {
          try {
            await open(challenge.url);
          } catch {
            setError('Could not open the verification page. Please try again.');
          }
        }}
      >
        <ExternalLink size={14} />
        Open verification
      </Button>
      <div className="flex items-center gap-3">
        <Input
          aria-label="Verification code"
          placeholder="Verification code"
          value={code}
          onChange={event => setCode(event.target.value)}
          autoComplete="off"
          spellCheck={false}
          maxLength={16384}
          disabled={busy}
        />
        <Button type="submit" disabled={busy || !code.trim()}>
          {busy ? 'Continuing…' : 'Continue'}
        </Button>
      </div>
      {error && (
        <p role="alert" className="text-destructive text-sm">
          {error}
        </p>
      )}
    </form>
  );
}
