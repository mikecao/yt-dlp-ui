import { Command } from '@tauri-apps/api/shell';
import { useEffect, useState } from 'react';
import classNames from 'classnames';
import debug from 'debug';
import ProgressBar from './ProgressBar';
import { DOWNLOADING, LOADING, COMPLETE, ERROR, SAVE_PATH } from '../constants';
import styles from './Download.module.css';

const log = debug('ui:download');
log.log = console.log.bind(console);

export default function Download({ url, onChange }) {
  const [state, setState] = useState({
    name: url,
    status: LOADING,
    size: '--',
    speed: '--',
    progress: 0,
  });
  const { name, status, size, speed, progress } = state;
  const path = localStorage.getItem(SAVE_PATH) || '.';

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
    }

    function stderr(line) {
      log(`stderr: ${line}`);
    }

    const command = Command.sidecar('binaries/yt-dlp', [
      url,
      '-o',
      `${path}/%(title)s.%(ext)s`,
      '--no-mtime',
      '--no-overwrites',
    ]);

    command.on('close', data => {
      log({ close: data });
      const status = data.code > 0 ? ERROR : COMPLETE;
      setState(state => ({ ...state, status }));
      onChange(status);
    });

    command.on('error', error => {
      log({ error });
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
      <td>
        <span
          className={classNames(styles.status, {
            [styles.error]: status === ERROR,
            [styles.complete]: status === COMPLETE,
          })}
        >
          {status}
        </span>
      </td>
      <td>
        <ProgressBar progress={progress} />
      </td>
      <td>{speed}</td>
      <td>{size}</td>
    </tr>
  );
}
