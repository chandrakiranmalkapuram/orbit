import { useCallback, useEffect, useMemo, useState } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'motion/react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { CATEGORY_LABELS } from './lib/actions';
import type {
  ActionCategory,
  ActionDefinition,
  ActionExecutionRequest,
  FileMetadata,
  OperationResult,
} from './types/orbit';

type Stage = 'idle' | 'ready' | 'working' | 'done' | 'error';

function formatBytes(bytes: number) {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

export default function App() {
  const reduceMotion = useReducedMotion();
  const [file, setFile] = useState<FileMetadata | null>(null);
  const [stage, setStage] = useState<Stage>('idle');
  const [dragging, setDragging] = useState(false);
  const [message, setMessage] = useState('Drop file here');
  const [result, setResult] = useState<OperationResult | null>(null);
  const [error, setError] = useState('');
  const [dark, setDark] = useState(true);
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);
  const [activeCategory, setActiveCategory] = useState<ActionCategory | 'all'>('all');
  const [selectedParamAction, setSelectedParamAction] = useState<ActionDefinition | null>(null);
  const [qualityParam, setQualityParam] = useState(75);
  const [scaleParam, setScaleParam] = useState(50);
  const [zipContents, setZipContents] = useState<string[] | null>(null);

  const inspect = useCallback(async (path: string) => {
    try {
      setStage('working');
      setError('');
      setResult(null);
      setZipContents(null);
      setSelectedParamAction(null);
      setActiveCategory('all');
      setMessage('Inspecting file…');

      const detected = await invoke<FileMetadata>('inspect_file', { path });
      setFile(detected);
      setStage('ready');
      setMessage(detected.name);
    } catch (e) {
      setFile(null);
      setStage('error');
      setError(String(e));
      setMessage('Could not inspect file');
    }
  }, []);

  useEffect(() => {
    let unlisten: (() => void) | undefined;
    getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === 'enter' || event.payload.type === 'over') {
          setDragging(true);
        } else if (event.payload.type === 'leave') {
          setDragging(false);
        } else if (event.payload.type === 'drop') {
          setDragging(false);
          const paths = event.payload.paths;
          if (paths && paths.length > 0) {
            void inspect(paths[0]);
          }
        }
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // webview event listener fallback
      });

    return () => {
      unlisten?.();
    };
  }, [inspect]);

  const chooseFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        title: 'Choose a file for ORBIT',
      });
      if (typeof selected === 'string') {
        void inspect(selected);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const handleActionClick = (action: ActionDefinition) => {
    if (!file) return;

    // Check if this action requires parameter input first
    if (action.parameter && !selectedParamAction) {
      if (action.parameter.type === 'quality') {
        setQualityParam(action.parameter.default);
      } else if (action.parameter.type === 'scale') {
        setScaleParam(action.parameter.default);
      }
      setSelectedParamAction(action);
      return;
    }

    void runAction(action);
  };

  const runAction = async (
    action: ActionDefinition,
    params?: { quality?: number; scalePercent?: number }
  ) => {
    if (!file) return;

    try {
      setStage('working');
      setError('');
      setResult(null);
      setMessage(action.label);
      setSelectedParamAction(null);

      if (action.id === 'preview-zip') {
        const entries = await invoke<string[]>('list_zip_entries', { path: file.path });
        setZipContents(entries);
        setStage('ready');
        setMessage('Archive contents');
        return;
      }

      if (action.id === 'copy-path') {
        await navigator.clipboard.writeText(file.path);
        setResult({
          outputPaths: [file.path],
          message: 'Copied file path to clipboard.',
        });
        setStage('done');
        return;
      }

      const req: ActionExecutionRequest = {
        path: file.path,
        actionId: action.id,
        quality: params?.quality ?? (action.parameter?.type === 'quality' ? qualityParam : undefined),
        scalePercent:
          params?.scalePercent ??
          (action.parameter?.type === 'scale' ? scaleParam : undefined),
      };

      const response = await invoke<OperationResult>('execute_action', { req });
      setResult(response);
      setStage('done');
      setMessage('Done!');
    } catch (e) {
      setError(String(e));
      setStage('error');
      setMessage('Something went wrong');
    }
  };

  const togglePin = async () => {
    try {
      const next = !alwaysOnTop;
      await invoke('toggle_always_on_top', { enabled: next });
      setAlwaysOnTop(next);
    } catch (e) {
      console.error(e);
    }
  };

  const closeApp = async () => {
    try {
      await invoke('close_window');
    } catch (e) {
      console.error(e);
    }
  };

  const reset = () => {
    setFile(null);
    setStage('idle');
    setResult(null);
    setError('');
    setZipContents(null);
    setSelectedParamAction(null);
    setActiveCategory('all');
    setMessage('Drop file here');
  };

  // Actions available from Rust registry
  const availableActions = useMemo(() => file?.actions ?? [], [file]);

  // Categories present in the available actions
  const categories = useMemo(() => {
    const cats = new Set<ActionCategory>();
    availableActions.forEach((a) => cats.add(a.category));
    return Array.from(cats);
  }, [availableActions]);

  // Actions filtered by selected category
  const displayedActions = useMemo(() => {
    if (activeCategory === 'all') {
      // If there are more than 8 actions, show the primary actions on initial view
      if (availableActions.length > 8) {
        return availableActions.filter((a) => a.isPrimary);
      }
      return availableActions;
    }
    return availableActions.filter((a) => a.category === activeCategory);
  }, [availableActions, activeCategory]);

  return (
    <main className={dark ? 'app dark' : 'app light'}>
      {/* Movable Top Capsule (Finder-like glass pill) */}
      <header className="floating-capsule" data-tauri-drag-region>
        <div className="drag-label" data-tauri-drag-region>
          <span className="capsule-dot" />
          <span>ORBIT</span>
        </div>
        <div className="capsule-actions">
          <button
            className={`capsule-btn ${alwaysOnTop ? 'active' : ''}`}
            onClick={togglePin}
            title={alwaysOnTop ? 'Always on top: Enabled' : 'Always on top: Disabled'}
            aria-label="Toggle always on top"
          >
            📌
          </button>
          <button
            className="capsule-btn"
            onClick={() => setDark(!dark)}
            title="Toggle theme appearance"
            aria-label="Toggle theme appearance"
          >
            {dark ? '☼' : '☾'}
          </button>
          <button
            className="capsule-btn close"
            onClick={closeApp}
            title="Close ORBIT"
            aria-label="Close ORBIT"
          >
            ✕
          </button>
        </div>
      </header>

      {/* Ambient Radial Glowing Atmosphere */}
      <div
        className={`ambient-glow ambient-center ${dragging ? 'is-dragging' : ''}`}
        aria-hidden="true"
      />

      {/* Central Interactive Workspace */}
      <section className="orbit-workspace" aria-label="ORBIT floating interface">
        {/* Orbital Action Buttons */}
        <AnimatePresence>
          {stage === 'ready' &&
            displayedActions.map((action, i) => {
              const total = displayedActions.length;
              const angle = (-90 + (360 / total) * i) * (Math.PI / 180);
              const radius = total > 6 ? 205 : 180;
              const left = `calc(50% + ${Math.cos(angle) * radius}px)`;
              const top = `calc(50% + ${Math.sin(angle) * radius}px)`;

              return (
                <motion.button
                  key={action.id}
                  className="radial-action"
                  style={{ left, top }}
                  initial={{ opacity: 0, scale: 0.5 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.6 }}
                  transition={{
                    duration: reduceMotion ? 0 : 0.28,
                    delay: reduceMotion ? 0 : i * 0.03,
                  }}
                  onClick={() => handleActionClick(action)}
                  title={`${action.label} — ${action.detail}`}
                  aria-label={`${action.label}: ${action.detail}`}
                >
                  <span className="action-icon">{action.symbol}</span>
                  <span className="action-text">{action.label}</span>
                  <span className="action-category-badge">{action.category}</span>
                </motion.button>
              );
            })}
        </AnimatePresence>

        {/* Central Core Glowing Orb */}
        <motion.button
          className={`orb-core ${stage} ${dragging ? 'is-dragging' : ''} ${stage === 'working' ? 'is-working' : ''} ${stage === 'done' ? 'is-done' : ''} ${stage === 'error' ? 'is-error' : ''}`}
          onClick={() => {
            if (stage === 'idle' || stage === 'error') void chooseFile();
            else if (stage === 'done') reset();
          }}
          onDragOver={(e) => {
            e.preventDefault();
            setDragging(true);
          }}
          onDragLeave={() => setDragging(false)}
          onDrop={(e) => {
            e.preventDefault();
            setDragging(false);
            const dropped = e.dataTransfer.files[0];
            if (dropped && 'path' in dropped) {
              void inspect(String((dropped as File & { path: string }).path));
            }
          }}
          aria-label="ORBIT central drop target"
        >
          <span className="orb-halo" />
          <div className="orb-inner">
            {stage === 'working' ? (
              <motion.span
                className="orb-glyph"
                animate={reduceMotion ? {} : { rotate: 360 }}
                transition={{ repeat: Infinity, duration: 1.8, ease: 'linear' }}
              >
                ✳
              </motion.span>
            ) : stage === 'done' ? (
              <span className="orb-glyph done">✓</span>
            ) : stage === 'error' ? (
              <span className="orb-glyph error">↻</span>
            ) : file ? (
              <>
                <span className="orb-glyph">
                  {file.kind === 'archive'
                    ? '▣'
                    : file.kind === 'image'
                      ? '◈'
                      : file.kind === 'pdf'
                        ? '▤'
                        : '◉'}
                </span>
                <span className="orb-subtext">{file.extension}</span>
              </>
            ) : (
              <>
                <span className="orb-glyph">+</span>
                <span className="orb-subtext">Drop File</span>
              </>
            )}
          </div>
        </motion.button>

        {/* Contextual Parameter Popover (When action has parameters) */}
        <AnimatePresence>
          {selectedParamAction && (
            <motion.div
              className="parameter-popover"
              initial={{ opacity: 0, y: 10, scale: 0.95 }}
              animate={{ opacity: 1, y: 0, scale: 1 }}
              exit={{ opacity: 0, y: 10, scale: 0.95 }}
            >
              <div className="popover-header">
                <span>{selectedParamAction.label} Options</span>
                <span
                  className="popover-close"
                  onClick={() => setSelectedParamAction(null)}
                  role="button"
                  tabIndex={0}
                >
                  ✕
                </span>
              </div>

              {selectedParamAction.parameter?.type === 'quality' && (
                <>
                  <div className="popover-slider-row">
                    <span style={{ fontSize: '11px', color: 'var(--muted)' }}>Quality:</span>
                    <input
                      type="range"
                      className="param-slider"
                      min={selectedParamAction.parameter.min}
                      max={selectedParamAction.parameter.max}
                      value={qualityParam}
                      onChange={(e) => setQualityParam(Number(e.target.value))}
                    />
                    <strong style={{ fontSize: '12px', minWidth: '35px' }}>{qualityParam}%</strong>
                  </div>
                  <div className="chips-row">
                    {[60, 75, 85, 95].map((q) => (
                      <button
                        key={q}
                        className={`param-chip ${qualityParam === q ? 'selected' : ''}`}
                        onClick={() => setQualityParam(q)}
                      >
                        {q}%
                      </button>
                    ))}
                  </div>
                </>
              )}

              {selectedParamAction.parameter?.type === 'scale' && (
                <div className="chips-row">
                  {selectedParamAction.parameter.presets.map((s) => (
                    <button
                      key={s}
                      className={`param-chip ${scaleParam === s ? 'selected' : ''}`}
                      onClick={() => setScaleParam(s)}
                    >
                      {s}%
                    </button>
                  ))}
                </div>
              )}

              <button
                className="popover-run-btn"
                onClick={() =>
                  void runAction(selectedParamAction, {
                    quality: qualityParam,
                    scalePercent: scaleParam,
                  })
                }
              >
                <span>Execute</span>
                <span>➔</span>
              </button>
            </motion.div>
          )}
        </AnimatePresence>

        {/* Category Filter Capsules Bar (When file is loaded and has actions) */}
        {stage === 'ready' && categories.length > 1 && (
          <nav className="category-filter-bar" aria-label="Action categories">
            <button
              className={`category-chip ${activeCategory === 'all' ? 'active' : ''}`}
              onClick={() => {
                setActiveCategory('all');
                setSelectedParamAction(null);
              }}
            >
              All ({availableActions.length})
            </button>
            {categories.map((cat) => (
              <button
                key={cat}
                className={`category-chip ${activeCategory === cat ? 'active' : ''}`}
                onClick={() => {
                  setActiveCategory(cat);
                  setSelectedParamAction(null);
                }}
              >
                {CATEGORY_LABELS[cat]}
              </button>
            ))}
          </nav>
        )}

        {/* Status / File Information Area */}
        <div className="status-area" aria-live="polite">
          <motion.div
            key={message}
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: reduceMotion ? 0 : 0.2 }}
          >
            <h1 className="status-title">
              {stage === 'done'
                ? 'Completed'
                : stage === 'working'
                  ? 'Processing…'
                  : stage === 'error'
                    ? 'Error Occurred'
                    : file
                      ? file.name
                      : 'Drop anything here'}
            </h1>
            <p className="status-subtitle">
              {stage === 'idle'
                ? 'Offline · Private · Native macOS file processor'
                : stage === 'ready'
                  ? `${file?.formatName || file?.extension.toUpperCase()} · ${file ? formatBytes(file.size) : ''}${file?.width && file?.height ? ` · ${file.width}×${file.height}` : ''} · Choose an action`
                  : stage === 'working'
                    ? 'Converting strictly on this Mac…'
                    : stage === 'done'
                      ? (result?.message ?? 'Saved beside original file.')
                      : error || 'Could not process this file.'}
            </p>
          </motion.div>

          {/* Contextual Actions in status row */}
          <div className="action-buttons-row">
            {stage === 'idle' && (
              <button className="pill-action-btn" onClick={() => void chooseFile()}>
                <span>Choose File</span>
                <span>↗</span>
              </button>
            )}

            {stage === 'done' && result?.outputPaths[0] && (
              <>
                <button
                  className="pill-action-btn"
                  onClick={() => void invoke('reveal_in_finder', { path: result.outputPaths[0] })}
                >
                  <span>Reveal in Finder</span>
                  <span>↗</span>
                </button>
                <button className="pill-action-btn" onClick={reset}>
                  <span>Process Another</span>
                  <span>↻</span>
                </button>
              </>
            )}

            {stage === 'ready' && (
              <button className="pill-action-btn" onClick={reset}>
                <span>Reset</span>
                <span>✕</span>
              </button>
            )}
          </div>

          {stage === 'error' && <p className="status-error">{error}</p>}
        </div>

        {/* ZIP Contents Preview Drawer */}
        <AnimatePresence>
          {zipContents && (
            <motion.div
              className="zip-drawer"
              initial={{ opacity: 0, scale: 0.95 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.95 }}
            >
              <div className="zip-header">
                <span className="zip-title">Archive Contents ({zipContents.length} items)</span>
                <button
                  onClick={() => setZipContents(null)}
                  style={{ fontSize: '18px', color: 'var(--muted)' }}
                  aria-label="Close archive preview"
                >
                  ✕
                </button>
              </div>
              <div className="zip-list">
                {zipContents.slice(0, 150).map((name, i) => (
                  <div className="zip-item" key={`${name}-${i}`}>
                    <span>{name.endsWith('/') ? '📁' : '📄'}</span>
                    <span>{name}</span>
                  </div>
                ))}
                {zipContents.length > 150 && (
                  <div style={{ textAlign: 'center', padding: '8px', color: 'var(--accent)' }}>
                    …and {zipContents.length - 150} more items
                  </div>
                )}
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </section>
    </main>
  );
}
