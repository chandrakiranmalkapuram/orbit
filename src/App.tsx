import { useCallback, useEffect, useMemo, useState } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'motion/react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { ACTIONS } from './lib/actions';
import type { ActionId, DetectedFile, OperationResult } from './types/orbit';

 type Stage = 'idle' | 'ready' | 'working' | 'done' | 'error';

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function App() {
  const reduceMotion = useReducedMotion();
  const [file, setFile] = useState<DetectedFile | null>(null);
  const [stage, setStage] = useState<Stage>('idle');
  const [dragging, setDragging] = useState(false);
  const [message, setMessage] = useState('Drop anything here');
  const [result, setResult] = useState<OperationResult | null>(null);
  const [error, setError] = useState('');
  const [dark, setDark] = useState(true);
  const [zipContents, setZipContents] = useState<string[] | null>(null);

  const inspect = useCallback(async (path: string) => {
    try {
      setStage('working'); setError(''); setResult(null); setZipContents(null);
      const detected = await invoke<DetectedFile>('inspect_file', { path });
      setFile(detected); setStage('ready'); setMessage(detected.name);
    } catch (e) {
      setFile(null); setStage('error'); setError(String(e)); setMessage('Could not open that file');
    }
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === 'enter' || event.payload.type === 'over') setDragging(true);
      if (event.payload.type === 'leave') setDragging(false);
      if (event.payload.type === 'drop') {
        setDragging(false);
        const paths = event.payload.paths;
        if (paths.length > 0) void inspect(paths[0]);
      }
    }).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [inspect]);

  const chooseFile = async () => {
    const selected = await open({ multiple: false, directory: false, title: 'Choose a file for ORBIT' });
    if (typeof selected === 'string') void inspect(selected);
  };

  const doAction = async (action: ActionId) => {
    if (!file) return;
    try {
      setStage('working'); setError(''); setResult(null); setMessage(ACTIONS[action].label);
      if (action === 'preview-zip') {
        const entries = await invoke<string[]>('list_zip_contents', { path: file.path });
        setZipContents(entries); setStage('ready'); setMessage('Archive contents'); return;
      }
      let response: OperationResult;
      if (action === 'extract-zip') response = await invoke<OperationResult>('extract_zip', { path: file.path });
      else if (action === 'to-png' || action === 'to-jpeg' || action === 'to-webp' || action === 'compress-image') {
        const format = action === 'compress-image' ? 'webp' : action === 'to-png' ? 'png' : action === 'to-jpeg' ? 'jpeg' : 'webp';
        response = await invoke<OperationResult>('convert_image', { path: file.path, format, quality: 82 });
      } else if (action === 'to-pdf') {
        throw new Error('Image-to-PDF is planned for the next MVP increment and is not enabled yet.');
      } else {
        throw new Error('This action is not available in this build.');
      }
      setResult(response); setStage('done'); setMessage('Done!');
    } catch (e) {
      setError(String(e)); setStage('error'); setMessage('Something went wrong');
    }
  };

  const actions = useMemo(() => file?.actions.map((id) => ACTIONS[id]).filter(Boolean) ?? [], [file]);
  const reset = () => { setFile(null); setStage('idle'); setResult(null); setError(''); setZipContents(null); setMessage('Drop anything here'); };

  return <main className={dark ? 'app dark' : 'app light'}>
    <header className="topbar">
      <button className="brand" onClick={reset} aria-label="ORBIT home"><span className="brand-mark">◉</span><span>ORBIT</span></button>
      <div className="topbar-right"><span className="privacy"><span className="privacy-dot"/> Offline by design</span><button className="theme-toggle" onClick={() => setDark(!dark)} aria-label="Toggle appearance">{dark ? '☼' : '☾'}</button></div>
    </header>

    <section className="stage" aria-label="File conversion workspace">
      <div className="ambient ambient-one"/><div className="ambient ambient-two"/>
      <div className="orbit-layout">
        <AnimatePresence>
          {stage === 'ready' && actions.map((action, i) => {
            const angle = (-90 + (360 / actions.length) * i) * Math.PI / 180;
            const radius = actions.length > 5 ? 202 : 176;
            return <motion.button key={action.id} className="action-orbit" style={{ left: `calc(50% + ${Math.cos(angle) * radius}px)`, top: `calc(50% + ${Math.sin(angle) * radius}px)` }} initial={{ opacity: 0, scale: .55 }} animate={{ opacity: 1, scale: 1 }} exit={{ opacity: 0, scale: .65 }} transition={{ duration: reduceMotion ? 0 : .28, delay: reduceMotion ? 0 : i * .035 }} onClick={() => void doAction(action.id)} title={action.detail}>
              <span className="action-symbol">{action.symbol}</span><span className="action-label">{action.label}</span>
            </motion.button>;
          })}
        </AnimatePresence>
        <motion.button className={`core ${dragging ? 'is-dragging' : ''} ${stage === 'working' ? 'is-working' : ''} ${stage === 'done' ? 'is-done' : ''}`} onClick={() => { if (stage === 'idle' || stage === 'error') void chooseFile(); else if (stage === 'done') reset(); }} onDragOver={(e) => { e.preventDefault(); setDragging(true); }} onDragLeave={() => setDragging(false)} onDrop={(e) => { e.preventDefault(); setDragging(false); const f = e.dataTransfer.files[0]; if (f && 'path' in f) void inspect(String((f as File & {path: string}).path)); }} aria-label="Drop a file or choose one">
          <span className="core-halo"/><span className="core-inner">
            {stage === 'working' ? <motion.span className="spinner" animate={reduceMotion ? {} : { rotate: 360 }} transition={{ repeat: Infinity, duration: 2, ease: 'linear' }}>✳</motion.span> : stage === 'done' ? <span className="checkmark">✓</span> : stage === 'error' ? <span className="core-plus">↻</span> : file ? <span className="file-glyph">{file.kind === 'archive' ? '▣' : file.kind === 'image' ? '◈' : '▤'}</span> : <span className="core-plus">+</span>}
          </span>
        </motion.button>
      </div>
      <div className="prompt" aria-live="polite">
        <motion.h1 key={message} initial={{ opacity: 0, y: 5 }} animate={{ opacity: 1, y: 0 }} transition={{ duration: reduceMotion ? 0 : .2 }}>{stage === 'done' ? 'Done!' : stage === 'working' ? 'Working on it' : stage === 'error' ? 'Let’s try again' : file ? file.name : 'Drop anything here'}</motion.h1>
        <p>{stage === 'idle' ? 'Files stay on your Mac. Always.' : stage === 'ready' ? `${file?.extension.toUpperCase() || 'FILE'} · ${file ? formatBytes(file.size) : ''} · Choose an action` : stage === 'working' ? 'Processing locally…' : stage === 'done' ? (result?.message ?? 'Saved beside your original file.') : 'The file could not be processed.'}</p>
        {stage === 'idle' && <button className="choose-link" onClick={() => void chooseFile()}>or choose a file <span>↗</span></button>}
        {stage === 'done' && result?.outputPaths[0] && <button className="choose-link" onClick={() => void invoke('reveal_in_finder', { path: result.outputPaths[0] })}>Reveal in Finder <span>↗</span></button>}
        {stage === 'error' && <p className="error-detail">{error}</p>}
      </div>
      <AnimatePresence>{zipContents && <motion.div className="contents-panel" initial={{ opacity: 0, y: 8 }} animate={{ opacity: 1, y: 0 }} exit={{ opacity: 0 }}><div className="contents-heading"><strong>Archive contents</strong><button onClick={() => setZipContents(null)}>×</button></div><div className="contents-list">{zipContents.slice(0, 100).map((name, i) => <div className="contents-row" key={`${name}-${i}`}><span>{name.endsWith('/') ? '▰' : '▤'}</span><span>{name}</span></div>)}{zipContents.length > 100 && <p>And {zipContents.length - 100} more items…</p>}</div></motion.div>}</AnimatePresence>
      <div className="bottom-note"><span className="lock">⌑</span> No uploads. No accounts. No tracking.</div>
    </section>
    <footer className="footer"><span>ORBIT <span className="muted">v0.1 · Early build</span></span><span className="footer-right">Made for your Mac <span className="tiny-orbit">✳</span></span></footer>
  </main>;
}
