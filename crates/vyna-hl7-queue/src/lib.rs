//! Durable store-and-forward queue for HL7 v2 messages.
//!
//! Outbound delivery needs somewhere to wait out outages: [`MemoryQueue`]
//! keeps messages in process memory while [`FileQueue`] journals them to
//! disk, surviving restarts and crashes. Both share the [`MessageQueue`]
//! contract: lease-based delivery with exponential-backoff retries and a
//! dead-letter bin after [`RetryPolicy::max_attempts`] deliveries.
//!
//! ```rust
//! use std::collections::HashMap;
//! use vyna_hl7_queue::{MemoryQueue, MessageQueue};
//!
//! let mut queue = MemoryQueue::new();
//! let id = queue.enqueue(b"MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5", HashMap::new()).unwrap();
//! let leased = queue.dequeue().unwrap().unwrap();
//! assert_eq!(leased.id, id);
//! queue.ack(&id).unwrap();
//! assert!(queue.dequeue().unwrap().is_none());
//! ```
//!
//! ## Crash model
//!
//! [`FileQueue`] writes each message as a body/metadata pair through
//! atomic renames with fsync, so an enqueued message survives a crash.
//! Messages leased but never acknowledged return to pending when their
//! lease expires (see [`RecoveryReport`] on open). One process may own a
//! directory; sharing a directory between processes is not supported.

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use thiserror::Error;
use vyna_hl7_core::Message;

/// Queue failure.
#[derive(Debug, Error)]
pub enum QueueError {
    /// Filesystem failure in the file backend.
    #[error("queue IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Corrupt or inconsistent queue state.
    #[error("queue state error: {0}")]
    State(String),
    /// Acknowledgement for an unknown or already-settled id.
    #[error("unknown message: {0}")]
    Unknown(String),
    /// Enqueue refused: the queue holds `limit` live messages already.
    #[error("queue full: at limit of {limit} live messages")]
    Full {
        /// Configured live-message limit.
        limit: usize,
    },
}

/// A leased delivery: [`MessageQueue::ack`] on success, [`MessageQueue::nack`]
/// to retry or dead-letter.
#[derive(Debug, Clone)]
pub struct Leased {
    /// Queue-assigned id.
    pub id: String,
    /// Raw message bytes.
    pub body: Vec<u8>,
    /// Caller-supplied headers from enqueue time.
    pub headers: HashMap<String, String>,
    /// Clock time (ms) when the message was first enqueued.
    pub enqueued_at_ms: u64,
    /// Deliveries so far, including this one.
    pub attempts: u32,
}

/// A message that exhausted its retries.
#[derive(Debug, Clone)]
pub struct DeadLetter {
    /// Queue-assigned id.
    pub id: String,
    /// Raw message bytes.
    pub body: Vec<u8>,
    /// Caller-supplied headers from enqueue time.
    pub headers: HashMap<String, String>,
    /// Deliveries attempted.
    pub attempts: u32,
    /// Last reported error, if any.
    pub last_error: Option<String>,
}

/// Queue depth counters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueueStats {
    /// Messages waiting for (re)delivery.
    pub pending: usize,
    /// Messages leased but unacknowledged.
    pub inflight: usize,
    /// Dead-lettered messages.
    pub dead: usize,
}

/// Outcome of [`MessageQueue::nack`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NackOutcome {
    /// Requeued; visible again after the backoff delay.
    Requeued {
        /// How long until the message is deliverable again.
        visible_in: Duration,
    },
    /// Retries exhausted; moved to the dead-letter bin.
    DeadLettered,
}

/// Retry behavior shared by all backends.
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Deliveries before a message is dead-lettered (minimum 1).
    pub max_attempts: u32,
    /// First backoff delay; doubles per attempt.
    pub base_delay: Duration,
    /// Backoff cap.
    pub max_delay: Duration,
    /// How long a lease lasts before the message is recoverable.
    pub lease: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 5,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(300),
            lease: Duration::from_secs(30),
        }
    }
}

impl RetryPolicy {
    /// Backoff after `attempts` deliveries (attempts >= 1).
    fn backoff(&self, attempts: u32) -> Duration {
        let shift = attempts.saturating_sub(1).min(20);
        let delay = self.base_delay.saturating_mul(1u32 << shift);
        delay.min(self.max_delay)
    }
}

/// Clock for visibility and lease math. Inject a [`ManualClock`] in tests.
pub trait QueueClock: Send + Sync {
    /// Current time as milliseconds since the Unix epoch.
    fn now_ms(&self) -> u64;
}

/// The system clock.
#[derive(Debug, Default)]
pub struct SystemClock;

impl QueueClock for SystemClock {
    fn now_ms(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
}

/// A hand-advanced clock for deterministic tests.
#[derive(Debug, Default)]
pub struct ManualClock {
    ms: AtomicU64,
}

impl ManualClock {
    /// Start at the given epoch milliseconds.
    pub fn new(ms: u64) -> Self {
        Self {
            ms: AtomicU64::new(ms),
        }
    }

    /// Move the clock forward.
    pub fn advance(&self, duration: Duration) {
        self.ms
            .fetch_add(duration.as_millis() as u64, Ordering::SeqCst);
    }
}

impl QueueClock for ManualClock {
    fn now_ms(&self) -> u64 {
        self.ms.load(Ordering::SeqCst)
    }
}

/// The store-and-forward contract all backends implement.
pub trait MessageQueue {
    /// Store a message, returning its queue-assigned id.
    fn enqueue(
        &mut self,
        body: &[u8],
        headers: HashMap<String, String>,
    ) -> Result<String, QueueError>;

    /// Encode and store an HL7 message with no headers.
    fn enqueue_message(&mut self, message: &Message) -> Result<String, QueueError> {
        self.enqueue(message.encode().as_bytes(), HashMap::new())
    }

    /// Lease the oldest visible message, or `None` when empty.
    fn dequeue(&mut self) -> Result<Option<Leased>, QueueError>;

    /// Settle a leased message.
    fn ack(&mut self, id: &str) -> Result<(), QueueError>;

    /// Return a leased message for retry (or dead-letter it), recording
    /// the caller's error.
    fn nack(&mut self, id: &str, error: Option<&str>) -> Result<NackOutcome, QueueError>;

    /// Inspect the dead-letter bin.
    fn dead_letters(&self) -> Result<Vec<DeadLetter>, QueueError>;

    /// Current depths.
    fn stats(&self) -> Result<QueueStats, QueueError>;
}

/// Process-wide id sequence: two handles on one directory must never mint
/// the same id, so the counter cannot live per handle.
static NEXT_ID_SEQ: AtomicU64 = AtomicU64::new(0);

/// `now + delay` in epoch milliseconds, saturating: extreme backoffs or
/// leases pin to `u64::MAX` instead of overflowing (a debug panic) or
/// wrapping (a release lie that contradicts the reported `visible_in`).
fn saturating_add_delay(now: u64, delay: Duration) -> u64 {
    now.saturating_add(u64::try_from(delay.as_millis()).unwrap_or(u64::MAX))
}

/// Serializes mutating [`FileQueue`] ops across same-process handles.
/// Dequeue already needed it so two handles never lease one message, but
/// enqueue/ack/nack/open must hold it too: without that, a dequeue's
/// recover/scan/claim window can observe a meta file whose body a
/// concurrent enqueue has not written yet, and quarantine a live message
/// as bodyless. Sharing a directory between processes is unsupported (see
/// [`FileQueue`]), so a process-wide mutex is sufficient — no lock files.
static DEQUEUE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Hold across every mutating [`FileQueue`] op (enqueue, dequeue, ack,
/// nack, open). A poisoned lock still serializes; the state on disk stays
/// consistent either way.
fn queue_lock() -> std::sync::MutexGuard<'static, ()> {
    DEQUEUE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Next queue id: hex nanos, pid, and a process-wide sequence
/// (filename-safe). Unique across same-process handles even when the clock
/// does not advance between enqueues.
fn next_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let seq = NEXT_ID_SEQ.fetch_add(1, Ordering::SeqCst);
    format!("{nanos:x}-{:x}-{seq:x}", std::process::id())
}

#[derive(Debug, Clone)]
enum EntryState {
    Pending { visible_at_ms: u64 },
    Inflight { leased_until_ms: u64 },
    Dead { last_error: Option<String> },
}

#[derive(Debug, Clone)]
struct Entry {
    id: String,
    body: Vec<u8>,
    headers: HashMap<String, String>,
    attempts: u32,
    enqueued_at_ms: u64,
    state: EntryState,
}

/// In-process queue. Fast and dependency-free, but messages die with the
/// process; use [`FileQueue`] when they must survive it.
pub struct MemoryQueue {
    entries: HashMap<String, Entry>,
    order: VecDeque<String>,
    policy: RetryPolicy,
    clock: Arc<dyn QueueClock>,
    max_depth: Option<usize>,
    live: usize,
}

impl MemoryQueue {
    /// Default policy on the system clock.
    pub fn new() -> Self {
        Self::with_policy_and_clock(RetryPolicy::default(), Arc::new(SystemClock))
    }

    /// Custom policy on the system clock.
    pub fn with_policy(policy: RetryPolicy) -> Self {
        Self::with_policy_and_clock(policy, Arc::new(SystemClock))
    }

    /// Custom policy and clock (pass an `Arc<ManualClock>` in tests).
    pub fn with_policy_and_clock(policy: RetryPolicy, clock: Arc<dyn QueueClock>) -> Self {
        Self {
            entries: HashMap::new(),
            order: VecDeque::new(),
            policy: RetryPolicy {
                max_attempts: policy.max_attempts.max(1),
                ..policy
            },
            clock,
            max_depth: None,
            live: 0,
        }
    }

    /// Cap live (pending + inflight) messages; further enqueues fail with
    /// [`QueueError::Full`]. Dead-lettered messages never count against the
    /// cap. `None` (the default) is unbounded.
    pub fn set_max_depth(&mut self, limit: Option<usize>) {
        self.max_depth = limit;
    }

    /// The live-message cap, if any.
    pub fn max_depth(&self) -> Option<usize> {
        self.max_depth
    }

    /// Expired leases return to pending, visible immediately.
    fn sweep_expired_leases(&mut self) {
        let now = self.clock.now_ms();
        for entry in self.entries.values_mut() {
            if let EntryState::Inflight { leased_until_ms } = entry.state
                && leased_until_ms <= now
            {
                entry.state = EntryState::Pending { visible_at_ms: now };
            }
        }
    }
}

impl Default for MemoryQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageQueue for MemoryQueue {
    fn enqueue(
        &mut self,
        body: &[u8],
        headers: HashMap<String, String>,
    ) -> Result<String, QueueError> {
        if let Some(limit) = self.max_depth
            && self.live >= limit
        {
            return Err(QueueError::Full { limit });
        }
        let id = next_id();
        let now = self.clock.now_ms();
        self.order.push_back(id.clone());
        self.entries.insert(
            id.clone(),
            Entry {
                id: id.clone(),
                body: body.to_vec(),
                headers,
                attempts: 0,
                enqueued_at_ms: now,
                state: EntryState::Pending { visible_at_ms: now },
            },
        );
        self.live += 1;
        Ok(id)
    }

    fn dequeue(&mut self) -> Result<Option<Leased>, QueueError> {
        self.sweep_expired_leases();
        let now = self.clock.now_ms();
        let next = self.order.iter().find(|id| {
            matches!(
                self.entries.get(*id).map(|e| &e.state),
                Some(EntryState::Pending { visible_at_ms }) if *visible_at_ms <= now
            )
        });
        let Some(id) = next.cloned() else {
            return Ok(None);
        };
        let entry = self.entries.get_mut(&id).expect("order and map agree");
        entry.attempts += 1;
        entry.state = EntryState::Inflight {
            leased_until_ms: saturating_add_delay(now, self.policy.lease),
        };
        Ok(Some(Leased {
            id: entry.id.clone(),
            body: entry.body.clone(),
            headers: entry.headers.clone(),
            enqueued_at_ms: entry.enqueued_at_ms,
            attempts: entry.attempts,
        }))
    }

    fn ack(&mut self, id: &str) -> Result<(), QueueError> {
        match self.entries.get(id).map(|e| &e.state) {
            Some(EntryState::Inflight { .. }) => {
                self.entries.remove(id);
                self.order.retain(|queued| queued != id);
                self.live = self.live.saturating_sub(1);
                Ok(())
            }
            _ => Err(QueueError::Unknown(id.to_string())),
        }
    }

    fn nack(&mut self, id: &str, error: Option<&str>) -> Result<NackOutcome, QueueError> {
        let now = self.clock.now_ms();
        let entry = self
            .entries
            .get_mut(id)
            .filter(|e| matches!(e.state, EntryState::Inflight { .. }))
            .ok_or_else(|| QueueError::Unknown(id.to_string()))?;
        if entry.attempts >= self.policy.max_attempts {
            entry.state = EntryState::Dead {
                last_error: error.map(str::to_string),
            };
            self.order.retain(|queued| queued != id);
            self.live = self.live.saturating_sub(1);
            Ok(NackOutcome::DeadLettered)
        } else {
            let delay = self.policy.backoff(entry.attempts);
            entry.state = EntryState::Pending {
                visible_at_ms: saturating_add_delay(now, delay),
            };
            Ok(NackOutcome::Requeued { visible_in: delay })
        }
    }

    fn dead_letters(&self) -> Result<Vec<DeadLetter>, QueueError> {
        let mut dead: Vec<DeadLetter> = self
            .entries
            .values()
            .filter_map(|e| match &e.state {
                EntryState::Dead { last_error } => Some(DeadLetter {
                    id: e.id.clone(),
                    body: e.body.clone(),
                    headers: e.headers.clone(),
                    attempts: e.attempts,
                    last_error: last_error.clone(),
                }),
                _ => None,
            })
            .collect();
        dead.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(dead)
    }

    fn stats(&self) -> Result<QueueStats, QueueError> {
        let mut stats = QueueStats {
            pending: 0,
            inflight: 0,
            dead: 0,
        };
        for entry in self.entries.values() {
            match entry.state {
                EntryState::Pending { .. } => stats.pending += 1,
                EntryState::Inflight { .. } => stats.inflight += 1,
                EntryState::Dead { .. } => stats.dead += 1,
            }
        }
        Ok(stats)
    }
}

/// On-disk entry metadata (the body lives in a sibling `.body` file).
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileMeta {
    id: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    attempts: u32,
    enqueued_at_ms: u64,
    #[serde(default)]
    visible_at_ms: u64,
    #[serde(default)]
    leased_until_ms: u64,
    #[serde(default)]
    last_error: Option<String>,
}

/// What [`FileQueue::open`] recovered.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecoveryReport {
    /// Leases that expired while away, returned to pending.
    pub requeued_stale: usize,
    /// Messages waiting for delivery.
    pub pending: usize,
    /// Dead-lettered messages.
    pub dead: usize,
    /// Bare ids of entries too corrupt to read, quarantined under `dead/`.
    pub skipped: Vec<String>,
}

/// File-backed durable queue.
///
/// Layout under the directory (one process per directory):
/// `pending/{id}.meta.json` + `{id}.body`, likewise `inflight/` and
/// `dead/`. Writes go through temp files plus atomic renames with fsync,
/// so an acknowledged enqueue survives a crash. Opening reaps `.tmp-*`
/// files stranded by a crash between the temp write and the rename.
/// Handles on one directory may be shared across threads: every mutating
/// op (including open's recovery) holds a process-wide lock.
pub struct FileQueue {
    pending: PathBuf,
    inflight: PathBuf,
    dead: PathBuf,
    policy: RetryPolicy,
    clock: Arc<dyn QueueClock>,
    max_depth: Option<usize>,
}

impl FileQueue {
    /// Open (creating) a queue directory with the default policy.
    pub fn open(dir: &Path) -> Result<(Self, RecoveryReport), QueueError> {
        Self::open_with_policy_and_clock(dir, RetryPolicy::default(), Arc::new(SystemClock))
    }

    /// Open with a custom policy.
    pub fn open_with_policy(
        dir: &Path,
        policy: RetryPolicy,
    ) -> Result<(Self, RecoveryReport), QueueError> {
        Self::open_with_policy_and_clock(dir, policy, Arc::new(SystemClock))
    }

    /// Open with a custom policy and clock.
    pub fn open_with_policy_and_clock(
        dir: &Path,
        policy: RetryPolicy,
        clock: Arc<dyn QueueClock>,
    ) -> Result<(Self, RecoveryReport), QueueError> {
        let pending = dir.join("pending");
        let inflight = dir.join("inflight");
        let dead = dir.join("dead");
        for subdir in [&pending, &inflight, &dead] {
            std::fs::create_dir_all(subdir)?;
        }
        let queue = Self {
            pending,
            inflight,
            dead,
            policy: RetryPolicy {
                max_attempts: policy.max_attempts.max(1),
                ..policy
            },
            clock,
            max_depth: None,
        };
        // Crash orphans from an interrupted write_entry predate this handle;
        // reaping runs once here rather than in recover() so a dequeue can
        // never delete a temp file another live handle is still writing.
        // Open takes the lock (it calls nothing else that does, so this
        // cannot deadlock): concurrent opens and live handles' mutating
        // ops serialize instead of reaping/recovering mid-write entries.
        let _guard = queue_lock();
        for dir in [&queue.pending, &queue.inflight, &queue.dead] {
            reap_temp_files(dir)?;
            reap_orphan_bodies(dir)?;
        }
        let report = queue.recover(true)?;
        Ok((queue, report))
    }

    /// Cap live (pending + inflight) messages; further enqueues fail with
    /// [`QueueError::Full`]. Dead-lettered messages never count against the
    /// cap. `None` (the default) is unbounded.
    pub fn set_max_depth(&mut self, limit: Option<usize>) {
        self.max_depth = limit;
    }

    /// The live-message cap, if any.
    pub fn max_depth(&self) -> Option<usize> {
        self.max_depth
    }

    /// Live (pending + inflight) message count.
    fn live_count(&self) -> Result<usize, QueueError> {
        Ok(list_ids(&self.pending)?.len() + list_ids(&self.inflight)?.len())
    }

    /// Crash recovery: stale leases return to pending, corrupt or
    /// bodyless entries are quarantined under `dead/`. Callers must hold
    /// [`queue_lock`].
    ///
    /// `quarantine_bodyless` is true only at open: a bodyless pending meta
    /// on the dequeue path may be half of a torn write from a concurrent
    /// enqueue, so dequeue skips those in place (see [`next_visible`])
    /// instead of quarantining a live message. Genuinely corrupt metas
    /// quarantine on both paths: atomic renames mean they were never
    /// mid-write.
    fn recover(&self, quarantine_bodyless: bool) -> Result<RecoveryReport, QueueError> {
        let mut report = RecoveryReport::default();
        let now = self.clock.now_ms();
        // Quarantine corrupt pending/dead entries first, so one bad file can
        // never brick the queue. Dead is scanned before pending, and both
        // before inflight, so newly quarantined entries are never reported
        // twice. A valid meta without its body (crash between the two
        // writes) is unrecoverable and quarantines like a corrupt meta.
        for dir in [&self.dead, &self.pending] {
            for id in list_ids(dir)? {
                let meta = read_meta(dir, &id)?;
                let bodyless = meta.is_some() && read_body(dir, &id)?.is_none();
                let corrupt = meta.is_none();
                if bodyless && dir == &self.pending && !quarantine_bodyless {
                    continue;
                }
                if corrupt || bodyless {
                    report.skipped.push(id.clone());
                    if dir != &self.dead {
                        quarantine(dir, &self.dead, &id)?;
                    }
                }
            }
        }
        for id in list_ids(&self.inflight)? {
            match read_meta(&self.inflight, &id)? {
                Some(_) if read_body(&self.inflight, &id)?.is_none() => {
                    report.skipped.push(id.clone());
                    quarantine(&self.inflight, &self.dead, &id)?;
                }
                Some(mut meta) if meta.leased_until_ms <= now => {
                    meta.visible_at_ms = now;
                    meta.leased_until_ms = 0;
                    move_entry(&self.inflight, &self.pending, &id, Some(&meta))?;
                    report.requeued_stale += 1;
                }
                Some(_) => {}
                None => {
                    report.skipped.push(id.clone());
                    quarantine(&self.inflight, &self.dead, &id)?;
                }
            }
        }
        report.pending = list_ids(&self.pending)?.len();
        report.dead = list_ids(&self.dead)?.len();
        Ok(report)
    }

    /// Oldest visible pending entry, if any. Metas are scanned first and
    /// only the winner's body is read, so a dequeue costs one body read
    /// instead of one per pending message.
    fn next_visible(&self) -> Result<Option<(FileMeta, Vec<u8>)>, QueueError> {
        let now = self.clock.now_ms();
        let mut visible: Vec<FileMeta> = Vec::new();
        for id in list_ids(&self.pending)? {
            let Some(meta) = read_meta(&self.pending, &id)? else {
                continue;
            };
            if meta.visible_at_ms <= now {
                visible.push(meta);
            }
        }
        visible.sort_by(|a, b| (a.enqueued_at_ms, &a.id).cmp(&(b.enqueued_at_ms, &b.id)));
        // Entries whose body went missing (crash leftovers) are skipped,
        // preserving the old single-pass semantics.
        for meta in visible {
            if let Some(body) = read_body(&self.pending, &meta.id)? {
                return Ok(Some((meta, body)));
            }
        }
        Ok(None)
    }
}

impl MessageQueue for FileQueue {
    fn enqueue(
        &mut self,
        body: &[u8],
        headers: HashMap<String, String>,
    ) -> Result<String, QueueError> {
        // Under the lock, before the cap check: the check-then-write is
        // atomic, and a dequeue's recover can never observe this entry
        // mid-write (meta without body) and quarantine it.
        let _guard = queue_lock();
        if let Some(limit) = self.max_depth
            && self.live_count()? >= limit
        {
            return Err(QueueError::Full { limit });
        }
        let id = next_id();
        let now = self.clock.now_ms();
        let meta = FileMeta {
            id: id.clone(),
            headers,
            attempts: 0,
            enqueued_at_ms: now,
            visible_at_ms: now,
            leased_until_ms: 0,
            last_error: None,
        };
        write_entry(&self.pending, &id, &meta, body)?;
        Ok(id)
    }

    fn dequeue(&mut self) -> Result<Option<Leased>, QueueError> {
        // One lease per message across same-process handles: the lock makes
        // the recover/scan/claim window atomic against every other
        // handle's mutating op.
        let _guard = queue_lock();
        // Opportunistic recovery keeps leases honest without reopening.
        // Bodyless pending entries are skipped in place here (only open
        // quarantines them): they may be a concurrent enqueue mid-write.
        self.recover(false)?;
        let Some((mut meta, body)) = self.next_visible()? else {
            return Ok(None);
        };
        meta.attempts += 1;
        meta.leased_until_ms = saturating_add_delay(self.clock.now_ms(), self.policy.lease);
        meta.visible_at_ms = 0;
        move_entry(&self.pending, &self.inflight, &meta.id, Some(&meta))?;
        Ok(Some(Leased {
            id: meta.id.clone(),
            body,
            headers: meta.headers.clone(),
            enqueued_at_ms: meta.enqueued_at_ms,
            attempts: meta.attempts,
        }))
    }

    fn ack(&mut self, id: &str) -> Result<(), QueueError> {
        validate_id(id)?;
        // Under the lock: the check-then-remove must be atomic against a
        // concurrent recover/move of the same entry.
        let _guard = queue_lock();
        if !meta_path(&self.inflight, id).exists() {
            return Err(QueueError::Unknown(id.to_string()));
        }
        remove_entry(&self.inflight, id)?;
        Ok(())
    }

    fn nack(&mut self, id: &str, error: Option<&str>) -> Result<NackOutcome, QueueError> {
        validate_id(id)?;
        // Under the lock: the read/move window must be atomic against a
        // concurrent recover or lease expiry of the same entry.
        let _guard = queue_lock();
        let Some(mut meta) = read_meta(&self.inflight, id)? else {
            return Err(QueueError::Unknown(id.to_string()));
        };
        if meta.attempts >= self.policy.max_attempts {
            meta.last_error = error.map(str::to_string);
            move_entry(&self.inflight, &self.dead, id, Some(&meta))?;
            Ok(NackOutcome::DeadLettered)
        } else {
            let delay = self.policy.backoff(meta.attempts);
            meta.visible_at_ms = saturating_add_delay(self.clock.now_ms(), delay);
            meta.leased_until_ms = 0;
            move_entry(&self.inflight, &self.pending, id, Some(&meta))?;
            Ok(NackOutcome::Requeued { visible_in: delay })
        }
    }

    fn dead_letters(&self) -> Result<Vec<DeadLetter>, QueueError> {
        // Under the lock: move_entry stages dest-before-source-removal,
        // so an unlocked scan can catch one message in two states.
        let _guard = queue_lock();
        let mut dead = Vec::new();
        for id in list_ids(&self.dead)? {
            let meta = read_meta(&self.dead, &id)?;
            let body = read_body(&self.dead, &id)?;
            match (meta, body) {
                (Some(meta), Some(body)) => dead.push(DeadLetter {
                    id: meta.id,
                    body,
                    headers: meta.headers,
                    attempts: meta.attempts,
                    last_error: meta.last_error,
                }),
                // Quarantined entries (corrupt meta and/or missing body)
                // count in stats(), so they must surface here too rather
                // than staying invisible: synthesize from what survived.
                (meta, body) => dead.push(DeadLetter {
                    id: id.clone(),
                    body: body.unwrap_or_default(),
                    headers: meta.as_ref().map(|m| m.headers.clone()).unwrap_or_default(),
                    attempts: meta.as_ref().map(|m| m.attempts).unwrap_or(0),
                    last_error: Some(
                        meta.as_ref()
                            .and_then(|m| m.last_error.clone())
                            .unwrap_or_else(|| "corrupt metadata quarantined".to_string()),
                    ),
                }),
            }
        }
        dead.sort_by(|a, b| a.id.cmp(&b.id));
        Ok(dead)
    }

    fn stats(&self) -> Result<QueueStats, QueueError> {
        // Under the lock: without it a concurrent move_entry (dest
        // staged before the source is removed) double-counts one
        // message across the three directory scans below.
        let _guard = queue_lock();
        Ok(QueueStats {
            pending: list_ids(&self.pending)?.len(),
            inflight: list_ids(&self.inflight)?.len(),
            dead: list_ids(&self.dead)?.len(),
        })
    }
}

/// Caller-supplied ids are joined into state-directory paths, so anything
/// that could escape the directory (separators, parent refs) or match
/// nothing is rejected as unknown. Generated ids are hex plus `-` and
/// always pass.
fn validate_id(id: &str) -> Result<(), QueueError> {
    if id.is_empty() || id.contains('/') || id.contains('\\') || id.contains("..") {
        return Err(QueueError::Unknown(id.to_string()));
    }
    Ok(())
}

fn meta_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.meta.json"))
}

fn body_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.body"))
}

/// Delete `.tmp-*` files stranded by a crash between temp write and
/// rename. Only regular files go: anything else under that prefix is left
/// alone rather than risking an unrelated directory.
fn reap_temp_files(dir: &Path) -> Result<(), QueueError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_name().to_string_lossy().starts_with(".tmp-") {
            continue;
        }
        if !entry.file_type()?.is_file() {
            continue;
        }
        match std::fs::remove_file(entry.path()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(QueueError::Io(e)),
        }
    }
    Ok(())
}

/// Delete `*.body` files with no `*.meta.json` sibling (e.g. a crash
/// between the two deletes of [`remove_entry`]). Nothing can ever deliver
/// them — listing, stats, and dequeue all key off metadata — so without
/// reaping they linger forever, invisible.
fn reap_orphan_bodies(dir: &Path) -> Result<(), QueueError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(base) = name.to_str().and_then(|n| n.strip_suffix(".body")) else {
            continue;
        };
        if meta_path(dir, base).exists() {
            continue;
        }
        match std::fs::remove_file(entry.path()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(QueueError::Io(e)),
        }
    }
    Ok(())
}

/// Entry ids from `*.meta.json` files, sorted for determinism.
fn list_ids(dir: &Path) -> Result<Vec<String>, QueueError> {
    let mut ids = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str().and_then(|n| n.strip_suffix(".meta.json")) else {
            continue;
        };
        ids.push(name.to_string());
    }
    ids.sort();
    Ok(ids)
}

/// Read metadata; `Ok(None)` when the file is missing or corrupt.
fn read_meta(dir: &Path, id: &str) -> Result<Option<FileMeta>, QueueError> {
    let path = meta_path(dir, id);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(QueueError::Io(e)),
    };
    Ok(serde_json::from_slice(&bytes).ok())
}

/// Read a body; `Ok(None)` when the file is missing.
fn read_body(dir: &Path, id: &str) -> Result<Option<Vec<u8>>, QueueError> {
    match std::fs::read(body_path(dir, id)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(QueueError::Io(e)),
    }
}

/// Sync a file and its directory so the entry survives a crash.
fn fsync_file_and_dir(path: &Path) -> Result<(), QueueError> {
    std::fs::File::open(path)?.sync_all()?;
    if let Some(parent) = path.parent() {
        std::fs::File::open(parent)?.sync_all()?;
    }
    Ok(())
}

/// Write an entry through temp files plus atomic renames. The body lands
/// first and the meta last, so a meta file implies a complete entry: a
/// concurrent reader can only ever see a body without its meta (invisible
/// — listing keys off metadata, and open reaps such orphans), never a
/// meta without its body.
fn write_entry(dir: &Path, id: &str, meta: &FileMeta, body: &[u8]) -> Result<(), QueueError> {
    let meta_json = serde_json::to_vec(meta)
        .map_err(|e| QueueError::State(format!("cannot encode entry {id}: {e}")))?;
    // Filename-safe temp suffix (ids are hex, but crash leftovers must not collide).
    let tag = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for (final_path, bytes) in [
        (body_path(dir, id), body),
        (meta_path(dir, id), meta_json.as_slice()),
    ] {
        let tmp = dir.join(format!(".tmp-{tag}-{id}"));
        std::fs::write(&tmp, bytes)?;
        fsync_file_and_dir(&tmp)?;
        std::fs::rename(&tmp, &final_path)?;
    }
    std::fs::File::open(dir)?.sync_all()?;
    Ok(())
}

/// Move an entry between states, optionally rewriting its metadata.
fn move_entry(from: &Path, to: &Path, id: &str, meta: Option<&FileMeta>) -> Result<(), QueueError> {
    if let Some(meta) = meta {
        let body = read_body(from, id)?.unwrap_or_default();
        write_entry(to, id, meta, &body)?;
        remove_entry(from, id)?;
    } else {
        std::fs::rename(meta_path(from, id), meta_path(to, id))?;
        let body_from = body_path(from, id);
        if body_from.exists() {
            std::fs::rename(body_from, body_path(to, id))?;
        }
        std::fs::File::open(to)?.sync_all()?;
    }
    Ok(())
}

/// Move an unreadable entry to the dead-letter bin for inspection.
fn quarantine(from: &Path, to: &Path, id: &str) -> Result<(), QueueError> {
    move_entry(from, to, id, None)
}

/// Delete an entry pair.
fn remove_entry(dir: &Path, id: &str) -> Result<(), QueueError> {
    for path in [meta_path(dir, id), body_path(dir, id)] {
        match std::fs::remove_file(&path) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(QueueError::Io(e)),
        }
    }
    std::fs::File::open(dir)?.sync_all()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_policy() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
            lease: Duration::from_secs(5),
        }
    }

    fn headers() -> HashMap<String, String> {
        HashMap::from([("message-type".to_string(), "ADT^A01".to_string())])
    }

    #[test]
    fn memory_round_trip() {
        let mut queue = MemoryQueue::new();
        let id = queue.enqueue(b"MSH", headers()).unwrap();

        let leased = queue.dequeue().unwrap().unwrap();
        assert_eq!(leased.id, id);
        assert_eq!(leased.body, b"MSH");
        assert_eq!(leased.headers, headers());
        assert_eq!(leased.attempts, 1);
        assert_eq!(
            queue.stats().unwrap(),
            QueueStats {
                pending: 0,
                inflight: 1,
                dead: 0
            }
        );

        queue.ack(&id).unwrap();
        assert!(queue.dequeue().unwrap().is_none());
        assert_eq!(
            queue.stats().unwrap(),
            QueueStats {
                pending: 0,
                inflight: 0,
                dead: 0
            }
        );
    }

    #[test]
    fn memory_nack_backs_off_then_dead_letters() {
        let clock = Arc::new(ManualClock::new(0));
        let mut queue = MemoryQueue::with_policy_and_clock(test_policy(), clock.clone());
        let id = queue.enqueue(b"MSH", headers()).unwrap();

        // Attempt 1 -> requeue with 1s backoff.
        let leased = queue.dequeue().unwrap().unwrap();
        assert_eq!(
            queue.nack(&leased.id, Some("boom")).unwrap(),
            NackOutcome::Requeued {
                visible_in: Duration::from_secs(1)
            }
        );
        assert!(queue.dequeue().unwrap().is_none());
        clock.advance(Duration::from_secs(1));

        // Attempt 2 -> requeue with 2s backoff.
        let leased = queue.dequeue().unwrap().unwrap();
        assert_eq!(leased.attempts, 2);
        assert_eq!(
            queue.nack(&id, None).unwrap(),
            NackOutcome::Requeued {
                visible_in: Duration::from_secs(2)
            }
        );
        clock.advance(Duration::from_secs(2));

        // Attempt 3 (max) -> dead letter.
        queue.dequeue().unwrap().unwrap();
        assert_eq!(
            queue.nack(&id, Some("final")).unwrap(),
            NackOutcome::DeadLettered
        );
        assert!(queue.dequeue().unwrap().is_none());

        let dead = queue.dead_letters().unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].body, b"MSH");
        assert_eq!(dead[0].attempts, 3);
        assert_eq!(dead[0].last_error.as_deref(), Some("final"));
        assert_eq!(
            queue.stats().unwrap(),
            QueueStats {
                pending: 0,
                inflight: 0,
                dead: 1
            }
        );
    }

    #[test]
    fn memory_max_delays_saturate_instead_of_overflowing() {
        // nack site: base_delay = MAX must not panic, and the message
        // must stay hidden consistently with visible_in = MAX.
        let clock = Arc::new(ManualClock::new(1_000));
        let policy = RetryPolicy {
            max_attempts: 5,
            base_delay: Duration::MAX,
            max_delay: Duration::MAX,
            lease: Duration::from_secs(5),
        };
        let mut queue = MemoryQueue::with_policy_and_clock(policy, clock.clone());
        let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();
        queue.dequeue().unwrap().unwrap();
        let outcome = queue.nack(&id, Some("e")).unwrap();
        assert_eq!(
            outcome,
            NackOutcome::Requeued {
                visible_in: Duration::MAX
            }
        );
        clock.advance(Duration::from_secs(60));
        assert!(
            queue.dequeue().unwrap().is_none(),
            "nacked message visible despite visible_in = MAX"
        );

        // dequeue site: lease = MAX must not panic, and the lease must
        // hold (no immediate redelivery).
        let clock = Arc::new(ManualClock::new(2_000));
        let policy = RetryPolicy {
            max_attempts: 5,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
            lease: Duration::MAX,
        };
        let mut queue = MemoryQueue::with_policy_and_clock(policy, clock.clone());
        queue.enqueue(b"MSH", HashMap::new()).unwrap();
        queue.dequeue().unwrap().unwrap();
        clock.advance(Duration::from_secs(60));
        assert!(
            queue.dequeue().unwrap().is_none(),
            "leased message redelivered despite lease = MAX"
        );
    }

    #[test]
    fn file_max_delays_saturate_instead_of_overflowing() {
        // nack site.
        let dir = scratch_dir();
        let clock = Arc::new(ManualClock::new(1_000));
        let policy = RetryPolicy {
            max_attempts: 5,
            base_delay: Duration::MAX,
            max_delay: Duration::MAX,
            lease: Duration::from_secs(5),
        };
        let (mut queue, _) =
            FileQueue::open_with_policy_and_clock(&dir, policy, clock.clone()).unwrap();
        let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();
        queue.dequeue().unwrap().unwrap();
        let outcome = queue.nack(&id, Some("e")).unwrap();
        assert_eq!(
            outcome,
            NackOutcome::Requeued {
                visible_in: Duration::MAX
            }
        );
        clock.advance(Duration::from_secs(60));
        assert!(
            queue.dequeue().unwrap().is_none(),
            "nacked message visible despite visible_in = MAX"
        );
        drop(queue);
        std::fs::remove_dir_all(&dir).unwrap();

        // dequeue site.
        let dir = scratch_dir();
        let clock = Arc::new(ManualClock::new(2_000));
        let policy = RetryPolicy {
            max_attempts: 5,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(10),
            lease: Duration::MAX,
        };
        let (mut queue, _) =
            FileQueue::open_with_policy_and_clock(&dir, policy, clock.clone()).unwrap();
        queue.enqueue(b"MSH", HashMap::new()).unwrap();
        queue.dequeue().unwrap().unwrap();
        clock.advance(Duration::from_secs(60));
        assert!(
            queue.dequeue().unwrap().is_none(),
            "leased message redelivered despite lease = MAX"
        );
        drop(queue);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn memory_unknown_ack_and_nack_are_errors() {
        let mut queue = MemoryQueue::new();
        assert!(matches!(queue.ack("nope"), Err(QueueError::Unknown(_))));
        assert!(matches!(
            queue.nack("nope", None),
            Err(QueueError::Unknown(_))
        ));

        // Settled ids cannot be acked twice.
        let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();
        queue.dequeue().unwrap().unwrap();
        queue.ack(&id).unwrap();
        assert!(matches!(queue.ack(&id), Err(QueueError::Unknown(_))));
    }

    #[test]
    fn memory_max_depth_rejects_overfill() {
        let mut queue = MemoryQueue::new();
        assert_eq!(queue.max_depth(), None);
        queue.set_max_depth(Some(1));

        let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();
        assert!(matches!(
            queue.enqueue(b"MSH", HashMap::new()),
            Err(QueueError::Full { limit: 1 })
        ));

        // Settling frees a slot.
        queue.dequeue().unwrap().unwrap();
        queue.ack(&id).unwrap();
        queue.enqueue(b"MSH", HashMap::new()).unwrap();
    }

    #[test]
    fn memory_dead_letters_do_not_count_against_cap() {
        let clock = Arc::new(ManualClock::new(0));
        let policy = RetryPolicy {
            max_attempts: 1,
            ..test_policy()
        };
        let mut queue = MemoryQueue::with_policy_and_clock(policy, clock);
        queue.set_max_depth(Some(1));

        let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();
        queue.dequeue().unwrap().unwrap();
        assert_eq!(
            queue.nack(&id, Some("e")).unwrap(),
            NackOutcome::DeadLettered
        );
        // Dead-lettered: a new message fits despite the dead bin holding one.
        queue.enqueue(b"MSH2", HashMap::new()).unwrap();
        assert_eq!(queue.dead_letters().unwrap().len(), 1);
    }

    #[test]
    fn memory_expired_lease_redelivers() {
        let clock = Arc::new(ManualClock::new(0));
        let mut queue = MemoryQueue::with_policy_and_clock(test_policy(), clock.clone());
        let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();

        queue.dequeue().unwrap().unwrap();
        assert!(queue.dequeue().unwrap().is_none()); // leased away
        clock.advance(Duration::from_secs(6)); // past the 5s lease

        let redelivered = queue.dequeue().unwrap().unwrap();
        assert_eq!(redelivered.id, id);
        assert_eq!(redelivered.attempts, 2);
    }

    #[test]
    fn enqueue_message_round_trips_hl7_bytes() {
        let mut queue = MemoryQueue::new();
        let message = vyna_hl7_parser::parse_message(
            "MSH|^~\\&|A|B|C|D|20240101||ADT^A01|1|P|2.5\rPID|1||7\r",
        )
        .unwrap();
        let id = queue.enqueue_message(&message).unwrap();

        let leased = queue.dequeue().unwrap().unwrap();
        assert_eq!(leased.id, id);
        let text = String::from_utf8(leased.body).unwrap();
        let reparsed = vyna_hl7_parser::parse_message(&text).unwrap();
        assert_eq!(reparsed, message);
    }

    static TEST_DIR_COUNTER: AtomicU64 = AtomicU64::new(0);

    /// Unique scratch directory per test (cleaned up by the caller).
    fn scratch_dir() -> PathBuf {
        let seq = TEST_DIR_COUNTER.fetch_add(1, Ordering::SeqCst);
        std::env::temp_dir().join(format!("vyna-queue-test-{}-{seq}", std::process::id()))
    }

    #[test]
    fn two_handles_on_one_dir_mint_unique_ids() {
        let dir = scratch_dir();
        let (mut first, _) = FileQueue::open(&dir).unwrap();
        let (mut second, _) = FileQueue::open(&dir).unwrap();
        let one = first.enqueue(b"one", HashMap::new()).unwrap();
        let two = second.enqueue(b"two", HashMap::new()).unwrap();
        // Same-process handles share one directory: ids must differ even
        // when the clock does not advance between the two enqueues.
        // Per-handle counters both start at zero, so the id sequence
        // space itself must be process-wide, not per handle.
        let seq = |id: &str| id.rsplit('-').next().unwrap_or("").to_string();
        assert_ne!(seq(&one), seq(&two), "sequence space must be process-wide");
        assert_ne!(one, two);
        drop(first);
        drop(second);

        // Both bodies survive: no overwrite of one entry by the other.
        let (mut queue, _) = FileQueue::open(&dir).unwrap();
        let mut bodies = Vec::new();
        while let Some(leased) = queue.dequeue().unwrap() {
            bodies.push(leased.body.clone());
            queue.ack(&leased.id).unwrap();
        }
        bodies.sort();
        assert_eq!(bodies, vec![b"one".to_vec(), b"two".to_vec()]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_persists_across_reopen() {
        let dir = scratch_dir();
        let id = {
            let (mut queue, report) = FileQueue::open(&dir).unwrap();
            assert_eq!(report, RecoveryReport::default());
            queue.enqueue(b"MSH", headers()).unwrap()
        };
        let (mut queue, report) = FileQueue::open(&dir).unwrap();
        assert_eq!(report.pending, 1);

        let leased = queue.dequeue().unwrap().unwrap();
        assert_eq!(leased.id, id);
        assert_eq!(leased.headers, headers());
        queue.ack(&id).unwrap();
        assert_eq!(
            queue.stats().unwrap(),
            QueueStats {
                pending: 0,
                inflight: 0,
                dead: 0
            }
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_recovers_unacked_leases_after_restart() {
        let dir = scratch_dir();
        let clock = Arc::new(ManualClock::new(0));
        let id = {
            let (mut queue, _) =
                FileQueue::open_with_policy_and_clock(&dir, test_policy(), clock.clone()).unwrap();
            let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();
            queue.dequeue().unwrap().unwrap(); // leased, never acked: simulated crash
            id
        };
        // Restart after the 5s lease expired.
        clock.advance(Duration::from_secs(6));
        let (mut queue, report) =
            FileQueue::open_with_policy_and_clock(&dir, test_policy(), clock.clone()).unwrap();
        assert_eq!(report.requeued_stale, 1);

        let redelivered = queue.dequeue().unwrap().unwrap();
        assert_eq!(redelivered.id, id);
        assert_eq!(redelivered.attempts, 2);
        queue.ack(&id).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_backoff_survives_reopen_and_dead_letters_persist() {
        let dir = scratch_dir();
        let clock = Arc::new(ManualClock::new(0));
        let id = {
            let (mut queue, _) =
                FileQueue::open_with_policy_and_clock(&dir, test_policy(), clock.clone()).unwrap();
            let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();
            queue.dequeue().unwrap().unwrap();
            queue.nack(&id, Some("e1")).unwrap(); // visible at t=1s
            id
        };
        // Reopen at t=0: still backing off.
        let (mut queue, _) =
            FileQueue::open_with_policy_and_clock(&dir, test_policy(), clock.clone()).unwrap();
        assert!(queue.dequeue().unwrap().is_none());
        clock.advance(Duration::from_secs(1));
        queue.dequeue().unwrap().unwrap();
        queue.nack(&id, Some("e2")).unwrap();
        clock.advance(Duration::from_secs(2));
        queue.dequeue().unwrap().unwrap();
        assert_eq!(
            queue.nack(&id, Some("e3")).unwrap(),
            NackOutcome::DeadLettered
        );

        let dead = queue.dead_letters().unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].last_error.as_deref(), Some("e3"));

        // Dead letters survive reopening too.
        drop(queue);
        let (queue, report) = FileQueue::open(&dir).unwrap();
        assert_eq!(report.dead, 1);
        assert_eq!(queue.dead_letters().unwrap().len(), 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_max_depth_rejects_overfill() {
        let dir = scratch_dir();
        let (mut queue, _) = FileQueue::open(&dir).unwrap();
        queue.set_max_depth(Some(1));

        let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();
        assert!(matches!(
            queue.enqueue(b"MSH", HashMap::new()),
            Err(QueueError::Full { limit: 1 })
        ));

        queue.dequeue().unwrap().unwrap();
        queue.ack(&id).unwrap();
        queue.enqueue(b"MSH", HashMap::new()).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_dequeue_skips_entries_with_missing_bodies() {
        let dir = scratch_dir();
        let (mut queue, _) = FileQueue::open(&dir).unwrap();
        let first = queue.enqueue(b"first", HashMap::new()).unwrap();
        let second = queue.enqueue(b"second", HashMap::new()).unwrap();

        // Simulate a crash leftover: the oldest entry's body goes missing.
        std::fs::remove_file(dir.join("pending").join(format!("{first}.body"))).unwrap();

        // One entry lost its body; the survivor still dequeues.
        let leased = queue.dequeue().unwrap().unwrap();
        assert_eq!(leased.body, b"second");
        assert_eq!(leased.id, second);
        queue.ack(&second).unwrap();
        assert!(queue.dequeue().unwrap().is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_quarantines_corrupt_entries() {
        let dir = scratch_dir();
        {
            let (mut queue, _) = FileQueue::open(&dir).unwrap();
            queue.enqueue(b"MSH", HashMap::new()).unwrap();
        }
        // Corrupt the pending metadata in place.
        let mut metas: Vec<_> = std::fs::read_dir(dir.join("pending"))
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
            .collect();
        assert_eq!(metas.len(), 1);
        std::fs::write(metas.pop().unwrap(), b"{not valid json").unwrap();

        let (queue, report) = FileQueue::open(&dir).unwrap();
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.pending, 0);
        assert_eq!(queue.stats().unwrap().dead, 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_quarantined_corrupt_meta_visible_in_dead_letters() {
        let dir = scratch_dir();
        let id = {
            let (mut queue, _) = FileQueue::open(&dir).unwrap();
            queue.enqueue(b"MSH-quarantined", HashMap::new()).unwrap()
        };
        // Corrupt the pending metadata in place, then reopen.
        let meta = dir.join("pending").join(format!("{id}.meta.json"));
        std::fs::write(&meta, b"{not valid json").unwrap();

        let (queue, _) = FileQueue::open(&dir).unwrap();
        assert_eq!(queue.stats().unwrap().dead, 1);
        // The quarantined entry must be inspectable, not just counted.
        let dead = queue.dead_letters().unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].id, id);
        assert_eq!(dead[0].body, b"MSH-quarantined");
        assert_eq!(
            dead[0].last_error.as_deref(),
            Some("corrupt metadata quarantined")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_dequeue_path_quarantine_label_claims_no_reopen() {
        let dir = scratch_dir();
        let (mut queue, _) = FileQueue::open(&dir).unwrap();
        let id = queue.enqueue(b"MSH-dequeue-path", HashMap::new()).unwrap();
        // Corrupt the pending metadata in place, then dequeue on the SAME
        // handle: the quarantine happens on the dequeue path with no
        // reopen, so the label must not claim "at open".
        let meta = dir.join("pending").join(format!("{id}.meta.json"));
        std::fs::write(&meta, b"{not valid json").unwrap();
        assert!(queue.dequeue().unwrap().is_none());

        let dead = queue.dead_letters().unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].id, id);
        assert_eq!(
            dead[0].last_error.as_deref(),
            Some("corrupt metadata quarantined")
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_dead_entry_with_deleted_body_visible_in_dead_letters() {
        let dir = scratch_dir();
        let policy = RetryPolicy {
            max_attempts: 1,
            ..test_policy()
        };
        let id = {
            let (mut queue, _) = FileQueue::open_with_policy(&dir, policy).unwrap();
            let id = queue.enqueue(b"MSH-doomed", HashMap::new()).unwrap();
            queue.dequeue().unwrap().unwrap();
            queue.nack(&id, Some("boom")).unwrap();
            id
        };
        // The body file goes missing after dead-lettering (disk fault,
        // operator error): the entry must still surface, with its error.
        std::fs::remove_file(dir.join("dead").join(format!("{id}.body"))).unwrap();

        let (queue, _) = FileQueue::open(&dir).unwrap();
        assert_eq!(queue.stats().unwrap().dead, 1);
        let dead = queue.dead_letters().unwrap();
        assert_eq!(dead.len(), 1);
        assert_eq!(dead[0].id, id);
        assert!(dead[0].body.is_empty());
        assert_eq!(dead[0].last_error.as_deref(), Some("boom"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_corrupt_inflight_reported_once() {
        let dir = scratch_dir();
        let id = {
            let (mut queue, _) = FileQueue::open(&dir).unwrap();
            let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();
            queue.dequeue().unwrap().unwrap();
            id
        };
        let meta = dir.join("inflight").join(format!("{id}.meta.json"));
        std::fs::write(&meta, b"{not valid json").unwrap();

        let (queue, report) = FileQueue::open(&dir).unwrap();
        assert_eq!(report.skipped, vec![id]);
        assert_eq!(report.pending, 0);
        assert_eq!(queue.stats().unwrap().dead, 1);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_bodyless_pending_quarantines_to_dead() {
        let dir = scratch_dir();
        let first = {
            let (mut queue, _) = FileQueue::open(&dir).unwrap();
            let first = queue.enqueue(b"first", HashMap::new()).unwrap();
            queue.enqueue(b"second", HashMap::new()).unwrap();
            first
        };
        std::fs::remove_file(dir.join("pending").join(format!("{first}.body"))).unwrap();

        let (mut queue, report) = FileQueue::open(&dir).unwrap();
        assert_eq!(report.skipped.len(), 1);
        assert!(report.skipped[0].contains(&first));
        assert_eq!(report.pending, 1);
        assert_eq!(report.dead, 1);

        // The survivor still dequeues; the bodyless entry never reappears.
        let leased = queue.dequeue().unwrap().unwrap();
        assert_eq!(leased.body, b"second");
        queue.ack(&leased.id).unwrap();
        assert!(queue.dequeue().unwrap().is_none());
        assert_eq!(
            queue.stats().unwrap(),
            QueueStats {
                pending: 0,
                inflight: 0,
                dead: 1
            }
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_ack_rejects_path_traversal_ids() {
        let dir = scratch_dir();
        let (mut queue, _) = FileQueue::open(&dir).unwrap();
        let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();

        // Never leased: still pending. Hostile ids must not settle it.
        for hostile in [
            format!("../pending/{id}"),
            "../inflight/anything".to_string(),
            "a/b".to_string(),
            "a\\b".to_string(),
            String::new(),
        ] {
            assert!(
                matches!(queue.ack(&hostile), Err(QueueError::Unknown(_))),
                "ack({hostile:?}) must be rejected"
            );
        }
        assert_eq!(queue.stats().unwrap().pending, 1);

        // The entry is intact and still deliverable.
        let leased = queue.dequeue().unwrap().unwrap();
        assert_eq!(leased.id, id);
        assert_eq!(leased.body, b"MSH");
        queue.ack(&id).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_open_reaps_crash_temp_files() {
        let dir = scratch_dir();
        let id = {
            let (mut queue, _) = FileQueue::open(&dir).unwrap();
            queue.enqueue(b"MSH", HashMap::new()).unwrap()
        };
        // Simulate crashes between temp write and rename in every state dir.
        for sub in ["pending", "inflight", "dead"] {
            std::fs::write(dir.join(sub).join(".tmp-123-abc"), b"partial").unwrap();
        }
        let (mut queue, _) = FileQueue::open(&dir).unwrap();
        for sub in ["pending", "inflight", "dead"] {
            let leftovers: Vec<_> = std::fs::read_dir(dir.join(sub))
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect();
            assert!(
                !leftovers
                    .iter()
                    .any(|n| n.to_string_lossy().starts_with(".tmp-")),
                "{sub} still holds crash temp files: {leftovers:?}"
            );
        }
        // Real entries survive the reaping.
        let leased = queue.dequeue().unwrap().unwrap();
        assert_eq!(leased.id, id);
        queue.ack(&id).unwrap();
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn concurrent_handles_never_lease_the_same_message() {
        use std::sync::Barrier;

        let dir = scratch_dir();
        let (mut seeder, _) = FileQueue::open(&dir).unwrap();
        // Barrier-synced dequeues from two same-process handles: without
        // an atomic claim both routinely win the next_visible/move_entry
        // race and lease the same id (the window spans dozens of syscalls,
        // so even a few rounds collide without the lock).
        //
        // Infinite lease on the racing handles: with the default 30s
        // real-clock lease, a loaded machine can stall the loser's claim
        // past the winner's deadline, and recovery then legitimately
        // re-leases the same id to the loser. The race under test is the
        // atomic claim, not the clock, so expiry is out of scope here
        // (see expired_lease_is_released_to_the_next_dequeue).
        let policy = RetryPolicy {
            lease: Duration::MAX,
            ..RetryPolicy::default()
        };
        for _ in 0..25 {
            let id = seeder.enqueue(b"MSH", HashMap::new()).unwrap();
            let barrier = Barrier::new(2);
            let (a, b) = std::thread::scope(|s| {
                let first = s.spawn(|| {
                    let (mut queue, _) = FileQueue::open_with_policy(&dir, policy.clone()).unwrap();
                    barrier.wait();
                    queue.dequeue().unwrap().map(|leased| leased.id)
                });
                let second = s.spawn(|| {
                    let (mut queue, _) = FileQueue::open_with_policy(&dir, policy.clone()).unwrap();
                    barrier.wait();
                    queue.dequeue().unwrap().map(|leased| leased.id)
                });
                (first.join().unwrap(), second.join().unwrap())
            });
            assert!(
                !(a.is_some() && a == b),
                "both handles leased {id}: {a:?} vs {b:?}"
            );
            assert!(a.is_some() || b.is_some(), "message {id} leased by none");
            // Settle the single outstanding lease for the next round.
            let (mut queue, _) = FileQueue::open(&dir).unwrap();
            if queue.ack(&id).is_err() {
                let leased = queue.dequeue().unwrap().expect("lease visible");
                queue.ack(&leased.id).unwrap();
            }
        }
        drop(seeder);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn expired_lease_is_released_to_the_next_dequeue() {
        // Pins the intended expiry path behind the race above: a lease
        // past its deadline returns to pending and is leased again with
        // attempts incremented, instead of being lost. No redelivery
        // while the lease is held.
        let clock = Arc::new(ManualClock::new(1_000));
        let policy = RetryPolicy {
            lease: Duration::from_secs(30),
            ..RetryPolicy::default()
        };
        let dir = scratch_dir();
        let (mut queue, _) =
            FileQueue::open_with_policy_and_clock(&dir, policy, clock.clone()).unwrap();
        let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();
        let first = queue.dequeue().unwrap().unwrap();
        assert_eq!(first.id, id);
        assert_eq!(first.attempts, 1);
        clock.advance(Duration::from_secs(29));
        assert!(queue.dequeue().unwrap().is_none());
        clock.advance(Duration::from_secs(2));
        let second = queue.dequeue().unwrap().unwrap();
        assert_eq!(second.id, id);
        assert_eq!(second.attempts, 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_open_reaps_orphan_bodies() {
        let dir = scratch_dir();
        let (queue, _) = FileQueue::open(&dir).unwrap();
        drop(queue);
        // Crash leftover: a body file with no metadata sibling.
        std::fs::write(dir.join("pending").join("stray.body"), b"orphan").unwrap();

        let (mut queue, _) = FileQueue::open(&dir).unwrap();
        assert!(
            !dir.join("pending").join("stray.body").exists(),
            "orphan body persists across open"
        );
        assert!(queue.dequeue().unwrap().is_none());
        assert_eq!(
            queue.stats().unwrap(),
            QueueStats {
                pending: 0,
                inflight: 0,
                dead: 0
            }
        );
        drop(queue);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn concurrent_enqueue_and_dequeue_loses_nothing() {
        use std::sync::Barrier;
        use std::sync::atomic::AtomicBool;

        let dir = scratch_dir();
        // Pre-seed so the dequeuer has work from the first barrier release.
        let seeds: Vec<String> = (0..20).map(|i| format!("seed-{i:02}")).collect();
        {
            let (mut seeder, _) = FileQueue::open(&dir).unwrap();
            for body in &seeds {
                seeder.enqueue(body.as_bytes(), HashMap::new()).unwrap();
            }
        }

        let n = 200usize;
        let seed_count = seeds.len();
        let barrier = Barrier::new(2);
        let enq_done = std::sync::Arc::new(AtomicBool::new(false));
        let (dir_d, bar_d) = (&dir, &barrier);
        let done_d = enq_done.clone();
        std::thread::scope(|s| {
            let producer = s.spawn(|| {
                let (mut queue, _) = FileQueue::open(&dir).unwrap();
                barrier.wait();
                for i in 0..n {
                    let body = format!("msg-{i:04}");
                    queue.enqueue(body.as_bytes(), HashMap::new()).unwrap();
                }
            });
            let consumer = s.spawn(move || {
                let (mut queue, _) = FileQueue::open(dir_d).unwrap();
                bar_d.wait();
                let mut got = std::collections::HashSet::new();
                let mut empty_streak = 0u32;
                while got.len() < n + seed_count {
                    match queue.dequeue().unwrap() {
                        Some(leased) => {
                            empty_streak = 0;
                            got.insert(String::from_utf8(leased.body.clone()).unwrap());
                            queue.ack(&leased.id).unwrap();
                        }
                        None if done_d.load(Ordering::SeqCst) => {
                            empty_streak += 1;
                            if empty_streak >= 50 {
                                break;
                            }
                            std::thread::sleep(std::time::Duration::from_millis(1));
                        }
                        None => std::thread::sleep(std::time::Duration::from_millis(1)),
                    }
                }
                got
            });
            producer.join().unwrap();
            enq_done.store(true, Ordering::SeqCst);
            let got = consumer.join().unwrap();

            // At-least-once: every enqueued body arrives exactly once.
            assert_eq!(got.len(), n + seed_count, "lost messages under concurrency");
            for i in 0..n {
                assert!(got.contains(&format!("msg-{i:04}")), "lost msg-{i:04}");
            }
            for body in &seeds {
                assert!(got.contains(body), "lost {body}");
            }
        });

        // Nothing quarantined, nothing left behind.
        let (queue, _) = FileQueue::open(&dir).unwrap();
        assert_eq!(
            queue.stats().unwrap(),
            QueueStats {
                pending: 0,
                inflight: 0,
                dead: 0
            }
        );
        assert!(queue.dead_letters().unwrap().is_empty());
        for entry in std::fs::read_dir(dir.join("dead")).unwrap() {
            let name = entry.unwrap().file_name().to_string_lossy().into_owned();
            if let Some(id) = name.strip_suffix(".meta.json") {
                assert!(
                    dir.join("dead").join(format!("{id}.body")).exists(),
                    "bodyless dead meta {id}"
                );
            }
        }
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn recovery_skipped_uses_uniform_bare_ids() {
        let dir = scratch_dir();
        {
            let (mut queue, _) = FileQueue::open(&dir).unwrap();
            queue.enqueue(b"A", HashMap::new()).unwrap();
            queue.enqueue(b"B", HashMap::new()).unwrap();
            queue.dequeue().unwrap().unwrap(); // one leased goes inflight
        }
        // Corrupt one pending meta and one inflight meta.
        for sub in ["pending", "inflight"] {
            for entry in std::fs::read_dir(dir.join(sub)).unwrap() {
                let path = entry.unwrap().path();
                if path.extension().and_then(|e| e.to_str()) == Some("json") {
                    std::fs::write(&path, b"{not valid json").unwrap();
                }
            }
        }
        let (queue, report) = FileQueue::open(&dir).unwrap();
        assert_eq!(report.skipped.len(), 2);
        for entry in &report.skipped {
            assert!(!entry.contains('/'), "mixed skipped format: {entry}");
        }
        assert_eq!(queue.stats().unwrap().dead, 2);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_stats_never_double_counts_under_concurrent_moves() {
        use std::sync::atomic::AtomicBool;

        let dir = scratch_dir();
        let policy = RetryPolicy {
            max_attempts: u32::MAX,
            base_delay: Duration::ZERO,
            max_delay: Duration::ZERO,
            lease: Duration::from_secs(30),
        };
        {
            let (mut seeder, _) = FileQueue::open_with_policy(&dir, policy.clone()).unwrap();
            seeder.enqueue(b"MSH", HashMap::new()).unwrap();
        }
        // One live message cycling pending -> inflight -> pending: the
        // total across states is always exactly 1. move_entry stages
        // dest-before-source-removal, so an unlocked stats() can catch
        // both copies and report 2.
        let stop = Arc::new(AtomicBool::new(false));
        std::thread::scope(|s| {
            let mover = s.spawn(|| {
                let (mut queue, _) = FileQueue::open_with_policy(&dir, policy).unwrap();
                for _ in 0..300 {
                    let leased = queue.dequeue().unwrap().expect("message visible");
                    assert!(matches!(
                        queue.nack(&leased.id, None).unwrap(),
                        NackOutcome::Requeued { .. }
                    ));
                }
            });
            let sampler = s.spawn(|| {
                let (queue, _) = FileQueue::open(&dir).unwrap();
                let mut worst = 0;
                while !stop.load(Ordering::SeqCst) {
                    let stats = queue.stats().unwrap();
                    worst = worst.max(stats.pending + stats.inflight + stats.dead);
                    // Nothing dead-letters here; dead_letters must stay
                    // empty (and race-free) throughout the movement.
                    assert!(queue.dead_letters().unwrap().is_empty());
                }
                worst
            });
            mover.join().unwrap();
            stop.store(true, Ordering::SeqCst);
            let worst = sampler.join().unwrap();
            assert_eq!(worst, 1, "stats() double-counted a moving message");
        });
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn file_nack_rejects_path_traversal_ids() {
        let dir = scratch_dir();
        let (mut queue, _) = FileQueue::open(&dir).unwrap();
        let id = queue.enqueue(b"MSH", HashMap::new()).unwrap();

        for hostile in [
            format!("../pending/{id}"),
            "../dead/anything".to_string(),
            "a/b".to_string(),
        ] {
            assert!(
                matches!(queue.nack(&hostile, Some("e")), Err(QueueError::Unknown(_))),
                "nack({hostile:?}) must be rejected"
            );
        }
        assert_eq!(queue.stats().unwrap().pending, 1);
        let leased = queue.dequeue().unwrap().unwrap();
        assert_eq!(leased.id, id);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
