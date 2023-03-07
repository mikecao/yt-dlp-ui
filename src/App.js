import { useState } from 'react';
import md5 from 'md5';
import Search from './components/Search';
import Downloads from './components/Downloads';
import styles from './App.module.css';
import '@fontsource/inter/400.css';
import '@fontsource/inter/600.css';

function App() {
  const [downloads, setDownloads] = useState([]);

  const handleSubmit = value => {
    setDownloads(state =>
      state.concat({
        id: md5(Date.now()),
        url: value,
      }),
    );
  };

  return (
    <div className={styles.app}>
      <div className={styles.header}>
        <Search onSubmit={handleSubmit} />
      </div>
      <div className={styles.view}>
        <Downloads downloads={downloads} />
      </div>
    </div>
  );
}

export default App;
