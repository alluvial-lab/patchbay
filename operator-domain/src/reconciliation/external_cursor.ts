import type { AdapterId } from "@patchbay/contracts";

const MAX_SCOPE_COMPONENT_LENGTH = 512;
const MAX_ENTRY_ID_LENGTH = 1024;

/** Cursor continuity is external-system continuity, never a Patchbay generation. */
export interface ExternalCursorScope {
  readonly adapterId: AdapterId["value"];
  readonly deploymentScope: string;
  readonly externalContinuityId: string;
}

export interface ProjectionReplacement<Entry, Cursor, Leaf> {
  readonly replacementEpoch: bigint;
  readonly exactEntries: readonly Entry[];
  readonly cursor: Cursor;
  readonly leaf: Leaf;
}

/** Adapter-owned external reconciliation boundary shared by adapter profiles. */
export interface AuthoritativeCursorReplacement<Scope, Entry, Cursor, Leaf> {
  reconcileKnown(scope: Scope, cursor: Cursor): Promise<readonly Entry[]>;
  stageReplacement(scope: Scope): Promise<{ entries: readonly Entry[]; leaf: Leaf }>;
  commitReplacement(
    scope: Scope,
    replacement: ProjectionReplacement<Entry, Cursor, Leaf>,
  ): Promise<void>;
}

export interface KnownCursorSuffix<Entry, Cursor, Leaf> {
  readonly baseCursor: Cursor;
  readonly entries: readonly Entry[];
  readonly cursor: Cursor;
  readonly leaf: Leaf;
}

export interface StagedProjectionReplacement<Entry, Leaf> {
  readonly replacementEpoch: bigint;
  readonly exactEntries: readonly Entry[];
  readonly leaf: Leaf;
}

export type PendingProjectionReplacement<Entry, Leaf> =
  | {
      readonly kind: "fetching";
      readonly replacementEpoch: bigint;
    }
  | ({ readonly kind: "staged" } & StagedProjectionReplacement<Entry, Leaf>);

export interface ExternalCursorProjectionRecord<Entry, Cursor, Leaf> {
  /** Store-owned compare-and-swap version, separate from the external epoch. */
  readonly recordVersion: bigint;
  readonly freshness: "current" | "stale";
  readonly projection: ProjectionReplacement<Entry, Cursor, Leaf>;
  readonly pendingReplacement?: PendingProjectionReplacement<Entry, Leaf>;
}

/**
 * The store must compare recordVersion and install the complete next record in
 * one atomic operation. A rejection must install nothing; a success may still
 * be followed by an ambiguous transport/process failure.
 */
export interface AtomicExternalCursorProjectionStore<
  Scope,
  Entry,
  Cursor,
  Leaf,
> {
  load(scope: Scope): Promise<ExternalCursorProjectionRecord<Entry, Cursor, Leaf> | undefined>;
  compareAndSwap(
    scope: Scope,
    expectedRecordVersion: bigint,
    next: ExternalCursorProjectionRecord<Entry, Cursor, Leaf>,
  ): Promise<void>;
}

export interface ExternalCursorValueContract<Entry, Cursor, Leaf> {
  entryIdentity(entry: Entry): string;
  entriesEqual(left: Entry, right: Entry): boolean;
  cursorsEqual(left: Cursor, right: Cursor): boolean;
  leavesEqual(left: Leaf, right: Leaf): boolean;
}

export class ExternalCursorInvariantError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ExternalCursorInvariantError";
  }
}

/**
 * Length framing makes the three verified continuity dimensions collision-free.
 * Extra object properties are deliberately ignored, including any runtime
 * generation decoration carried by a caller.
 */
export function externalCursorScopeKey(scope: ExternalCursorScope): string {
  const fields = [
    boundedIdentity(scope.adapterId, "adapterId", MAX_SCOPE_COMPONENT_LENGTH),
    boundedIdentity(scope.deploymentScope, "deploymentScope", MAX_SCOPE_COMPONENT_LENGTH),
    boundedIdentity(
      scope.externalContinuityId,
      "externalContinuityId",
      MAX_SCOPE_COMPONENT_LENGTH,
    ),
  ];
  return fields.map((field) => `${field.length}:${field}`).join("|");
}

/**
 * Adapter-neutral transition owner for one external cursor projection. Fetches
 * and durable writes are injected so tests and adapters can model crash points
 * without filesystem behavior entering the domain.
 */
export class ExternalCursorProjectionMachine<
  Scope extends ExternalCursorScope,
  Entry,
  Cursor,
  Leaf,
> {
  constructor(
    private readonly store: AtomicExternalCursorProjectionStore<Scope, Entry, Cursor, Leaf>,
    private readonly values: ExternalCursorValueContract<Entry, Cursor, Leaf>,
  ) {}

  async read(
    scope: Scope,
  ): Promise<ExternalCursorProjectionRecord<Entry, Cursor, Leaf> | undefined> {
    this.validateScope(scope);
    const record = await this.store.load(scope);
    if (!record) return undefined;
    this.validateRecord(record);
    return copyRecord(record);
  }

  /** Apply a known-cursor suffix without deleting any pre-existing member. */
  async applyKnownSuffix(
    scope: Scope,
    suffix: KnownCursorSuffix<Entry, Cursor, Leaf>,
  ): Promise<void> {
    const current = await this.loadRequired(scope);
    if (current.freshness !== "current" || current.pendingReplacement) {
      throw new ExternalCursorInvariantError(
        "a known-cursor suffix cannot apply while authoritative replacement is pending",
      );
    }

    const merged = this.mergeSuffix(current.projection.exactEntries, suffix.entries);
    const isCommittedRetry = this.values.cursorsEqual(current.projection.cursor, suffix.cursor)
      && this.values.leavesEqual(current.projection.leaf, suffix.leaf)
      && this.entrySequencesEqual(current.projection.exactEntries, merged);
    if (isCommittedRetry) return;

    if (!this.values.cursorsEqual(current.projection.cursor, suffix.baseCursor)) {
      throw new ExternalCursorInvariantError("known-cursor suffix base does not match current cursor");
    }

    const next: ExternalCursorProjectionRecord<Entry, Cursor, Leaf> = {
      recordVersion: current.recordVersion + 1n,
      freshness: "current",
      projection: {
        replacementEpoch: current.projection.replacementEpoch,
        exactEntries: merged,
        cursor: suffix.cursor,
        leaf: suffix.leaf,
      },
    };
    await this.store.compareAndSwap(scope, current.recordVersion, next);
  }

  /**
   * Mark the old projection stale before fetching, then persist the validated
   * complete candidate without making its leaf or cursor current.
   */
  async stageAuthoritativeReplacement(
    scope: Scope,
    fetchComplete: () => Promise<{ entries: readonly Entry[]; leaf: Leaf }>,
  ): Promise<StagedProjectionReplacement<Entry, Leaf>> {
    let current = await this.loadRequired(scope);

    if (current.pendingReplacement?.kind === "staged") {
      return copyStage(current.pendingReplacement);
    }

    if (!current.pendingReplacement) {
      const fetching: ExternalCursorProjectionRecord<Entry, Cursor, Leaf> = {
        recordVersion: current.recordVersion + 1n,
        freshness: "stale",
        projection: copyProjection(current.projection),
        pendingReplacement: {
          kind: "fetching",
          replacementEpoch: current.projection.replacementEpoch + 1n,
        },
      };
      await this.store.compareAndSwap(scope, current.recordVersion, fetching);
      current = fetching;
    } else if (current.freshness !== "stale") {
      throw new ExternalCursorInvariantError("a fetching replacement requires a stale projection");
    }

    const fetched = await fetchComplete();
    const exactEntries = this.validateExactEntries(fetched.entries);
    const latest = await this.loadRequired(scope);
    const epoch = current.pendingReplacement!.replacementEpoch;

    if (latest.pendingReplacement?.kind === "staged") {
      if (
        latest.pendingReplacement.replacementEpoch === epoch
        && this.entrySequencesEqual(latest.pendingReplacement.exactEntries, exactEntries)
        && this.values.leavesEqual(latest.pendingReplacement.leaf, fetched.leaf)
      ) {
        return copyStage(latest.pendingReplacement);
      }
      throw new ExternalCursorInvariantError("replacement epoch already has conflicting staged content");
    }
    if (
      latest.pendingReplacement?.kind !== "fetching"
      || latest.pendingReplacement.replacementEpoch !== epoch
      || latest.freshness !== "stale"
    ) {
      throw new ExternalCursorInvariantError("replacement staging lost its expected epoch");
    }

    const staged: StagedProjectionReplacement<Entry, Leaf> = {
      replacementEpoch: epoch,
      exactEntries,
      leaf: fetched.leaf,
    };
    const next: ExternalCursorProjectionRecord<Entry, Cursor, Leaf> = {
      recordVersion: latest.recordVersion + 1n,
      freshness: "stale",
      projection: copyProjection(latest.projection),
      pendingReplacement: { kind: "staged", ...staged },
    };
    await this.store.compareAndSwap(scope, latest.recordVersion, next);
    return copyStage(staged);
  }

  /** Install exact projection membership, leaf, cursor, and epoch together. */
  async commitReplacement(
    scope: Scope,
    replacement: ProjectionReplacement<Entry, Cursor, Leaf>,
  ): Promise<void> {
    const exactReplacement: ProjectionReplacement<Entry, Cursor, Leaf> = {
      replacementEpoch: replacement.replacementEpoch,
      exactEntries: this.validateExactEntries(replacement.exactEntries),
      cursor: replacement.cursor,
      leaf: replacement.leaf,
    };
    const current = await this.loadRequired(scope);

    if (!current.pendingReplacement) {
      if (
        current.freshness === "current"
        && this.projectionsEqual(current.projection, exactReplacement)
      ) return;
      throw new ExternalCursorInvariantError("no staged authoritative replacement exists");
    }
    if (current.pendingReplacement.kind !== "staged" || current.freshness !== "stale") {
      throw new ExternalCursorInvariantError("authoritative replacement is not ready to commit");
    }
    if (
      current.pendingReplacement.replacementEpoch !== exactReplacement.replacementEpoch
      || !this.entrySequencesEqual(
        current.pendingReplacement.exactEntries,
        exactReplacement.exactEntries,
      )
      || !this.values.leavesEqual(current.pendingReplacement.leaf, exactReplacement.leaf)
    ) {
      throw new ExternalCursorInvariantError("committed replacement differs from its staged exact set");
    }

    const committed: ExternalCursorProjectionRecord<Entry, Cursor, Leaf> = {
      recordVersion: current.recordVersion + 1n,
      freshness: "current",
      projection: exactReplacement,
    };
    await this.store.compareAndSwap(scope, current.recordVersion, committed);
  }

  private async loadRequired(
    scope: Scope,
  ): Promise<ExternalCursorProjectionRecord<Entry, Cursor, Leaf>> {
    this.validateScope(scope);
    const record = await this.store.load(scope);
    if (!record) throw new ExternalCursorInvariantError("external cursor projection is not initialized");
    this.validateRecord(record);
    return record;
  }

  private validateScope(scope: Scope): void {
    externalCursorScopeKey(scope);
  }

  private validateRecord(record: ExternalCursorProjectionRecord<Entry, Cursor, Leaf>): void {
    nonNegative(record.recordVersion, "recordVersion");
    nonNegative(record.projection.replacementEpoch, "replacementEpoch");
    this.validateExactEntries(record.projection.exactEntries);
    const pending = record.pendingReplacement;
    if (!pending) return;
    if (record.freshness !== "stale") {
      throw new ExternalCursorInvariantError("a pending replacement cannot coexist with current freshness");
    }
    if (pending.replacementEpoch !== record.projection.replacementEpoch + 1n) {
      throw new ExternalCursorInvariantError("pending replacement epoch must immediately follow current epoch");
    }
    if (pending.kind === "staged") this.validateExactEntries(pending.exactEntries);
  }

  private validateExactEntries(entries: readonly Entry[]): readonly Entry[] {
    const result = [...entries];
    const identities = new Set<string>();
    for (const entry of result) {
      const identity = this.entryIdentity(entry);
      if (identities.has(identity)) {
        throw new ExternalCursorInvariantError(`duplicate exact entry identity: ${identity}`);
      }
      identities.add(identity);
    }
    return result;
  }

  private mergeSuffix(existing: readonly Entry[], suffix: readonly Entry[]): readonly Entry[] {
    const result = [...this.validateExactEntries(existing)];
    const indexes = new Map(result.map((entry, index) => [this.entryIdentity(entry), index]));
    const suffixIdentities = new Set<string>();
    for (const entry of suffix) {
      const identity = this.entryIdentity(entry);
      if (suffixIdentities.has(identity)) {
        throw new ExternalCursorInvariantError(`duplicate suffix entry identity: ${identity}`);
      }
      suffixIdentities.add(identity);
      const index = indexes.get(identity);
      if (index === undefined) {
        indexes.set(identity, result.length);
        result.push(entry);
      } else if (!this.values.entriesEqual(result[index]!, entry)) {
        throw new ExternalCursorInvariantError(`known suffix conflicts at entry identity: ${identity}`);
      }
    }
    return result;
  }

  private entryIdentity(entry: Entry): string {
    return boundedIdentity(
      this.values.entryIdentity(entry),
      "entry identity",
      MAX_ENTRY_ID_LENGTH,
    );
  }

  private entrySequencesEqual(left: readonly Entry[], right: readonly Entry[]): boolean {
    return left.length === right.length
      && left.every((entry, index) => {
        const candidate = right[index]!;
        return this.entryIdentity(entry) === this.entryIdentity(candidate)
          && this.values.entriesEqual(entry, candidate);
      });
  }

  private projectionsEqual(
    left: ProjectionReplacement<Entry, Cursor, Leaf>,
    right: ProjectionReplacement<Entry, Cursor, Leaf>,
  ): boolean {
    return left.replacementEpoch === right.replacementEpoch
      && this.entrySequencesEqual(left.exactEntries, right.exactEntries)
      && this.values.cursorsEqual(left.cursor, right.cursor)
      && this.values.leavesEqual(left.leaf, right.leaf);
  }
}

function boundedIdentity(value: string, field: string, max: number): string {
  if (typeof value !== "string" || value.length === 0 || value.length > max) {
    throw new ExternalCursorInvariantError(`${field} must be a bounded non-empty string`);
  }
  return value;
}

function nonNegative(value: bigint, field: string): void {
  if (typeof value !== "bigint" || value < 0n) {
    throw new ExternalCursorInvariantError(`${field} must be a non-negative bigint`);
  }
}

function copyProjection<Entry, Cursor, Leaf>(
  projection: ProjectionReplacement<Entry, Cursor, Leaf>,
): ProjectionReplacement<Entry, Cursor, Leaf> {
  return { ...projection, exactEntries: [...projection.exactEntries] };
}

function copyStage<Entry, Leaf>(
  stage: StagedProjectionReplacement<Entry, Leaf>,
): StagedProjectionReplacement<Entry, Leaf> {
  return { ...stage, exactEntries: [...stage.exactEntries] };
}

function copyRecord<Entry, Cursor, Leaf>(
  record: ExternalCursorProjectionRecord<Entry, Cursor, Leaf>,
): ExternalCursorProjectionRecord<Entry, Cursor, Leaf> {
  const pending = record.pendingReplacement;
  return {
    recordVersion: record.recordVersion,
    freshness: record.freshness,
    projection: copyProjection(record.projection),
    ...(pending
      ? {
          pendingReplacement: pending.kind === "fetching"
            ? { ...pending }
            : { kind: "staged" as const, ...copyStage(pending) },
        }
      : {}),
  };
}
