import download from '../assets/download.svg';
import styles from './DropZone.module.css';

export default function DropZone({ show, onDrop, onLeave }) {
  if (!show) {
    return null;
  }

  return (
    <div className={styles.dropzone} onDragLeave={onLeave}>
      <img src={download} alt="" />
      <textarea onChange={e => onDrop(e.target.value)}></textarea>
    </div>
  );
}
