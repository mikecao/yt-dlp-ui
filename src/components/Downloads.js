import styles from './Downloads.module.css';
import Download from './Download';

export default function Downloads({ downloads, onChange }) {
  return (
    <div className={styles.downloads}>
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Status</th>
            <th>Progress</th>
            <th>Speed</th>
            <th>Size</th>
          </tr>
        </thead>
        <tbody>
          {downloads.map(({ id, url }) => {
            return <Download key={id} url={url} onChange={status => onChange(id, status)} />;
          })}
        </tbody>
      </table>
    </div>
  );
}
