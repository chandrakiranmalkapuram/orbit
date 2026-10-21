import { useCallback, useEffect, useMemo, useState } from 'react';
import { AnimatePresence, motion, useReducedMotion } from 'motion/react';
import { open } from '@tauri-apps/plugin-dialog';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { OrbitLogo } from './components/OrbitLogo';
import { CATEGORY_LABELS } from './lib/actions';
import type {
  ActionCategory,
  ActionDefinition,
  ActionExecutionRequest,
  SelectionMetadata,
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
  const [selection, setSelection] = useState<SelectionMetadata | null>(null);
  const [stage, setStage] = useState<Stage>('idle');
  const [dragging, setDragging] = useState(false);
  const [message, setMessage] = useState('Drop files here');
  const [result, setResult] = useState<OperationResult | null>(null);
  const [error, setError] = useState('');
  const [dark, setDark] = useState(true);
  const [alwaysOnTop, setAlwaysOnTop] = useState(true);
  const [activeCategory, setActiveCategory] = useState<ActionCategory | 'all'>('all');
  const [selectedParamAction, setSelectedParamAction] = useState<ActionDefinition | null>(null);
  const [qualityParam, setQualityParam] = useState(75);
  const [scaleParam, setScaleParam] = useState(50);
  const [showAllDrawer, setShowAllDrawer] = useState(false);
  const [zipContents, setZipContents] = useState<string[] | null>(null);

  const inspect = useCallback(async (paths: string[]) => {
    if (!paths || paths.length === 0) return;

    try {
      setStage('working');
      setError('');
      setResult(null);
      setZipContents(null);
      setSelectedParamAction(null);
      setShowAllDrawer(false);
      setActiveCategory('all');
      setMessage(paths.length === 1 ? 'Inspecting file…' : `Inspecting ${paths.length} files…`);

      const data = await invoke<SelectionMetadata>('inspect_selection', { paths });
      setSelection(data);
      setStage('ready');
      setMessage(data.summaryLabel);
    } catch (e) {
      setSelection(null);
      setStage('error');
      setError(String(e));
      setMessage('Could not inspect selection');
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
            void inspect(paths);
          }
        }
      })
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => {
        // Fallback for non-webview environments
      });

    return () => {
      unlisten?.();
    };
  }, [inspect]);

  const chooseFiles = async () => {
    try {
      const selected = await open({
        multiple: true,
        directory: false,
        title: 'Choose files for ORBIT',
      });
      if (Array.isArray(selected) && selected.length > 0) {
        void inspect(selected);
      } else if (typeof selected === 'string') {
        void inspect([selected]);
      }
    } catch (e) {
      setError(String(e));
    }
  };

  const handleActionClick = (action: ActionDefinition) => {
    if (!selection) return;

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
    if (!selection || selection.files.length === 0) return;

    try {
      setStage('working');
      setError('');
      setResult(null);
      setMessage(action.label);
      setSelectedParamAction(null);
      setShowAllDrawer(false);

      if (action.id === 'preview-zip') {
        const firstPath = selection.files[0].path;
        const entries = await invoke<string[]>('list_zip_entries', { path: firstPath });
        setZipContents(entries);
        setStage('ready');
        setMessage('Archive contents');
        return;
      }

      if (action.id === 'copy-path') {
        const pathsText = selection.files.map((f) => f.path).join('\n');
        await navigator.clipboard.writeText(pathsText);
        setResult({
          outputPaths: selection.files.map((f) => f.path),
          message:
            selection.files.length === 1
              ? 'Copied path to clipboard.'
              : `Copied ${selection.files.length} paths to clipboard.`,
        });
        setStage('done');
        return;
      }

      const req: ActionExecutionRequest = {
        paths: selection.files.map((f) => f.path),
        path: selection.files[0]?.path,
        actionId: action.id,
        quality:
          params?.quality ?? (action.parameter?.type === 'quality' ? qualityParam : undefined),
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
    setSelection(null);
    setStage('idle');
    setResult(null);
    setError('');
    setZipContents(null);
    setSelectedParamAction(null);
    setShowAllDrawer(false);
    setActiveCategory('all');
    setMessage('Drop files here');
  };

  const availableActions = useMemo(() => selection?.actions ?? [], [selection]);

  const categories = useMemo(() => {
    const cats = new Set<ActionCategory>();
    availableActions.forEach((a) => cats.add(a.category));
    return Array.from(cats);
  }, [availableActions]);

  // Layout math: primary radial actions
  const displayedActions = useMemo(() => {
    if (activeCategory === 'all') {
      // Show up to 6 primary actions cleanly on the circle to guarantee zero overlap
      const primaries = availableActions.filter((a) => a.isPrimary);
      if (primaries.length > 0 && primaries.length <= 6) {
        return primaries;
      }
      return availableActions.slice(0, 6);
    }
    return availableActions.filter((a) => a.category === activeCategory).slice(0, 8);
  }, [availableActions, activeCategory]);

  return (
    <main className={dark ? 'app dark' : 'app light'}>
      {/* Movable Top Capsule */}
      <header className="floating-capsule" data-tauri-drag-region>
        <div className="drag-label" data-tauri-drag-region>
          <OrbitLogo size={16} />
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

      {/* Ambient Radial Atmosphere */}
      <div
        className={`ambient-glow ambient-center ${dragging ? 'is-dragging' : ''}`}
        aria-hidden="true"
      />

      {/* Upper Orbital Stage */}
      <section className="orbit-stage" aria-label="ORBIT floating interface">
        {/* Radial Action Buttons */}
        <AnimatePresence>
          {stage === 'ready' &&
            displayedActions.map((action, i) => {
              const total = displayedActions.length;
              const angle = (-90 + (360 / total) * i) * (Math.PI / 180);
              const radius = 175;
              const left = `calc(50% + ${Math.cos(angle) * radius}px)`;
              const top = `calc(50% + ${Math.sin(angle) * radius}px)`;

              return (
                <motion.button
                  key={action.id}
                  className={`radial-action ${action.isPrimary ? 'primary' : ''}`}
                  style={{ left, top }}
                  initial={{ opacity: 0, scale: 0.5 }}
                  animate={{ opacity: 1, scale: 1 }}
                  exit={{ opacity: 0, scale: 0.6 }}
                  transition={{
                    duration: reduceMotion ? 0 : 0.25,
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

        {/* Central Glowing Orb */}
        <motion.button
          className={`orb-core ${stage} ${dragging ? 'is-dragging' : ''} ${stage === 'working' ? 'is-working' : ''} ${stage === 'done' ? 'is-done' : ''} ${stage === 'error' ? 'is-error' : ''}`}
          onClick={() => {
            if (stage === 'idle' || stage === 'error') void chooseFiles();
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
            const droppedPaths: string[] = [];
            for (let i = 0; i < e.dataTransfer.files.length; i++) {
              const f = e.dataTransfer.files[i];
              if (f && 'path' in f) {
                droppedPaths.push(String((f as File & { path: string }).path));
              }
            }
            if (droppedPaths.length > 0) {
              void inspect(droppedPaths);
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
            ) : selection ? (
              <>
                <span className="orb-glyph">
                  {selection.isMulti
                    ? '▤'
                    : selection.files[0]?.kind === 'pdf'
                      ? '▤'
                      : selection.files[0]?.kind === 'archive'
                        ? '▣'
                        : selection.files[0]?.kind === 'image'
                          ? '◈'
                          : '◉'}
                </span>
                <span className="orb-subtext">
                  {selection.isMulti
                    ? `${selection.files.length} FILES`
                    : selection.files[0]?.extension}
                </span>
              </>
            ) : (
              <>
                <OrbitLogo size={52} className="orb-central-logo" />
                <span className="orb-subtext">Drop Files</span>
              </>
            )}
          </div>
        </motion.button>

        {/* Parameter Popover - Positioned directly over the central orb */}
        <AnimatePresence>
          {selectedParamAction && (
            <motion.div
              className="parameter-popover"
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
              exit={{ opacity: 0, scale: 0.9 }}
            >
              <div className="popover-header">
                <span>{selectedParamAction.label}</span>
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
                    <span style={{ fontSize: '10px', color: 'var(--muted)' }}>Quality:</span>
                    <input
                      type="range"
                      className="param-slider"
                      min={selectedParamAction.parameter.min}
                      max={selectedParamAction.parameter.max}
                      value={qualityParam}
                      onChange={(e) => setQualityParam(Number(e.target.value))}
                    />
                    <strong style={{ fontSize: '11px', minWidth: '30px' }}>{qualityParam}%</strong>
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
                <span>Apply</span>
                <span>➔</span>
              </button>
            </motion.div>
          )}
        </AnimatePresence>
      </section>

      {/* Category Navigation Bar - Positioned cleanly BELOW the orbit */}
      {stage === 'ready' && categories.length > 1 && (
        <nav className="category-filter-bar" aria-label="Action categories">
          <button
            className={`category-chip ${activeCategory === 'all' && !showAllDrawer ? 'active' : ''}`}
            onClick={() => {
              setActiveCategory('all');
              setSelectedParamAction(null);
              setShowAllDrawer(false);
            }}
          >
            All ({availableActions.length})
          </button>
          {categories.map((cat) => (
            <button
              key={cat}
              className={`category-chip ${activeCategory === cat && !showAllDrawer ? 'active' : ''}`}
              onClick={() => {
                setActiveCategory(cat);
                setSelectedParamAction(null);
                setShowAllDrawer(false);
              }}
            >
              {CATEGORY_LABELS[cat]}
            </button>
          ))}
          {availableActions.length > 6 && (
            <button
              className={`category-chip ${showAllDrawer ? 'active' : ''}`}
              onClick={() => {
                setShowAllDrawer(!showAllDrawer);
                setSelectedParamAction(null);
              }}
              title="Browse all actions in palette"
            >
              More ▾
            </button>
          )}
        </nav>
      )}

      {/* Status & Information Area */}
      <div className="status-area" aria-live="polite">
        <motion.div
          key={message}
          initial={{ opacity: 0, y: 3 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: reduceMotion ? 0 : 0.2 }}
        >
          <h1 className="status-title">
            {stage === 'done'
              ? 'Complete'
              : stage === 'working'
                ? 'Processing…'
                : stage === 'error'
                  ? 'Error Occurred'
                  : selection
                    ? selection.summaryLabel
                    : 'Drop anything here'}
          </h1>
          <p className="status-subtitle">
            {stage === 'idle'
              ? 'Offline · Private · Native macOS file processor'
              : stage === 'ready'
                ? selection?.isMulti
                  ? `${selection.files.length} images · ${formatBytes(selection.totalSize)} · Drop order preserved`
                  : `${selection?.files[0]?.formatName || selection?.files[0]?.extension.toUpperCase()} · ${selection ? formatBytes(selection.totalSize) : ''}${selection?.files[0]?.width && selection?.files[0]?.height ? ` · ${selection.files[0].width}×${selection.files[0].height}` : ''} · Choose an action`
                : stage === 'working'
                  ? 'Converting locally on your Mac…'
                  : stage === 'done'
                    ? (result?.message ?? 'Saved beside original files.')
                    : error || 'Could not process this file.'}
          </p>
        </motion.div>

        {/* Action Buttons Row */}
        <div className="action-buttons-row">
          {stage === 'idle' && (
            <button className="pill-action-btn" onClick={() => void chooseFiles()}>
              <span>Choose Files</span>
              <span>↗</span>
            </button>
          )}

          {stage === 'ready' && selection?.isMulti && (
            <button
              className="pill-action-btn primary-highlight"
              onClick={() => {
                const combineAction = availableActions.find((a) => a.id === 'combine-pdf');
                if (combineAction) void runAction(combineAction);
              }}
            >
              <span>Combine into one PDF</span>
              <span>➔</span>
            </button>
          )}

          {stage === 'done' && result?.outputPaths[0] && (
            <>
              <button
                className="pill-action-btn primary-highlight"
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

      {/* Expandable Action Shelf / Palette */}
      <AnimatePresence>
        {showAllDrawer && (
          <motion.div
            className="all-actions-drawer"
            initial={{ opacity: 0, scale: 0.95 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.95 }}
          >
            <div className="drawer-header">
              <span className="drawer-title">
                Available Operations ({availableActions.length})
              </span>
              <button
                onClick={() => setShowAllDrawer(false)}
                style={{ fontSize: '16px', color: 'var(--muted)' }}
                aria-label="Close action palette"
              >
                ✕
              </button>
            </div>
            <div className="drawer-grid">
              {availableActions.map((action) => (
                <button
                  key={action.id}
                  className="drawer-card"
                  onClick={() => {
                    setShowAllDrawer(false);
                    handleActionClick(action);
                  }}
                >
                  <span className="drawer-card-icon">{action.symbol}</span>
                  <div className="drawer-card-info">
                    <span className="drawer-card-label">{action.label}</span>
                    <span className="drawer-card-detail">{action.detail}</span>
                  </div>
                </button>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>

      {/* ZIP Archive Content Preview Drawer */}
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
    </main>
  );
}
