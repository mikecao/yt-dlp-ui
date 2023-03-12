import { useState } from 'react';
import md5 from 'md5';
import produce from 'immer';
import Search from './components/Search';
import Downloads from './components/Downloads';
import Button from './components/Button';
import { COMPLETE, ERROR, SAVE_PATH } from './constants';
import styles from './App.module.css';
import SavePath from './components/SavePath';
import classNames from 'classnames';

function App() {
  const [downloads, setDownloads] = useState([]);
  const [savePath, setSavePath] = useState(localStorage.getItem(SAVE_PATH) || '');
  const [showDrop, setShowDrop] = useState(false);

  const handleSubmit = value => {
    setDownloads(state =>
      state.concat({
        id: md5(Date.now()),
        url: value,
      }),
    );
  };

  const handleChange = (id, status) => {
    setDownloads(state =>
      produce(state, draft => {
        const item = draft.find(download => download.id === id);
        if (item) {
          item.status = status;
        }
        return draft;
      }),
    );
  };

  const handleClear = () => {
    setDownloads(state =>
      state.filter(({ status }) => {
        console.log({ status });
        return status !== ERROR && status !== COMPLETE;
      }),
    );
  };

  const handleSavePath = value => {
    localStorage.setItem(SAVE_PATH, value);
    setSavePath(value);
  };

  const handleDrop = e => {
    setShowDrop(false);
    handleSubmit(e.target.value);
  };

  const handleEnter = e => {
    setShowDrop(true);
  };

  const handleLeave = e => {
    setShowDrop(false);
  };

  return (
    <div className={styles.app} onDragEnter={handleEnter}>
      <div
        className={classNames(styles.drop, { [styles.show]: showDrop })}
        onDragLeave={handleLeave}
      >
        <textarea key={showDrop} onChange={handleDrop}></textarea>
      </div>
      <div className={styles.header}>
        <Search onSubmit={handleSubmit} />
      </div>
      <div className={styles.view}>
        <Downloads downloads={downloads} onChange={handleChange} />
      </div>
      <div className={styles.footer}>
        <SavePath path={savePath} onChange={handleSavePath} />
        <Button onClick={handleClear}>Clear completed</Button>
      </div>
    </div>
  );
}

export default App;
