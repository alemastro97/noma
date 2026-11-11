import { RotateCcw } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  AUTOMATION_DENIED,
  backupList,
  backupRestore,
  capturesRecent,
  capturesRetry,
  cliInfo,
  cliInstall,
  importApple,
  importAppleSources,
  importText,
  mcpInfo,
  mirrorGet,
  mirrorSet,
  pickFolder,
  statsGet,
  type AppleSource,
  type CaptureRow,
  type CliInfo,
  type McpInfo,
  type MirrorConfig,
  type Snapshot,
  type Stats,
  type TextFormat,
} from "../../api/data";
import { onItemsChanged } from "../../api/events";
import { dataCopy as t } from "../../copy";
import { useProjectsStore } from "../../stores/projects";
import base from "./Settings.module.css";
import styles from "./DataTools.module.css";
import { intlLocale } from "../../lib/dates";

const message = (err: unknown) => (err instanceof Error ? err.message : String(err));
const when = (iso: string) =>
  new Date(iso).toLocaleString(intlLocale(), { dateStyle: "medium", timeStyle: "short" });

/** Re-runs `load` whenever cards change. */
function useReload(load: () => void) {
  useEffect(() => {
    load();
    let unlisten: (() => void) | undefined;
    let alive = true;
    void onItemsChanged(() => load())
      .then((u) => (alive ? (unlisten = u) : u()))
      .catch(() => undefined);
    return () => {
      alive = false;
      unlisten?.();
    };
  }, [load]);
}

// ── Restore ─────────────────────────────────────────────────────────────

function RestoreSection() {
  const [snapshots, setSnapshots] = useState<Snapshot[]>([]);
  const [confirming, setConfirming] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void backupList()
      .then((s) => setSnapshots(s ?? []))
      .catch(() => undefined);
  }, []);

  const restore = (name: string) => {
    setBusy(true);
    setError(null);
    void backupRestore(name)
      // Everything changed underneath: start the window over.
      .then(() => window.location.reload())
      .catch((err) => {
        setError(t.restoreFailed(message(err)));
        setBusy(false);
        setConfirming(null);
      });
  };

  return (
    <section className={base.section} aria-labelledby="restore-title">
      <div>
        <h2 id="restore-title" className={base.h2}>
          {t.restoreTitle}
        </h2>
        <p className={base.lead}>{t.restoreLead}</p>
      </div>
      {error && <p className={base.status}>{error}</p>}
      {snapshots.length === 0 ? (
        <p className={base.status}>{t.noSnapshots}</p>
      ) : (
        <ul className={styles.list}>
          {snapshots.map((s) => (
            <li key={s.name} className={styles.listRow}>
              <span className={styles.rowMain}>
                <span className={styles.rowTitle}>{when(s.at)}</span>
                <span className={styles.rowMeta}>{t.snapshotSize(s.sizeBytes)}</span>
              </span>
              {confirming === s.name ? (
                <span className={styles.rowActions}>
                  <span className={styles.confirm}>{t.restoreConfirm}</span>
                  <button
                    type="button"
                    className={base.primary}
                    disabled={busy}
                    onClick={() => restore(s.name)}
                  >
                    {busy ? t.restoring : t.restoreYes}
                  </button>
                  <button
                    type="button"
                    className={base.ghost}
                    disabled={busy}
                    onClick={() => setConfirming(null)}
                  >
                    {t.cancel}
                  </button>
                </span>
              ) : (
                <button
                  type="button"
                  className={base.ghost}
                  disabled={busy}
                  onClick={() => setConfirming(s.name)}
                >
                  <RotateCcw size={14} strokeWidth={1.75} aria-hidden="true" />
                  {t.restore}
                </button>
              )}
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

// ── Import ──────────────────────────────────────────────────────────────

type ImportSource = "markdown" | "todoist" | AppleSource;

const isApple = (s: ImportSource): s is AppleSource => s === "notes" || s === "reminders";

function ImportSection() {
  const projects = useProjectsStore((s) => s.projects);
  const [source, setSource] = useState<ImportSource>("markdown");
  const [projectId, setProjectId] = useState("");
  const [names, setNames] = useState<string[] | null>(null);
  const [name, setName] = useState("");
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState<string | null>(null);
  const file = useRef<HTMLInputElement>(null);

  const fail = (err: unknown) => {
    const msg = message(err);
    setStatus(msg === AUTOMATION_DENIED ? t.automationDenied : t.importFailed(msg));
  };
  const run = (job: Promise<{ created: number; skipped: number }>) => {
    setBusy(true);
    setStatus(t.importing);
    void job
      .then((s) => setStatus(t.imported(s.created, s.skipped)))
      .catch(fail)
      .finally(() => setBusy(false));
  };
  const target = projectId || null;

  const onFile = (f: File | undefined) => {
    if (!f) return;
    const format: TextFormat =
      source === "todoist" ? "todoist" : /\.(md|markdown)$/i.test(f.name) ? "markdown" : "text";
    run(f.text().then((content) => importText(format, content, target)));
    if (file.current) file.current.value = "";
  };

  const loadNames = (s: AppleSource) => {
    setBusy(true);
    setStatus(null);
    void importAppleSources(s)
      .then((n) => {
        setNames(n ?? []);
        setName(n?.[0] ?? "");
      })
      .catch(fail)
      .finally(() => setBusy(false));
  };

  return (
    <section className={base.section} aria-labelledby="import-title">
      <div>
        <h2 id="import-title" className={base.h2}>
          {t.importTitle}
        </h2>
        <p className={base.lead}>{t.importLead}</p>
      </div>
      <div className={base.row}>
        <span className={base.label}>{t.importFrom}</span>
        <select
          className={base.select}
          value={source}
          onChange={(e) => {
            setSource(e.target.value as ImportSource);
            setNames(null);
            setStatus(null);
          }}
        >
          {(Object.keys(t.sources) as ImportSource[]).map((s) => (
            <option key={s} value={s}>
              {t.sources[s]}
            </option>
          ))}
        </select>
      </div>
      <div className={base.row}>
        <span className={base.label}>{t.importTo}</span>
        <select
          className={base.select}
          value={projectId}
          onChange={(e) => setProjectId(e.target.value)}
        >
          <option value="">{t.inbox}</option>
          {projects.map((p) => (
            <option key={p.id} value={p.id}>
              {p.name}
            </option>
          ))}
        </select>
      </div>
      <p className={base.lead}>{t.sourceHints[source]}</p>
      {isApple(source) ? (
        <div className={styles.field}>
          {names === null ? (
            <button
              type="button"
              className={base.ghost}
              disabled={busy}
              onClick={() => loadNames(source)}
            >
              {t.loadSources(source)}
            </button>
          ) : (
            <>
              <select
                className={base.select}
                aria-label={t.pickSource(source)}
                value={name}
                onChange={(e) => setName(e.target.value)}
              >
                {names.map((n) => (
                  <option key={n} value={n}>
                    {n}
                  </option>
                ))}
              </select>
              <button
                type="button"
                className={base.primary}
                disabled={busy || !name}
                onClick={() => run(importApple(source, name, target))}
              >
                {t.importNow}
              </button>
            </>
          )}
        </div>
      ) : (
        <div className={styles.field}>
          <input
            ref={file}
            type="file"
            className={styles.hidden}
            accept={source === "todoist" ? ".csv,text/csv" : ".md,.markdown,.txt,text/plain"}
            onChange={(e) => onFile(e.target.files?.[0])}
          />
          <button
            type="button"
            className={base.ghost}
            disabled={busy}
            onClick={() => file.current?.click()}
          >
            {t.chooseFile}
          </button>
        </div>
      )}
      {status && <p className={base.status}>{status}</p>}
    </section>
  );
}

// ── Folder mirror ───────────────────────────────────────────────────────

function MirrorToggle() {
  const [config, setConfig] = useState<MirrorConfig | null>(null);
  const [status, setStatus] = useState<string | null>(null);
  useEffect(() => {
    void mirrorGet()
      .then(setConfig)
      .catch(() => undefined);
  }, []);
  const enabled = config === null ? null : config.enabled ? "1" : "0";
  const folder = config?.folder ?? "";

  const update = (patch: { enabled?: boolean; folder?: string }) =>
    void mirrorSet(patch)
      .then((c) => {
        setConfig(c);
        setStatus(c.written ? t.exportedTo(c.written) : null);
      })
      .catch((err) => setStatus(message(err)));

  const setEnabled = (v: string) => update({ enabled: v === "1" });

  const choose = () =>
    void pickFolder(t.folderPrompt)
      .then((p) => {
        if (p) update({ folder: p });
      })
      .catch((err) => setStatus(message(err)));

  return (
    <div className={styles.block}>
      <label className={base.toggleRow}>
        <span className={base.toggleText}>
          <span className={base.toggleLabel}>{t.autoExport}</span>
          <span className={base.lead}>{t.autoExportHint}</span>
        </span>
        <input
          type="checkbox"
          role="switch"
          className={base.switch}
          checked={enabled === "1"}
          disabled={enabled === null}
          onChange={(e) => setEnabled(e.target.checked ? "1" : "0")}
        />
      </label>
      {enabled === "1" && (
        <div className={styles.field}>
          <span className={styles.path}>{folder || t.noFolder}</span>
          <button type="button" className={base.ghost} onClick={choose}>
            {t.chooseFolder}
          </button>
        </div>
      )}
      {status && enabled === "1" && <p className={base.status}>{status}</p>}
    </div>
  );
}

// ── Stats ───────────────────────────────────────────────────────────────

function StatsSection() {
  const [stats, setStats] = useState<Stats | null>(null);
  const load = useCallback(() => {
    void statsGet()
      .then(setStats)
      .catch(() => undefined);
  }, []);
  useReload(load);
  if (!stats) return null;
  const max = Math.max(1, ...stats.weeks.flatMap((w) => [w.captures, w.completed]));
  const captures = stats.weeks.reduce((n, w) => n + w.captures, 0);
  const completed = stats.weeks.reduce((n, w) => n + w.completed, 0);
  const day = (d: string) =>
    new Date(`${d}T12:00:00`).toLocaleDateString(intlLocale(), { day: "numeric", month: "short" });

  return (
    <section className={base.section} aria-labelledby="stats-title">
      <div>
        <h2 id="stats-title" className={base.h2}>
          {t.statsTitle}
        </h2>
        <p className={base.lead}>{t.statsLead}</p>
      </div>
      <div className={styles.chart} role="img" aria-label={t.chartLabel}>
        {stats.weeks.map((w) => (
          <div
            key={w.start}
            className={styles.week}
            title={t.weekOf(day(w.start), w.captures, w.completed)}
          >
            <div className={styles.bars}>
              <span className={styles.bar} style={{ height: `${(w.captures / max) * 100}%` }} />
              <span
                className={styles.bar}
                data-series="completed"
                style={{ height: `${(w.completed / max) * 100}%` }}
              />
            </div>
            <span className={styles.weekLabel}>{day(w.start)}</span>
          </div>
        ))}
      </div>
      <div className={styles.legend}>
        <span>
          <span className={styles.swatch} />
          {t.capturesLegend}
        </span>
        <span>
          <span className={styles.swatch} data-series="completed" />
          {t.completedLegend}
        </span>
      </div>
      <p className={base.status}>
        {t.totals(captures, completed)} · {t.avgLatency(stats.avgLatencyMs)}
      </p>
    </section>
  );
}

// ── Capture history ─────────────────────────────────────────────────────

const PAGE = 15;

function HistorySection() {
  const [rows, setRows] = useState<CaptureRow[] | null>(null);
  const [limit, setLimit] = useState(PAGE);
  const load = useCallback(() => {
    void capturesRecent(limit + 1)
      .then((r) => setRows(r ?? []))
      .catch(() => undefined);
  }, [limit]);
  useReload(load);
  if (!rows) return null;

  return (
    <section className={base.section} aria-labelledby="history-title">
      <div>
        <h2 id="history-title" className={base.h2}>
          {t.historyTitle}
        </h2>
        <p className={base.lead}>{t.historyLead}</p>
      </div>
      {rows.length === 0 ? (
        <p className={base.status}>{t.noCaptures}</p>
      ) : (
        <ul className={styles.list}>
          {rows.slice(0, limit).map((c) => (
            <li key={c.id} className={styles.listRow}>
              <span className={styles.rowMain}>
                <span className={styles.rowTitle} title={c.rawText}>
                  {c.rawText}
                </span>
                <span className={styles.rowMeta}>
                  {[
                    when(c.createdAt),
                    t.source[c.source] ?? c.source,
                    c.latencyMs !== null ? t.latency(c.latencyMs) : null,
                    c.status === "done"
                      ? c.itemTitle
                        ? t.becameCard(c.itemTitle)
                        : t.cardGone
                      : null,
                  ]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
              </span>
              <span className={styles.rowActions}>
                <span className={styles.pill} data-status={c.status}>
                  {t.status[c.status]}
                </span>
                {c.status === "failed" && (
                  <button
                    type="button"
                    className={base.ghost}
                    onClick={() => void capturesRetry(c.id).then(load)}
                  >
                    {t.retry}
                  </button>
                )}
              </span>
            </li>
          ))}
        </ul>
      )}
      {rows.length > limit && (
        <div className={base.dataActions}>
          <button type="button" className={base.ghost} onClick={() => setLimit(limit + PAGE * 2)}>
            {t.showMore}
          </button>
        </div>
      )}
    </section>
  );
}

// ── Connect ─────────────────────────────────────────────────────────────

function CopyButton({ text }: { text: string }) {
  const [done, setDone] = useState(false);
  return (
    <button
      type="button"
      className={base.ghost}
      onClick={() =>
        void navigator.clipboard
          .writeText(text)
          .then(() => {
            setDone(true);
            setTimeout(() => setDone(false), 1500);
          })
          .catch(() => undefined)
      }
    >
      {done ? t.copied : t.copy}
    </button>
  );
}

function ConnectSection() {
  const [cli, setCli] = useState<CliInfo | null>(null);
  const [mcp, setMcp] = useState<McpInfo | null>(null);
  const [installed, setInstalled] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void cliInfo()
      .then(setCli)
      .catch(() => undefined);
    void mcpInfo()
      .then(setMcp)
      .catch(() => undefined);
  }, []);

  const snippet = mcp
    ? JSON.stringify({ mcpServers: { noma: { command: mcp.command } } }, null, 2)
    : "";

  return (
    <section className={base.section} aria-labelledby="connect-title">
      <div>
        <h2 id="connect-title" className={base.h2}>
          {t.connectTitle}
        </h2>
        <p className={base.lead}>{t.connectLead}</p>
      </div>

      <div className={styles.block}>
        <h3 className={styles.h3}>{t.urlTitle}</h3>
        <p className={base.lead}>{t.urlHint}</p>
        <ul className={styles.examples}>
          {t.urlExamples.map((e) => (
            <li key={e.url} className={styles.example}>
              <code>{e.url}</code>
              <span>{e.label}</span>
            </li>
          ))}
        </ul>
      </div>

      {cli && (
        <div className={styles.block}>
          <h3 className={styles.h3}>{t.cliTitle}</h3>
          <p className={base.lead}>{t.cliHint(cli.path)}</p>
          <pre className={styles.code}>{t.cliUsage}</pre>
          <div className={base.dataActions}>
            <button
              type="button"
              className={base.primary}
              onClick={() =>
                void cliInstall()
                  .then((info) => {
                    setCli(info);
                    setInstalled(true);
                    setError(null);
                  })
                  .catch((err) => setError(message(err)))
              }
            >
              {cli.installed ? t.cliReinstall : t.cliInstall}
            </button>
            <CopyButton text={cli.script} />
          </div>
          {installed && <p className={base.status}>{t.cliInstalled}</p>}
          {error && <p className={base.status}>{error}</p>}
          <details className={styles.details}>
            <summary>{t.showScript}</summary>
            <pre className={styles.code}>{cli.script}</pre>
          </details>
        </div>
      )}

      {mcp && (
        <div className={styles.block}>
          <h3 className={styles.h3}>{t.mcpTitle}</h3>
          <p className={base.lead}>{t.mcpHint}</p>
          <pre className={styles.code}>{snippet}</pre>
          <div className={base.dataActions}>
            <CopyButton text={snippet} />
          </div>
          {!mcp.exists && <p className={base.status}>{t.mcpMissing}</p>}
          <p className={base.lead}>{t.mcpCaptureNote}</p>
        </div>
      )}
    </section>
  );
}

/** Settings → Your data, continued: restore, import, mirror, stats, history, connect. */
export function DataToolsSection() {
  return (
    <>
      <section className={base.section} aria-label={t.autoExport}>
        <MirrorToggle />
      </section>
      <RestoreSection />
      <ImportSection />
      <StatsSection />
      <HistorySection />
      <ConnectSection />
    </>
  );
}
