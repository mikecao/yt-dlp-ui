import { open } from '@tauri-apps/api/dialog';
import styles from './SavePath.module.css';

export default function SavePath({ path, onChange }) {
  const handleOpen = async () => {
    const value = await open({ directory: true });

    onChange(value);
  };

  return (
    <div className={styles.container}>
      <div>Save to</div>
      <input type="text" value={path} onChange={e => onChange(e.target.value)} readOnly={true} />
      <button onClick={handleOpen}>...</button>
    </div>
  );
}
