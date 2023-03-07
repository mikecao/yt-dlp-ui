import styles from './Downloads.module.css';
import Download from './Download';

export default function Downloads({ downloads }) {
  return (
    <div className={styles.downloads}>
      <table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Status</th>
            <th>Speed</th>
            <th>Size</th>
            <th>Progress</th>
          </tr>
        </thead>
        <tbody>
          {downloads.map(({ id, url }) => {
            return <Download key={id} url={url} />;
          })}
        </tbody>
      </table>
    </div>
  );
}
