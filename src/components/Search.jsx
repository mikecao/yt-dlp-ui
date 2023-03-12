import { useState } from 'react';
import Button from './Button';
import styles from './Search.module.css';

export default function Search({ onSubmit }) {
  const [value, setValue] = useState('');

  const handleSubmit = () => {
    if (value) {
      onSubmit(value);
      setValue('');
    }
  };

  const handleKey = e => {
    if (e.key === 'Enter') {
      onSubmit(value);
      setValue('');
    }
  };

  return (
    <div className={styles.search}>
      <input
        placeholder="Enter URL"
        value={value}
        onChange={e => setValue(e.target.value)}
        onKeyDown={handleKey}
      />
      <Button onClick={handleSubmit}>Download</Button>
    </div>
  );
}
