import { Command } from '@tauri-apps/api/shell';
import { useEffect, useState } from 'react';
import debug from 'debug';
import ProgressBar from './ProgressBar';
import styles from './Download.module.css';

const log = debug('download');

export default function Download({ url }) {
  const [state, setState] = useState({
    name: url,
    status: 'Loading',
    size: '',
    speed: '',
    progress: 0,
  });
  const { name, status, size, speed, progress } = state;

  useEffect(() => {
    async function spawn() {
      const child = await command.spawn();
      log(`pid: ${child.pid}`);
    }

    function stdout(line) {
      const name = line.match(/Destination:\s+(.*)/);
      if (name) {
        setState(state => ({ ...state, name: name[1] }));
      }

      const progress = line.match(/([\d\.]+)% of/);
      if (progress) {
        setState(state => ({ ...state, progress: progress[1] }));
      }

      const size = line.match(/([\d\.]+[KMG]iB)/);
      if (size) {
        setState(state => ({ ...state, size: size[1] }));
      }

      const speed = line.match(/at\s+([\d\.]+[\w\/]+)/);
      if (speed) {
        setState(state => ({ ...state, speed: speed[1], status: 'Downloading' }));
      }

      log(`stdout: ${line}`);
    }

    function stderr(line) {
      log(`stderr: ${line}`);
    }

    const command = Command.sidecar('binaries/yt-dlp', [url, '-o', 'd:/temp/%(title)s.%(ext)s']);

    command.on('close', data => {
      log({ close: data });
      setState(state => ({ ...state, status: data.code > 0 ? 'Error' : 'Complete' }));
    });

    command.on('error', error => {
      error({ error });
    });

    command.stdout.on('data', stdout);
    command.stderr.on('data', stderr);

    if (url) {
      spawn();
    }
  }, [url]);

  return (
    <tr>
      <td>
        <div className={styles.name}>{name}</div>
      </td>
      <td>{status}</td>
      <td>{speed}</td>
      <td>{size}</td>
      <td>
        <ProgressBar progress={progress} />
      </td>
    </tr>
  );
}
