import styles from './Search.module.css';

export default function Search({ value = '', onChange }) {
  const handleChange = e => {
    onChange(e.target.value);
  }

  return <input className={styles.search} value={value} onChange={handleChange} />
}
