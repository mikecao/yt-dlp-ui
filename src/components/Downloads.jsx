import Download from './Download';
import styles from './Downloads.module.css';

export default function Downloads({ downloads, onChange }) {
  return (
    <div className={styles.downloads}>
      <div className={styles.header}>
        <div>Name</div>
        <div>Status</div>
        <div>Progress</div>
        <div>Speed</div>
        <div>Size</div>
      </div>
      <div className={styles.body}>
        {downloads.map(({ id, url }) => {
          return <Download key={id} url={url} onChange={status => onChange(id, status)} />;
        })}
      </div>
    </div>
  );
}
