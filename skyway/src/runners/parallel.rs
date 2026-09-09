//! Parallel execution of a [`PipelinePlan`] using Rayon.
//!
//! The runner owns every execution detail the plan leaves open: how many
//! workers decode and filter chunks, how many chunks may be in flight between
//! stages, and where sequence restoration happens. The plan's decisions are
//! executed as given.
//!
//! Back-pressure comes from an admission gate: a worker may hand chunk `i` to
//! the sequencer only while fewer than `max_in_flight_chunks` chunks from the
//! next-needed one are outstanding. This is deadlock-free because readers
//! pull chunk indices in ascending order (see [`Reader`]), so the chunk the
//! sequencer is waiting for is always already held by a worker that the gate
//! admits immediately.

use rayon::{ThreadPool, ThreadPoolBuilder, prelude::*};

use std::{
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex, PoisonError,
        mpsc::{Receiver, Sender, SyncSender, channel, sync_channel},
    },
    thread::{self, JoinHandle},
};

use crate::{
    SkywayError,
    chunks::{ChunkBuilder, ElementChunk, OrderedChunkIterator},
    elements::{Element, Metadata},
    plan::{FilterPlan, OrderPlan, PipelinePlan},
    readers::Reader,
    sort::{chunk_elements, sort_elements},
};

#[cfg(feature = "filter")]
use crate::{
    filter::{
        CompiledFilters, Discovery, ElementFilter, discover_chunk, emit_chunk, filter_chunk,
        reference_closure,
    },
    plan::ReplayPlan,
};

#[cfg(feature = "filter")]
use super::spool::{SpoolReader, SpoolWriter, decode_chunk, encode_chunk};

/// Execution options specific to the parallel runner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParallelOptions {
    /// Number of worker threads. `None` uses Rayon's default (one per CPU).
    pub workers: Option<usize>,
    /// Maximum number of chunks buffered between pipeline stages, counted
    /// from the next chunk the sequencer needs. On top of this, each worker
    /// may hold one decoded chunk while it waits for admission.
    pub max_in_flight_chunks: usize,
}

impl Default for ParallelOptions {
    fn default() -> Self {
        ParallelOptions {
            workers: None,
            max_in_flight_chunks: 32,
        }
    }
}

impl ParallelOptions {
    /// Describe how this runner will execute `plan`.
    pub fn explain(&self, plan: &PipelinePlan) -> String {
        let mut lines = vec!["Execution: parallel".to_string()];

        lines.push(match self.workers {
            Some(n) => format!("  {n} worker threads"),
            None => "  One worker thread per CPU".to_string(),
        });
        lines.push(format!(
            "  Up to {} chunks buffered between stages, plus one in progress per worker",
            self.max_in_flight_chunks
        ));

        if plan.replays_input() {
            lines.push("  Discovery summaries are merged without sequence restoration".to_string());
        }

        match plan.ordering {
            OrderPlan::PreserveSequence => {
                lines.push("  Emission restores chunk sequence".to_string());
                lines.push("  No global element sort".to_string());
            }
            OrderPlan::Sort(_) => {
                lines.push("  All elements are collected and sorted before writing".to_string());
            }
        }

        lines.join("\n")
    }
}

/// Handle to the background work of a running pipeline.
pub struct PipelineWorker(JoinHandle<Result<(), SkywayError>>);

impl PipelineWorker {
    /// Wait for the pipeline to finish and report any error it hit.
    ///
    /// Call this after the writer has consumed the element stream; joining
    /// earlier would wait for output nobody is reading.
    pub fn finish(self) -> Result<(), SkywayError> {
        join(self.0)
    }
}

/// The output of a running pipeline.
pub struct PipelineOutput {
    pub elements: Receiver<ElementChunk>,
    pub metadata: Receiver<Metadata>,
    pub worker: PipelineWorker,
}

fn join(handle: JoinHandle<Result<(), SkywayError>>) -> Result<(), SkywayError> {
    handle.join().unwrap_or_else(|_| {
        Err(SkywayError::UnexpectedError(
            "A pipeline thread panicked".to_string(),
        ))
    })
}

fn consumer_gone() -> SkywayError {
    SkywayError::UnexpectedError("The next pipeline stage stopped accepting chunks".to_string())
}

fn transform_metadata(
    metadata_receiver: Receiver<Metadata>,
    metadata_sender: Sender<Metadata>,
    preserve_generator: bool,
) {
    // All readers send one Metadata value, including formats without metadata.
    // If the reader failed before sending any, there is nothing to forward.
    let Some(mut metadata) = metadata_receiver.into_iter().next() else {
        return;
    };

    if !preserve_generator {
        metadata.generator = Some(format!("skyway v{}", env!("CARGO_PKG_VERSION")))
    }

    // The receiver may already be gone if the writer does not use metadata.
    let _ = metadata_sender.send(metadata);
}

fn build_pool(workers: Option<usize>) -> Result<Option<Arc<ThreadPool>>, SkywayError> {
    workers
        .map(|n| {
            ThreadPoolBuilder::new()
                .num_threads(n)
                .build()
                .map(Arc::new)
                .map_err(|e| {
                    SkywayError::UnexpectedError(format!("Unable to create thread pool: {e}"))
                })
        })
        .transpose()
}

fn in_pool<T: Send>(pool: Option<&ThreadPool>, f: impl FnOnce() -> T + Send) -> T {
    match pool {
        Some(pool) => pool.install(f),
        None => f(),
    }
}

/// Admission gate bounding the number of chunks outstanding between the
/// workers and the sequencer.
///
/// Chunk `index` is admitted once fewer than `limit` chunks ahead of the
/// sequencer's next-needed chunk are outstanding, that is, while
/// `index < emitted + limit`. The sequencer advances `emitted` as it hands
/// chunks downstream, and closes the gate when it stops consuming.
struct Gate {
    state: Mutex<GateState>,
    ready: Condvar,
    limit: usize,
}

struct GateState {
    emitted: usize,
    closed: bool,
}

impl Gate {
    fn new(limit: usize) -> Arc<Self> {
        Arc::new(Gate {
            state: Mutex::new(GateState {
                emitted: 0,
                closed: false,
            }),
            ready: Condvar::new(),
            limit: limit.max(1),
        })
    }

    /// Wait until chunk `index` may be forwarded to the sequencer.
    ///
    /// This parks a Rayon worker (a global-pool thread when no dedicated
    /// pool was built), the same hazard class as the blocking channel send in
    /// [`forward`]. Writers do not use Rayon, so no other pool user can be
    /// starved. It never deadlocks as long as readers honour the ascending
    /// pull order documented on [`Reader`]: the chunk the sequencer needs
    /// next is always held by a worker that this gate admits at once.
    ///
    /// Returns an error once the gate has been closed, so that parked
    /// workers wake up and stop when the pipeline is shutting down.
    fn admit(&self, index: usize) -> Result<(), SkywayError> {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        while !state.closed && index >= state.emitted + self.limit {
            state = self
                .ready
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
        if state.closed {
            Err(consumer_gone())
        } else {
            Ok(())
        }
    }

    /// Record that the sequencer handed one more chunk downstream.
    fn advance(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.emitted += 1;
        drop(state);
        self.ready.notify_all();
    }

    /// Stop admitting chunks and wake every parked worker. Idempotent.
    fn close(&self) {
        let mut state = self.state.lock().unwrap_or_else(PoisonError::into_inner);
        state.closed = true;
        drop(state);
        self.ready.notify_all();
    }
}

/// Closes the gate if the worker holding it unwinds from a panic, so that
/// sibling workers parked in [`Gate::admit`] do not wait forever.
struct GateFuse(Arc<Gate>);

impl Drop for GateFuse {
    fn drop(&mut self) {
        if thread::panicking() {
            self.0.close();
        }
    }
}

/// Close the gate when a worker-side step failed, so parked siblings stop.
fn abort<T>(gate: &Gate, result: Result<T, SkywayError>) -> Result<T, SkywayError> {
    if result.is_err() {
        gate.close();
    }
    result
}

/// A stage producing filtered chunks in arbitrary order on a background thread.
struct Producer {
    receiver: Receiver<ElementChunk>,
    handle: JoinHandle<Result<(), SkywayError>>,
}

fn spawn_producer<F>(pool: Option<Arc<ThreadPool>>, gate: Arc<Gate>, produce: F) -> Producer
where
    F: FnOnce(&Arc<Gate>, &SyncSender<ElementChunk>) -> Result<(), SkywayError> + Send + 'static,
{
    // Admitted but unemitted chunks never exceed the gate's limit, so this
    // send never blocks; the bound is belt and braces.
    let (sender, receiver) = sync_channel(gate.limit);
    let handle = thread::spawn(move || in_pool(pool.as_deref(), || produce(&gate, &sender)));
    Producer { receiver, handle }
}

fn forward(
    gate: &Gate,
    sender: &SyncSender<ElementChunk>,
    chunk: ElementChunk,
) -> Result<(), SkywayError> {
    abort(gate, sender.send(chunk).map_err(|_| consumer_gone()))
}

/// Start the decode-and-filter stage described by `plan.filtering`.
#[allow(clippy::too_many_arguments)]
fn start_filtering<R: Reader>(
    reader: R,
    source: Option<PathBuf>,
    chunk_builder: ChunkBuilder,
    metadata_sender: Sender<Metadata>,
    #[cfg(feature = "filter")] program: Arc<CompiledFilters>,
    filtering: FilterPlan,
    pool: Option<Arc<ThreadPool>>,
    gate: Arc<Gate>,
) -> Result<Producer, SkywayError> {
    match filtering {
        FilterPlan::None => Ok(spawn_producer(pool, gate, move |gate, sender| {
            reader
                .read_file(source, metadata_sender, chunk_builder)
                .map_init(
                    || GateFuse(gate.clone()),
                    |fuse, chunk| fuse.0.admit(chunk.index).map(|_| chunk),
                )
                .try_for_each(|chunk| forward(gate, sender, chunk?))
        })),

        #[cfg(feature = "filter")]
        FilterPlan::OnePass => Ok(spawn_producer(pool, gate, move |gate, sender| {
            reader
                .read_file(source, metadata_sender, chunk_builder)
                .map(|chunk| filter_chunk(chunk, program.as_ref()))
                .map_init(
                    || GateFuse(gate.clone()),
                    |fuse, chunk| fuse.0.admit(chunk.index).map(|_| chunk),
                )
                .try_for_each(|chunk| forward(gate, sender, chunk?))
        })),

        #[cfg(feature = "filter")]
        FilterPlan::ReferenceClosure {
            replay: ReplayPlan::ReopenSource,
        } => {
            // Discovery: read once, evaluate every element, merge per-chunk
            // summaries. No sequence restoration is needed here, so the gate
            // is not involved.
            let discovery_reader = reader.clone();
            let discovery_source = source.clone();
            let discovery_program = program.clone();
            let discovery = in_pool(pool.as_deref(), move || {
                discovery_reader
                    .read_file(discovery_source, metadata_sender, chunk_builder)
                    .map(|chunk| discover_chunk(chunk, discovery_program.as_ref()))
                    .try_reduce(Discovery::new, |a, b| a.try_merge(b))
            })?;
            let keep = Arc::new(reference_closure(discovery));

            // Emission: read again, keep what the closure decided.
            Ok(spawn_producer(pool, gate, move |gate, sender| {
                // The metadata was already forwarded during discovery. The
                // reader still needs a live channel to send into.
                let (unused_sender, unused_receiver) = channel();
                let result = reader
                    .read_file(source, unused_sender, chunk_builder)
                    .map(|chunk| emit_chunk(chunk, &keep, program.as_ref()))
                    .map_init(
                        || GateFuse(gate.clone()),
                        |fuse, chunk| fuse.0.admit(chunk.index).map(|_| chunk),
                    )
                    .try_for_each(|chunk| forward(gate, sender, chunk?));
                drop(unused_receiver);
                result
            }))
        }

        #[cfg(feature = "filter")]
        FilterPlan::ReferenceClosure {
            replay: ReplayPlan::SpoolDecoded,
        } => {
            // Discovery, while spooling every decoded chunk (before filter
            // mutation) so emission can evaluate the same inputs.
            let mut spool = SpoolWriter::new()?;
            let (spool_sender, spool_receiver) = sync_channel::<(usize, Vec<u8>)>(gate.limit);
            let spool_thread = thread::spawn(move || -> Result<SpoolReader, SkywayError> {
                for (index, bytes) in spool_receiver {
                    spool.append(index, &bytes)?;
                }
                Ok(spool.finish()?)
            });

            let discovery = in_pool(pool.as_deref(), || {
                reader
                    .read_file(source, metadata_sender, chunk_builder)
                    .map(|chunk| {
                        let bytes = encode_chunk(&chunk.content);
                        spool_sender
                            .send((chunk.index, bytes))
                            .map_err(|_| consumer_gone())?;
                        discover_chunk(chunk, program.as_ref())
                    })
                    .try_reduce(Discovery::new, |a, b| a.try_merge(b))
            });
            drop(spool_sender);

            let spool_reader = spool_thread.join().unwrap_or_else(|_| {
                Err(SkywayError::UnexpectedError(
                    "The spool writer thread panicked".to_string(),
                ))
            });
            let discovery = discovery?;
            let spool_reader = spool_reader?;
            let keep = Arc::new(reference_closure(discovery));

            // Emission from the spool, which replays chunks in index order.
            Ok(spawn_producer(pool, gate, move |gate, sender| {
                spool_reader
                    .par_bridge()
                    .map_init(
                        || GateFuse(gate.clone()),
                        |fuse, record: std::io::Result<(usize, Vec<u8>)>| {
                            let gate = &fuse.0;
                            abort(gate, (|| -> Result<ElementChunk, SkywayError> {
                                let (index, bytes) = record?;
                                let chunk = decode_chunk(index, &bytes)?;
                                gate.admit(chunk.index)?;
                                Ok(emit_chunk(chunk, &keep, program.as_ref()))
                            })())
                        },
                    )
                    .try_for_each(|chunk| forward(gate, sender, chunk?))
            }))
        }

        #[cfg(not(feature = "filter"))]
        _ => Err(SkywayError::UnexpectedError(
            "Filtering was planned, but skyway was built without filter support".to_string(),
        )),
    }
}

/// Execute `ordering` on the producer's output, sending to `output`.
fn sequence(
    producer: Producer,
    gate: &Gate,
    ordering: OrderPlan,
    chunk_size: usize,
    output: SyncSender<ElementChunk>,
) -> Result<(), SkywayError> {
    let mut ordered = OrderedChunkIterator::tolerant(producer.receiver.iter());

    let result = match ordering {
        OrderPlan::PreserveSequence => {
            let mut result = Ok(());
            for (index, content) in ordered.by_ref().enumerate() {
                if output.send(ElementChunk { index, content }).is_err() {
                    gate.close();
                    result = Err(consumer_gone());
                    break;
                }
                gate.advance();
            }
            result
        }
        OrderPlan::Sort(order) => {
            // Restore the input sequence first so that the stable sort gives
            // deterministic output for elements with equal keys.
            let mut elements: Vec<Element> = Vec::new();
            for content in ordered.by_ref() {
                elements.extend(Vec::from(content));
                gate.advance();
            }
            sort_elements(&mut elements, order);

            chunk_elements(elements, chunk_size)
                .try_for_each(|chunk| output.send(chunk).map_err(|_| consumer_gone()))
        }
    };

    // Whatever happened, no more chunks will be consumed: wake parked workers.
    gate.close();

    let incomplete = ordered.is_incomplete();
    // Dropping the receiver unblocks a producer that may be waiting to send.
    drop(ordered);

    // The producer's error is the root cause if there is one.
    join(producer.handle)?;
    result?;

    if incomplete {
        return Err(SkywayError::UnexpectedError(
            "The chunk sequence ended before every chunk arrived".to_string(),
        ));
    }

    Ok(())
}

/// Run the processing stages shared by every input format.
///
/// The reader remains a concrete type selected by the caller. All reader types
/// produce the same channel-based output, which can then be passed to any
/// writer. The stages run in the background; the returned receivers stream
/// chunks as they become available, and `worker` reports the outcome once the
/// stream has been consumed.
pub fn run_pipeline<R: Reader>(
    reader: R,
    source: Option<PathBuf>,
    chunk_size: usize,
    #[cfg(feature = "filter")] filters: Vec<Box<dyn ElementFilter>>,
    plan: PipelinePlan,
    preserve_generator: bool,
    options: ParallelOptions,
) -> Result<PipelineOutput, SkywayError> {
    let chunk_builder = ChunkBuilder::new(chunk_size);
    let capacity = options.max_in_flight_chunks.max(1);

    let (metadata_sender, metadata_receiver) = channel();
    let (transformed_metadata_sender, transformed_metadata_receiver) = channel();
    thread::spawn(move || {
        transform_metadata(
            metadata_receiver,
            transformed_metadata_sender,
            preserve_generator,
        )
    });

    let (output_sender, output_receiver) = sync_channel(capacity);

    #[cfg(feature = "filter")]
    let program = Arc::new(CompiledFilters::new(filters));

    let handle = thread::spawn(move || -> Result<(), SkywayError> {
        let pool = build_pool(options.workers)?;
        let gate = Gate::new(options.max_in_flight_chunks);

        let producer = start_filtering(
            reader,
            source,
            chunk_builder,
            metadata_sender,
            #[cfg(feature = "filter")]
            program,
            plan.filtering,
            pool,
            gate.clone(),
        )?;

        sequence(producer, &gate, plan.ordering, chunk_size, output_sender)
    });

    Ok(PipelineOutput {
        elements: output_receiver,
        metadata: transformed_metadata_receiver,
        worker: PipelineWorker(handle),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        collections::HashMap,
        sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc::RecvTimeoutError,
        },
        time::{Duration, Instant},
    };

    use crate::{elements::ElementType, plan::Order};

    const WORKERS: usize = 2;
    const LIMIT: usize = 4;

    fn options() -> ParallelOptions {
        ParallelOptions {
            workers: Some(WORKERS),
            max_in_flight_chunks: LIMIT,
        }
    }

    fn plan(ordering: OrderPlan) -> PipelinePlan {
        PipelinePlan {
            filtering: FilterPlan::None,
            ordering,
            filter_reasons: vec![],
            order_reasons: vec![],
        }
    }

    fn node(id: i64) -> Element {
        Element {
            changeset: None,
            user: None,
            version: None,
            uid: None,
            id,
            timestamp: None,
            visible: None,
            tags: HashMap::new(),
            element_type: ElementType::Node { lat: 0, lon: 0 },
        }
    }

    /// Decodes `chunks` chunks of one element each, blocking chunk 0 until
    /// released and counting every decode.
    #[derive(Clone)]
    struct FakeReader {
        chunks: usize,
        decoded: Arc<AtomicUsize>,
        release: Arc<(Mutex<bool>, Condvar)>,
        panic_at: Option<usize>,
    }

    impl FakeReader {
        fn new(chunks: usize) -> Self {
            FakeReader {
                chunks,
                decoded: Arc::new(AtomicUsize::new(0)),
                release: Arc::new((Mutex::new(false), Condvar::new())),
                panic_at: None,
            }
        }

        fn release_chunk_zero(&self) {
            let (released, signal) = &*self.release;
            *released.lock().unwrap() = true;
            signal.notify_all();
        }

        fn decoded(&self) -> usize {
            self.decoded.load(Ordering::SeqCst)
        }

        fn wait_for_decoded(&self, at_least: usize) {
            let deadline = Instant::now() + Duration::from_secs(5);
            while self.decoded() < at_least {
                assert!(Instant::now() < deadline, "workers never decoded {at_least} chunks");
                thread::sleep(Duration::from_millis(5));
            }
        }
    }

    impl Reader for FakeReader {
        fn read_file(
            self,
            _src: Option<PathBuf>,
            metadata_sender: Sender<Metadata>,
            _chunk_builder: ChunkBuilder,
        ) -> impl ParallelIterator<Item = ElementChunk> {
            let _ = metadata_sender.send(Metadata::default());
            (0..self.chunks).par_bridge().map(move |index| {
                self.decoded.fetch_add(1, Ordering::SeqCst);
                if self.panic_at == Some(index) {
                    panic!("fake reader panicked at chunk {index}");
                }
                if index == 0 {
                    let (released, signal) = &*self.release;
                    let mut released = released.lock().unwrap();
                    while !*released {
                        released = signal.wait(released).unwrap();
                    }
                }
                ElementChunk {
                    index,
                    content: vec![node(index as i64)].into_boxed_slice(),
                }
            })
        }
    }

    fn start(reader: &FakeReader, ordering: OrderPlan) -> PipelineOutput {
        run_pipeline(
            reader.clone(),
            None,
            1,
            #[cfg(feature = "filter")]
            Vec::new(),
            plan(ordering),
            false,
            options(),
        )
        .unwrap()
    }

    /// Join the pipeline on a helper thread so that a hang fails the test
    /// instead of stalling it.
    fn finish_within(worker: PipelineWorker, timeout: Duration) -> Result<(), SkywayError> {
        let (sender, receiver) = channel();
        thread::spawn(move || {
            let _ = sender.send(worker.finish());
        });
        match receiver.recv_timeout(timeout) {
            Ok(result) => result,
            Err(RecvTimeoutError::Timeout) => panic!("the pipeline hung"),
            Err(RecvTimeoutError::Disconnected) => panic!("the joining thread died"),
        }
    }

    fn ids(elements: Receiver<ElementChunk>) -> Vec<i64> {
        elements
            .iter()
            .flat_map(|chunk| chunk.into_iter().map(|element| element.id))
            .collect()
    }

    fn assert_bounded_then_complete(ordering: OrderPlan) {
        const CHUNKS: usize = 64;
        let reader = FakeReader::new(CHUNKS);
        let output = start(&reader, ordering);

        // Chunk 0 is stuck, so nothing is emitted and the gate must hold the
        // workers back once `LIMIT` chunks are outstanding.
        reader.wait_for_decoded(LIMIT);
        thread::sleep(Duration::from_millis(100));
        let decoded = reader.decoded();
        assert!(
            decoded <= LIMIT + WORKERS,
            "{decoded} chunks were decoded with only {LIMIT} admitted"
        );

        reader.release_chunk_zero();
        let ids = ids(output.elements);
        finish_within(output.worker, Duration::from_secs(5)).unwrap();

        assert_eq!(ids, (0..CHUNKS as i64).collect::<Vec<_>>());
        assert_eq!(reader.decoded(), CHUNKS);
    }

    #[test]
    fn gate_bounds_decoded_chunks() {
        assert_bounded_then_complete(OrderPlan::PreserveSequence);
    }

    #[test]
    fn sort_path_advances_gate() {
        assert_bounded_then_complete(OrderPlan::Sort(Order::Id));
    }

    #[test]
    fn consumer_gone_wakes_parked_workers() {
        let reader = FakeReader::new(64);
        let output = start(&reader, OrderPlan::PreserveSequence);
        drop(output.elements);

        // Wait until a worker is parked in the gate.
        reader.wait_for_decoded(LIMIT + 1);
        thread::sleep(Duration::from_millis(50));

        reader.release_chunk_zero();
        let result = finish_within(output.worker, Duration::from_secs(5));
        assert!(result.is_err());
    }

    #[test]
    fn worker_panic_does_not_hang() {
        let mut reader = FakeReader::new(64);
        reader.panic_at = Some(3);
        reader.release_chunk_zero();
        let output = start(&reader, OrderPlan::PreserveSequence);

        let ids = ids(output.elements);
        let result = finish_within(output.worker, Duration::from_secs(5));
        assert!(result.is_err());
        assert!(ids.len() < 64);
    }

    #[test]
    fn gate_admits_below_limit_immediately() {
        let gate = Gate::new(3);
        for index in 0..3 {
            gate.admit(index).unwrap();
        }
    }

    #[test]
    fn gate_treats_zero_limit_as_one() {
        let gate = Gate::new(0);
        gate.admit(0).unwrap();
    }

    /// Run `admit` on a helper thread, returning a receiver for its result.
    fn admit_in_background(gate: &Arc<Gate>, index: usize) -> Receiver<Result<(), SkywayError>> {
        let gate = gate.clone();
        let (sender, receiver) = channel();
        thread::spawn(move || {
            let _ = sender.send(gate.admit(index));
        });
        receiver
    }

    #[test]
    fn gate_blocks_until_advanced() {
        let gate = Gate::new(2);
        let blocked = admit_in_background(&gate, 2);
        assert_eq!(
            blocked.recv_timeout(Duration::from_millis(100)).unwrap_err(),
            RecvTimeoutError::Timeout
        );

        gate.advance();
        blocked.recv_timeout(Duration::from_secs(5)).unwrap().unwrap();
    }

    #[test]
    fn gate_close_releases_blocked_and_rejects_later_admits() {
        let gate = Gate::new(1);
        let blocked = admit_in_background(&gate, 5);
        assert_eq!(
            blocked.recv_timeout(Duration::from_millis(100)).unwrap_err(),
            RecvTimeoutError::Timeout
        );

        gate.close();
        assert!(blocked.recv_timeout(Duration::from_secs(5)).unwrap().is_err());
        assert!(gate.admit(0).is_err());
        gate.close();
        assert!(gate.admit(0).is_err());
    }
}
