import styles from './ProgressBar.module.css';

export default function ProgressBar({ progress }) {
  return (
    <div className={styles.container}>
      <div className={styles.bar}>
        <div className={styles.progress} style={{ width: `${progress}%` }} />
      </div>
      <div>{progress}%</div>
    </div>
  );
}
