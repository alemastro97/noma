import { FileText, FolderSearch, Paperclip, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import * as cardsApi from "../../api/cards";
import type { Attachment } from "../../api/cards";
import { cardsCopy } from "../../copy";
import { toastError } from "../../stores/toasts";
import styles from "./Cards.module.css";

/** Thumbnails only for images up to this size. */
const THUMB_MAX_BYTES = 15 * 1024 * 1024;

function message(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function isImage(a: Attachment): boolean {
  return a.mime.startsWith("image/") && a.mime !== "image/heic" && a.size <= THUMB_MAX_BYTES;
}

function Thumb({ attachment }: { attachment: Attachment }) {
  const [url, setUrl] = useState<string | null>(null);
  useEffect(() => {
    if (!isImage(attachment)) return;
    let alive = true;
    let objectUrl: string | null = null;
    cardsApi
      .readAttachment(attachment.id)
      .then((buf) => {
        if (!alive) return;
        objectUrl = URL.createObjectURL(new Blob([buf], { type: attachment.mime }));
        setUrl(objectUrl);
      })
      .catch(() => undefined);
    return () => {
      alive = false;
      if (objectUrl) URL.revokeObjectURL(objectUrl);
    };
  }, [attachment]);
  if (url) return <img className={styles.thumb} src={url} alt="" draggable={false} />;
  return (
    <span className={styles.thumb} data-file aria-hidden="true">
      <FileText size={18} strokeWidth={1.5} />
    </span>
  );
}

/** Files attached to a card: add, preview, open in Preview / Finder, remove. */
export function Attachments({ itemId, version }: { itemId: string; version: string }) {
  const [list, setList] = useState<Attachment[]>([]);
  const [busy, setBusy] = useState(false);
  const input = useRef<HTMLInputElement>(null);

  const load = useCallback(() => {
    cardsApi
      .listAttachments(itemId)
      .then((l) => setList(l ?? []))
      .catch(() => undefined);
  }, [itemId]);

  useEffect(() => load(), [load, version]);

  const addFiles = async (files: FileList | File[]) => {
    setBusy(true);
    try {
      for (const file of Array.from(files)) {
        const a = await cardsApi.addAttachment(itemId, file);
        setList((l) => [...l, a]);
      }
    } catch (err) {
      toastError(message(err));
    } finally {
      setBusy(false);
    }
  };

  const fail = (err: unknown) => toastError(message(err));

  return (
    <section
      className={styles.section}
      aria-label={cardsCopy.attachments}
      onDragOver={(e) => {
        if (e.dataTransfer.types.includes("Files")) e.preventDefault();
      }}
      onDrop={(e) => {
        if (!e.dataTransfer.files.length) return;
        e.preventDefault();
        void addFiles(e.dataTransfer.files);
      }}
    >
      <div className={styles.sectionHead}>
        <span>{cardsCopy.attachments}</span>
        <button
          type="button"
          className={styles.linkBtn}
          disabled={busy}
          onClick={() => input.current?.click()}
        >
          <Paperclip size={13} strokeWidth={1.75} aria-hidden="true" />
          {cardsCopy.attach}
        </button>
        <input
          ref={input}
          type="file"
          multiple
          hidden
          onChange={(e) => {
            if (e.target.files?.length) void addFiles(e.target.files);
            e.target.value = "";
          }}
        />
      </div>
      {list.length > 0 && (
        <ul className={styles.files}>
          {list.map((a) => (
            <li key={a.id} className={styles.file}>
              <button
                type="button"
                className={styles.fileMain}
                title={cardsCopy.openFile}
                onClick={() => void cardsApi.openAttachment(a.id).catch(fail)}
              >
                <Thumb attachment={a} />
                <span className={styles.fileText}>
                  <span className={styles.fileName}>{a.name}</span>
                  <span className={styles.sectionMeta}>{cardsCopy.fileSize(a.size)}</span>
                </span>
              </button>
              <span className={styles.rowActions}>
                <button
                  type="button"
                  className={styles.miniBtn}
                  aria-label={cardsCopy.showInFinder}
                  title={cardsCopy.showInFinder}
                  onClick={() => void cardsApi.openAttachment(a.id, true).catch(fail)}
                >
                  <FolderSearch size={13} strokeWidth={1.75} aria-hidden="true" />
                </button>
                <button
                  type="button"
                  className={styles.miniBtn}
                  aria-label={cardsCopy.removeFile}
                  title={cardsCopy.removeFile}
                  onClick={() => {
                    setList((l) => l.filter((x) => x.id !== a.id));
                    void cardsApi.deleteAttachment(a.id).catch((err: unknown) => {
                      fail(err);
                      load();
                    });
                  }}
                >
                  <X size={13} strokeWidth={2} aria-hidden="true" />
                </button>
              </span>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
