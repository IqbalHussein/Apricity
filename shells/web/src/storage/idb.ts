import { recipeParse, recipeToJson } from "../core/wasm";
import type { Recipe } from "../core/wasm";
import type { PromptHistoryStore } from "../state/ai";

/** IndexedDB database name (ADR-0010). */
export const DB_NAME = "apricity";
/** Schema version of the database. */
export const DB_VERSION = 1;
/** Object stores, one per record type. */
export const STORE_NAMES = ["recipes", "luts", "prompts", "usage", "settings"] as const;
/** Name of one object store. */
export type StoreName = (typeof STORE_NAMES)[number];

/**
 * Minimal async key/value surface over the object stores. Keys are strings and
 * values are structured-cloneable. Two implementations exist: IndexedDB and an
 * in-memory fallback (private windows, tests).
 */
export interface StorageBackend {
  /** `true` when records survive a reload; `false` for the in-memory fallback. */
  readonly persistent: boolean;
  /**
   * @param store - Object store.
   * @param key - Record key.
   * @returns The record, or `undefined` when absent.
   */
  get<T>(store: StoreName, key: string): Promise<T | undefined>;
  /**
   * Insert or replace a record.
   *
   * @param store - Object store.
   * @param key - Record key.
   * @param value - Structured-cloneable value.
   */
  put(store: StoreName, key: string, value: unknown): Promise<void>;
  /**
   * Remove a record; absent keys are ignored.
   *
   * @param store - Object store.
   * @param key - Record key.
   */
  delete(store: StoreName, key: string): Promise<void>;
  /**
   * @param store - Object store.
   * @returns Every `[key, value]` pair of the store.
   */
  entries<T>(store: StoreName): Promise<[string, T][]>;
}

/** In-memory {@link StorageBackend}; nothing persists past the page. */
export class MemoryBackend implements StorageBackend {
  readonly persistent = false;
  private readonly data = new Map<StoreName, Map<string, unknown>>();

  private table(store: StoreName): Map<string, unknown> {
    let t = this.data.get(store);
    if (!t) {
      t = new Map();
      this.data.set(store, t);
    }
    return t;
  }

  get<T>(store: StoreName, key: string): Promise<T | undefined> {
    const v = this.table(store).get(key);
    return Promise.resolve(v === undefined ? undefined : (structuredClone(v) as T));
  }

  put(store: StoreName, key: string, value: unknown): Promise<void> {
    this.table(store).set(key, structuredClone(value));
    return Promise.resolve();
  }

  delete(store: StoreName, key: string): Promise<void> {
    this.table(store).delete(key);
    return Promise.resolve();
  }

  entries<T>(store: StoreName): Promise<[string, T][]> {
    return Promise.resolve(
      [...this.table(store)].map(([k, v]) => [k, structuredClone(v) as T] as [string, T]),
    );
  }
}

function request<T>(r: IDBRequest<T>): Promise<T> {
  return new Promise((resolve, reject) => {
    r.onsuccess = () => resolve(r.result);
    r.onerror = () => reject(r.error ?? new Error("IndexedDB request failed"));
  });
}

/** {@link StorageBackend} over a real `IDBDatabase`. */
export class IdbBackend implements StorageBackend {
  readonly persistent = true;

  /** @param db - An open database with all {@link STORE_NAMES} created. */
  constructor(private readonly db: IDBDatabase) {}

  private store(name: StoreName, mode: IDBTransactionMode): IDBObjectStore {
    return this.db.transaction(name, mode).objectStore(name);
  }

  get<T>(store: StoreName, key: string): Promise<T | undefined> {
    return request<T | undefined>(this.store(store, "readonly").get(key));
  }

  async put(store: StoreName, key: string, value: unknown): Promise<void> {
    await request(this.store(store, "readwrite").put(value, key));
  }

  async delete(store: StoreName, key: string): Promise<void> {
    await request(this.store(store, "readwrite").delete(key));
  }

  async entries<T>(store: StoreName): Promise<[string, T][]> {
    const s = this.store(store, "readonly");
    const [keys, values] = await Promise.all([request(s.getAllKeys()), request(s.getAll())]);
    return keys.map((k, i) => [String(k), values[i] as T] as [string, T]);
  }
}

/**
 * Open the `apricity` database, creating the object stores on first use.
 *
 * @param factory - IndexedDB factory (injectable for tests); defaults to the global one.
 * @returns The IndexedDB backend, or the in-memory fallback when IndexedDB is
 *   missing or refuses to open (private windows, blocked site data). Never throws.
 */
export async function openBackend(
  factory: IDBFactory | undefined = globalThis.indexedDB,
): Promise<StorageBackend> {
  if (!factory) return new MemoryBackend();
  try {
    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const req = factory.open(DB_NAME, DB_VERSION);
      req.onupgradeneeded = () => {
        for (const name of STORE_NAMES) {
          if (!req.result.objectStoreNames.contains(name)) req.result.createObjectStore(name);
        }
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error ?? new Error("IndexedDB open failed"));
      req.onblocked = () => reject(new Error("IndexedDB open blocked"));
    });
    return new IdbBackend(db);
  } catch {
    return new MemoryBackend();
  }
}

/** A stored recipe record: the recipe as Rust-owned JSON, so loads always re-validate. */
interface RecipeRecord {
  json: string;
}

/**
 * Recipe persistence. Records are stored as `recipeToJson` text and parsed
 * with `recipeParse` on every load, so migration and sanitization (ADR-0010)
 * run each time. Records that no longer parse are skipped.
 */
export class RecipeStore {
  /**
   * @param backend - Storage to use.
   * @param parse - Recipe parser; defaults to the core's `recipeParse`.
   * @param toJson - Recipe serializer; defaults to the core's `recipeToJson`.
   */
  constructor(
    private readonly backend: StorageBackend,
    private readonly parse: (json: string) => Recipe = recipeParse,
    private readonly toJson: (r: Recipe) => string = recipeToJson,
  ) {}

  /**
   * @returns Every stored recipe that still parses, newest update first.
   */
  async list(): Promise<Recipe[]> {
    const out: Recipe[] = [];
    for (const [, rec] of await this.backend.entries<RecipeRecord>("recipes")) {
      try {
        out.push(this.parse(rec.json));
      } catch {
        // an unreadable record is skipped rather than blocking the library
      }
    }
    return out.sort((a, b) => b.updated_at_ms - a.updated_at_ms || a.id.localeCompare(b.id));
  }

  /**
   * Insert or replace a recipe.
   *
   * @param recipe - Recipe to store, keyed by its id.
   */
  async put(recipe: Recipe): Promise<void> {
    await this.backend.put("recipes", recipe.id, { json: this.toJson(recipe) } satisfies RecipeRecord);
  }

  /**
   * @param id - Recipe id to remove.
   */
  async delete(id: string): Promise<void> {
    await this.backend.delete("recipes", id);
  }
}

/** An imported 3D LUT as stored. */
export interface LutRecord {
  /** Host-generated id, written to `stage2.lut_id`. */
  id: string;
  /** Display title (the `.cube` TITLE, or the file name). */
  title: string;
  /** Grid points per axis. */
  size: number;
  /** Complete `.cube` file text. */
  text: string;
}

/** Imported LUT persistence. */
export class LutStore {
  /** @param backend - Storage to use. */
  constructor(private readonly backend: StorageBackend) {}

  /** @returns All stored LUTs. */
  async list(): Promise<LutRecord[]> {
    return (await this.backend.entries<LutRecord>("luts")).map(([, v]) => v);
  }

  /**
   * @param lut - LUT to store, keyed by its id.
   */
  async put(lut: LutRecord): Promise<void> {
    await this.backend.put("luts", lut.id, lut);
  }

  /**
   * @param id - LUT id to remove.
   */
  async delete(id: string): Promise<void> {
    await this.backend.delete("luts", id);
  }
}

const PROMPT_KEY = "history";

/** {@link PromptHistoryStore} over the `prompts` object store. */
export class IdbPromptStore implements PromptHistoryStore {
  /** @param backend - Storage to use. */
  constructor(private readonly backend: StorageBackend) {}

  async load(): Promise<readonly string[]> {
    const v = await this.backend.get<unknown>("prompts", PROMPT_KEY);
    return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : [];
  }

  async save(entries: readonly string[]): Promise<void> {
    await this.backend.put("prompts", PROMPT_KEY, [...entries]);
  }
}

/** Monthly usage counters for one calendar month. */
export interface UsageRecord {
  /** Calendar month, `YYYY-MM` (UTC). */
  month: string;
  /** Images imported this month. */
  imports: number;
  /** AI prompts sent this month. */
  prompts: number;
}

/** Usage counter persistence, keyed by calendar month. */
export class UsageStore {
  /** @param backend - Storage to use. */
  constructor(private readonly backend: StorageBackend) {}

  /**
   * @param month - Calendar month, `YYYY-MM`.
   * @returns The stored counters, or `undefined` for a month with no activity.
   */
  async get(month: string): Promise<UsageRecord | undefined> {
    const v = await this.backend.get<Partial<UsageRecord>>("usage", month);
    if (!v) return undefined;
    return {
      month,
      imports: Number.isFinite(v.imports) ? (v.imports as number) : 0,
      prompts: Number.isFinite(v.prompts) ? (v.prompts as number) : 0,
    };
  }

  /**
   * @param rec - Counters to store, keyed by `rec.month`.
   */
  async put(rec: UsageRecord): Promise<void> {
    await this.backend.put("usage", rec.month, rec);
  }
}

/** Settings persisted across sessions. */
export interface Settings {
  /** `true` once the user finished or dismissed the onboarding. */
  onboardingDone?: boolean;
}

/** Settings persistence (one record per key). */
export class SettingsStore {
  /** @param backend - Storage to use. */
  constructor(private readonly backend: StorageBackend) {}

  /** @returns All stored settings. */
  async load(): Promise<Settings> {
    const s: Settings = {};
    const done = await this.backend.get<unknown>("settings", "onboardingDone");
    if (typeof done === "boolean") s.onboardingDone = done;
    return s;
  }

  /**
   * @param key - Setting name.
   * @param value - Setting value.
   */
  async set<K extends keyof Settings>(key: K, value: NonNullable<Settings[K]>): Promise<void> {
    await this.backend.put("settings", key, value);
  }
}

let backendPromise: Promise<StorageBackend> | null = null;

/**
 * The app-wide storage backend, opened once.
 *
 * @returns The IndexedDB backend, or the in-memory fallback when unavailable.
 */
export function storage(): Promise<StorageBackend> {
  backendPromise ??= openBackend();
  return backendPromise;
}

/**
 * Replace the app-wide backend (tests).
 *
 * @param backend - Backend to use from now on.
 */
export function setStorageBackend(backend: StorageBackend): void {
  backendPromise = Promise.resolve(backend);
}
