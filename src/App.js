import { useState } from 'react';
import Button from "./components/Button";
import Search from './components/Search';
import styles from './App.module.css';

function App() {
  const [search, setSearch] = useState('');
  const [downloads, setDownloads] = useState([]);

  const handleStart = () => {

  }

  return (
    <div className={styles.app}>
      <div className={styles.header}>
        <Search value={search} onChange={setSearch} /><Button text="Start" onClick={handleStart} />
      </div>
      <div className={styles.view}>
        <h3>Downloads</h3>
        {downloads.map(() => <div>download</div>)}
      </div>
    </div>
  );
}

export default App;
