// Implementation of the repartitionexec operator for shredded records, implemented to partition data calculated via multisemijoin

use core::hash;
use std::num;
use std::task::{Context, Poll};
use std::{pin::Pin, sync::Arc};

use datafusion::arrow::array::{
    ArrayRef, Datum, Int32Array, Int64Array, RecordBatch, UInt32Array, UInt64Array, UInt8Array
};
use datafusion::arrow::compute::kernels::partition;
use datafusion::arrow::compute::take;
use datafusion::common::hash_utils::{self, create_hashes};
use datafusion::execution::memory_pool::MemoryReservation;
use datafusion::physical_plan::repartition::BatchPartitioner;
use datafusion::physical_plan::{Partitioning, PhysicalExpr};
use datafusion::{
    error::DataFusionError,
    execution::{memory_pool::MemoryConsumer, RecordBatchStream, TaskContext},
    physical_plan::{
        metrics::MetricsSet, repartition::RepartitionExec, stream::RecordBatchStreamAdapter,
        ExecutionPlan,
    },
};
use futures::stream::TryFlatten;
use futures::{FutureExt, Stream, StreamExt, TryStreamExt};
use rand::prelude::Distribution;

use crate::yannakakis::multisemijoin::MultiSemiJoinStreamAdapter;
use crate::yannakakis::unnest;
use super::data::{Idx, NestedColumn, NonSingularNestedColumn, SemiJoinResultBatch, SingularNestedColumn};
use super::kernel::take_nested_column_inplace;
use super::multisemijoin::MultiSemiJoinBatchStream;
use super::sel::Sel;
use super::{
    data::{GroupedRelRef, NestedBatch, NestedSchemaRef},
    groupby::GroupBy,
    multisemijoin::{MultiSemiJoin, SendableSemiJoinResultBatchStream},
};
use distributor_channels::{channels, DistributionReceiver, DistributionSender};
use hashbrown::HashMap;
use parking_lot::Mutex;
use spawned_task::SpawnedTask;
// // use datafusion::physical_plan::Partitioning;

use std::fmt::Debug;

mod distributor_channels;
mod spawned_task;

type MaybeNestedBatch = Option<Result<SemiJoinResultBatch, DataFusionError>>;
type InputPartitionsToCurentPartitionSender = Vec<DistributionSender<MaybeNestedBatch>>;
type InputPartitionsToCurrentPartitionReceiver = Vec<DistributionReceiver<MaybeNestedBatch>>;
type SharedMemoryReservation = Arc<Mutex<MemoryReservation>>;

//state of repartition to store channels for each partition, alongside the abort helper
#[derive(Debug)]
struct RepartitionExecState {
    //channel for sending batches from input to output, key = partition number, value are channels
    channels: HashMap<
        usize,
        (
            InputPartitionsToCurentPartitionSender,
            InputPartitionsToCurrentPartitionReceiver,
            SharedMemoryReservation,
        ),
    >,

    debugTester: String,

    abort_helper: Arc<Vec<SpawnedTask<()>>>,
}

impl RepartitionExecState {
    fn new(input: Arc<MultiSemiJoin>, context: Arc<TaskContext>, repartition_key: usize) -> Self {
        let num_input_partitions = input.guard_partition_count();
        let (input_channels, output_channels) = {
            //only the preserve_order = false route has been implemented for now
            let (input_channels, output_channels) = channels(num_input_partitions);
            let input_channels = input_channels
                .into_iter()
                .map(|item| vec![item; num_input_partitions])
                .collect::<Vec<_>>(); //turn into 2D vector
            let output_channels = output_channels
                .into_iter()
                .map(|item| vec![item])
                .collect::<Vec<_>>(); //turn into 2D vector

            (input_channels, output_channels)
        };

        let mut channels = HashMap::with_capacity(input_channels.len()); // init hashmap with amount of partitions
        for (partition, (input_channel, output_channel)) in
            input_channels.into_iter().zip(output_channels).enumerate()
        {
            let reservation = Arc::new(Mutex::new(
                MemoryConsumer::new(format!("{}[{partition}]", "RepartitionExec"))
                    .register(context.memory_pool()),
            ));
            channels.insert(partition, (input_channel, output_channel, reservation));
        }

        //TODO: add metrics

        // goal is to launch 1 task per input partition, these tasks gather input via a helper function and send it to the output channel
        // each task has its own waiter, which is used to wait for the task to finish
        let mut spawned_tasks = Vec::with_capacity(num_input_partitions);
        // eprintln!("{}", num_input_partitions);
        for i in 0..num_input_partitions {
            let channels_in: HashMap<_, _> = channels
                .iter()
                .map(
                    |(partition, (input_channels, _output_channels, reservations))| {
                        (
                            *partition,
                            (input_channels[i].clone(), reservations.clone()),
                        )
                    },
                )
                .collect();

            let input_task = SpawnedTask::spawn(RepartitionMultiSemiJoin::pull_from_input(
                Arc::clone(&input),
                i,
                channels_in.clone(),
                Arc::clone(&context),
                repartition_key,
            ));

            let wait_for_task =
                SpawnedTask::spawn(RepartitionMultiSemiJoin::wait_for_task(input_task,
                channels_in.into_iter().map(|(partition, (tx, _reservation))| (partition, tx)).collect()
            ));
            spawned_tasks.push(wait_for_task);
        }

        Self {
            channels: channels,
            debugTester: "Test".to_string(),
            abort_helper: Arc::new(spawned_tasks),
        }
    }

    pub fn debug_tester() {
        println!("RepartitionExecState debugTester");
    }
}

type LazyRepState = Arc<tokio::sync::OnceCell<Mutex<RepartitionExecState>>>; // oncecell to make sure we only initialize once, mutex to make sure we can (correctly) access it from multiple threads

// partitioner for batches based on a partitioning
pub struct MsjBatchPartitioner {
    state: MsjBatchPartitionerState,
}

// batch partitioner possibilities, hash is supposed to be used but roundrobin was created to test the workings
enum MsjBatchPartitionerState {
    Hash {
        random_state: ahash::RandomState,
        num_partitions: usize,
        hash_buffer: Vec<u64>,
        partition_key: usize,
    },
    RoundRobin {
        num_partitions: usize,
        next_idx: usize,
    },
}

impl MsjBatchPartitioner {
    pub fn try_new(
        num_partitions: usize,
        partition_id: usize,
        partition_key: usize,
    ) -> Result<Self, DataFusionError> {
        let state = match partition_id {
            0 => MsjBatchPartitionerState::Hash {
                random_state: ahash::RandomState::with_seeds(0, 0, 0, 0),
                num_partitions,
                hash_buffer: vec![],
                partition_key: partition_key,
            },
            1 => MsjBatchPartitionerState::RoundRobin {
                num_partitions,
                next_idx: 0,
            },
            other => {
                return Err(DataFusionError::NotImplemented(format!(
                    "Partitioning {:?} not implemented for MsjBatchPartitioner",
                    other
                )))
            }
        };

        Ok(Self { state })
    }

    pub fn partition<F>(
        &mut self,
        batch: SemiJoinResultBatch,
        partition: usize,
        mut f: F,
    ) -> Result<(), DataFusionError>
    where
        F: FnMut(usize, SemiJoinResultBatch) -> Result<(), DataFusionError>,
    {
        self.partition_iter(batch, partition)?
            .try_for_each(|res| match res {
                Ok((partition, batch)) => f(partition, batch),
                Err(e) => Err(e),
            })
    }

    fn partition_iter(
        &mut self,
        batch: SemiJoinResultBatch,
        partition: usize,
    ) -> Result<
        impl Iterator<Item = Result<(usize, SemiJoinResultBatch), DataFusionError>>,
        DataFusionError,
    > {
        let it: Box<
            dyn Iterator<Item = Result<(usize, SemiJoinResultBatch), DataFusionError>> + Send,
        > = match &mut self.state {
            MsjBatchPartitionerState::RoundRobin {
                num_partitions,
                next_idx,
            } => {
                let idx = *next_idx;
                *next_idx = (*next_idx + 1) % *num_partitions;
                Box::new(std::iter::once(Ok((idx, batch))))
            }
            MsjBatchPartitionerState::Hash {
                random_state,
                num_partitions,
                hash_buffer,
                partition_key,
            } => {
                let num_partitions = *num_partitions;
                match batch {
                    SemiJoinResultBatch::Flat(val) => {
                        println!("partition key: {} in id", partition_key);
                        let column = val.column(*partition_key);
                        // println!("val column count: {}", val.num_columns());
                        // println!(
                        //     "[Flat] val num rows: {} on partition {}",
                        //     val.num_rows(),
                        //     partition
                        // );
                        // println!("batch: {:?}", val);
                        hash_buffer.clear();
                        hash_buffer.resize(val.num_rows(), 0);
                        let array_ref = column.clone();
                        create_hashes(&[array_ref], random_state, hash_buffer)?;

                        //init indices
                        let mut indices: Vec<_> = (0..num_partitions)
                            .map(|_| Vec::with_capacity(val.num_rows()))
                            .collect();

                        for (index, hash) in hash_buffer.iter().enumerate() {
                            indices[(hash % num_partitions as u64) as usize].push(index as u32);
                        }


                        //decrease indices size //! debugging purposes!
                        // for i in 0..num_partitions {
                        //     if indices[i].len() > 2 {
                        //         indices[i] = indices[i][0..1].to_vec();
                        //     }
                        // }

                        //vector containing the rebuilt batches
                        let mut batches: Vec<SemiJoinResultBatch> = Vec::new();
                        //for each partition, create a new batch
                        for i in 0..num_partitions {
                            //rebuild batches
                            let new_columns: Vec<ArrayRef> = val
                                .columns()
                                .iter()
                                .map(|column| {
                                    let indices =
                                        Arc::new(UInt32Array::from(indices[i].clone())) as ArrayRef;
                                    let new_column = take(column.as_ref(), &indices, None)?;
                                    Ok::<_, DataFusionError>(new_column)
                                })
                                .collect::<Result<Vec<_>, _>>()?;
                            let new_batch = SemiJoinResultBatch::Flat(RecordBatch::try_new(
                                val.schema().clone(),
                                new_columns,
                            )?);
                            let nb = new_batch.clone();
                            match nb {
                                SemiJoinResultBatch::Flat(val) => {
                                    // println!("[Flat]nb num rows: {} on partition {}", val.num_rows(), partition);
                                }
                                _ => {}
                            }
                            // println!("-----\noriginal batch: {:?}\n batch {}: {:?}\n-----", val, i, new_batch);
                            batches.push(new_batch);
                            
                        }
                        return Ok(Box::new(
                            batches
                                .into_iter()
                                .enumerate()
                                .map(|(i, batch)| Ok((i, batch))),
                        )
                            as Box<
                                dyn Iterator<
                                        Item = Result<
                                            (usize, SemiJoinResultBatch),
                                            DataFusionError,
                                        >,
                                    > + Send,
                            >);
                    }
                    SemiJoinResultBatch::Nested(val) => {
                        // println!("partition key: {} in id", partition_key);
                        let column = val.regular_column(*partition_key);
                    
                        

                        hash_buffer.clear();
                        hash_buffer.resize(val.num_rows(), 0);
                        let array_ref = column.clone();
                        create_hashes(&[array_ref], random_state, hash_buffer)?;

                        //init indices
                        let mut indices: Vec<_> = (0..num_partitions)
                            .map(|_| Vec::with_capacity(val.num_rows()))
                            .collect();
                        //fill indices, based on calculated hash
                        for (index, hash) in hash_buffer.iter().enumerate() {
                            indices[(hash % num_partitions as u64) as usize].push(index as u32);
                        }
                        //vector containing the rebuilt batches
                        let mut batches: Vec<SemiJoinResultBatch> = Vec::new();
                        //rebuild a batch for each partition
                        for i in 0..num_partitions {
                            let arr: Vec<u32> = indices[i].iter().map(|x| *x as u32).collect();
                            // let arr: Sel = Sel::new(arr);

                            let schema = val.schema().clone();

                            // let v = val.clone();
                            // let mut inner_cols = val.inner.nested_cols;
                            //rebuild batches
                            // println!("inner cols len: {}", inner_cols.len());
                            let mut inner_cols_final: Vec<NestedColumn> = Vec::new();
                            for col in val.inner.nested_cols.iter() {
                                // take_nested_column_inplace(col, &arr);
                                let c = take_rows_from_nestedcol(col, arr.as_ref())?;
                                inner_cols_final.push(c);
                            }

                            // println!("sel len: {}", sel.len());

                            let regular_cols = val
                                .inner
                                .regular_cols
                                .iter()
                                .map(|column| {
                                    let indices =
                                        Arc::new(UInt32Array::from(indices[i].clone())) as ArrayRef;
                                    let new_column = take(column.as_ref(), &indices, None)?;
                                    Ok::<_, DataFusionError>(new_column)
                                })
                                .collect::<Result<Vec<_>, _>>()?;

                            let new_batch = SemiJoinResultBatch::Nested(NestedBatch::new(
                                schema,
                                regular_cols,
                                inner_cols_final,
                            ));
                            // println!("-----\n-----\noriginal batch:\n {:?}\n+++++\n new batch for partition {}:\n {:?}\n-----\n-----", val, i, new_batch);

                            batches.push(new_batch);
                        }

                        // return Ok(batches
                        //     .into_iter()
                        //     .enumerate()
                        //     .map(|(i, batch)| Ok((i, batch))));
                        return Ok(Box::new(
                            batches
                                .into_iter()
                                .enumerate()
                                .map(|(i, batch)| Ok((i, batch))),
                        )
                            as Box<
                                dyn Iterator<
                                        Item = Result<
                                            (usize, SemiJoinResultBatch),
                                            DataFusionError,
                                        >,
                                    > + Send,
                            >);
                    }
                }

                //throw error
                Err(DataFusionError::NotImplemented(
                    "Hash partitioning not implemented for MsjBatchPartitioner".to_string(),
                ))?
            }
        };
        Err(DataFusionError::NotImplemented(
            "Hash partitioning not implemented for MsjBatchPartitioner".to_string(),
        ))?
    }
}

pub trait MultiSemiJoinWrapper: Debug + Send + Sync {
    fn schema(&self) -> &NestedSchemaRef;
    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableSemiJoinResultBatchStream, DataFusionError>;
    fn as_json(&self, output: &mut String) -> Result<(), std::fmt::Error>;
    fn collect_metrics(&self, output_buffer: &mut String, indent: usize);
    fn guard(&self) -> &Arc<dyn ExecutionPlan>;
    fn children(&self) -> &[Arc<GroupBy>];
    fn semijoin_keys(&self) -> &Vec<Vec<usize>>;
    fn partitioned(&self) -> bool;
    fn id(&self) -> usize;
}

/// Create new `Vec<u32>` by taking the elements in `data` at the positions in `indices`.
/// # Panics
/// Panics if an index in `indices` is out of bounds for `data`.
#[inline(always)]
fn take_unnest(data: &[u32], indices: &[Idx]) -> Result<Vec<u32>, DataFusionError> {
    let mut result = Vec::with_capacity(indices.len());
    for idx in indices {
        result.push(unsafe { *data.get_unchecked(*idx as usize) });
    }
    Ok(result)
}

/// Create new [NestedColumn] by taking the rows in `nestedcol` at the positions in `row_ids`.
/// # Panics
/// Panics if an index in `row_ids` is out of bounds for `nestedcol`.
#[inline(always)]
pub fn take_rows_from_nestedcol(
    nestedcol: &NestedColumn,
    row_ids: &[Idx],
) -> Result<NestedColumn, DataFusionError> {
    match nestedcol {
        NestedColumn::Singular(s_nestedcol) => {
            let new_weights = take_unnest(&s_nestedcol.weights, row_ids)?;
            let nestedcol = SingularNestedColumn {
                weights: new_weights,
            };
            Ok(NestedColumn::Singular(nestedcol))
        }
        NestedColumn::NonSingular(ns_nestedcol) => {
            let new_weights = take_unnest(&ns_nestedcol.weights, row_ids)?;
            let new_hols = take_unnest(&ns_nestedcol.hols, row_ids)?;
            let new_data = ns_nestedcol.clone_data(); // ! changed this to make a deep copy of the data

            let nestedcol = NonSingularNestedColumn {
                weights: new_weights,
                hols: new_hols,
                data: Arc::new(new_data), // clone arc = cheap
            };

            Ok(NestedColumn::NonSingular(nestedcol))
        }
    }
}

//repartitionexec operator to be placed on top of a multisemijoin
#[derive(Debug)]
pub struct RepartitionMultiSemiJoin {
    //state for te repartitionexec operator, containing the channels for each partition and the abort helper
    state: LazyRepState,
    //child operator to be repartitioned, to be converted to a vector of multisemijoin operators
    child: Arc<MultiSemiJoin>,
    //amount of input partitions
    partitions: usize,
    //partitioning scheme
    //TODO: implement partitioning scheme from datafusion (the way repartitionexec does it)
    partition_key: usize,
}

impl RepartitionMultiSemiJoin {
    //create new repartitionexec operator, also creating the necessary multisemijoin operator
    pub fn new(
        guard: Arc<dyn ExecutionPlan>,
        children: Vec<Arc<GroupBy>>,
        equijoin_keys: Vec<Vec<(usize, usize)>>,
        id: usize,
        partition_key: usize,
    ) -> Self {
        // println!("RepartitionMultiSemiJoin new");
        let guardpartitions = guard.output_partitioning().partition_count();
        Self {
            state: Default::default(),
            child: Arc::new(MultiSemiJoin::new(guard, children, equijoin_keys, id)),
            partitions: guardpartitions,
            partition_key: partition_key,
        }
    }

    //try to make new repartitionexec
    pub fn try_new(
        guard: Arc<dyn ExecutionPlan>,
        children: Vec<Arc<GroupBy>>,
        equijoin_keys: Vec<Vec<(usize, usize)>>,
        id: usize,
        partition_key: usize,
    ) -> Result<Self, DataFusionError> {
        let guardpartitions = guard.output_partitioning().partition_count();
        Ok(Self {
            state: Default::default(), //for now, just add a default value, it will be created later on in the execute function (get_or_init)
            child: Arc::new(MultiSemiJoin::new(guard, children, equijoin_keys, id)),
            partitions: guardpartitions,
            partition_key: partition_key,
        })
    }

    pub fn child(&self) -> &Arc<MultiSemiJoin> {
        &self.child
    }

    // execute function for repartitionexec operator, it is supposed to execute the child operator(s) and repartition the data it receives
    pub fn execute_impl(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableSemiJoinResultBatchStream, DataFusionError> {
        //clone all necessary variables to be used in the async block
        let rep_state = Arc::clone(&self.state);
        let schema = self.child.schema().clone();
        let input = Arc::clone(&self.guard());
        let contextclone = context.clone();
        let child = Arc::clone(&self.child);
        let rep_key = self.partition_key;

        // println!("repartitionmsj execute");
        //create stream object to be returned
        let stream = futures::stream::once(async move {
            let num_input_partitions = input.output_partitioning().partition_count();
            //create or initialize the state object
            let state = rep_state
                .get_or_init(|| async move {
                    //create or initialize the state object
                    Mutex::new(RepartitionExecState::new(child, contextclone, rep_key))
                })
                .await;

            // let state = state.lock();
            //test block
            // println!("repartitionmsj test block");

            //retrieve the output channel relevant to the partition
            let (mut output_channel, reservation, abort_helper) = {
                let mut state = state.lock();

                let (_input_channel, output_channel, reservation) = state
                    .channels
                    .remove(&partition)
                    .expect("partition not used yet");

                (output_channel, reservation, state.abort_helper.clone())
            };
            Ok::<Pin<Box<dyn MultiSemiJoinBatchStream + Send>>, DataFusionError>(Box::pin(
                MsjRepartitionStream {
                    num_input_partitions,
                    num_input_partitions_processed: 0,
                    input: output_channel.swap_remove(0), //retrieve 0th element from output channel so we don't have a vec
                    schema,
                    reservation,
                    drop_helper: abort_helper,
                },
            )
                as SendableSemiJoinResultBatchStream)
        })
        .try_flatten();
        // Ok(Box::pin(stream))

        let stream = MultiSemiJoinStreamAdapter::new(self.child.schema().clone(), stream);

        Ok(Box::pin(stream))

        // let child_stream = self.child.execute(partition, context)?;
        // return Ok(child_stream);
        // }
    }

    pub fn statetest(
        &self,
        context: Arc<TaskContext>,
    ) -> futures::stream::Once<impl std::future::Future<Output = String>> {
        let rep_state = Arc::clone(&self.state);
        let contextclone = context.clone();
        let childguard = self.child.guard().clone();
        let child = Arc::clone(&self.child);
        let rep_key = self.partition_key;
        let stream = futures::stream::once(async move {
            //this is where the stream object to be returned should be created
            let state = rep_state
                .get_or_init(|| async move {
                    //create or initialize the state object
                    Mutex::new(RepartitionExecState::new(child, contextclone, rep_key))
                })
                .await;

            let state = state.lock();

            // eprintln!("repartitionmsj test block");
            // eprintln!("{}", state.debugTester);

            // let child_stream = self.child.execute(partition, contextclone)?;
            // child_stream
            state.debugTester.clone()
        });

        stream
    }

    //function to pull data from an input plan and feed it to output channels
    //pull data from the input, feeding it to the output channels
    async fn pull_from_input(
        input: Arc<MultiSemiJoin>,
        partition: usize,
        mut output_channnels: HashMap<
            usize,
            (
                DistributionSender<MaybeNestedBatch>,
                SharedMemoryReservation,
            ),
        >,
        context: Arc<TaskContext>,
        repartition_key: usize,
    ) -> Result<(), DataFusionError> {
        println!("pull from input on partition {}", partition);
        let mut input_stream = input.execute(partition, context)?;
        let num_outputs = output_channnels.len();
        // println!("num outputs: {}", num_outputs);
        let mut partitioner = MsjBatchPartitioner::try_new(num_outputs, 0, repartition_key)?; // ! num partitions obviously should not be a hardcoded 2

        loop {
            //get batch from input stream, break the loop if there is no next
            let batch = input_stream.next().await; //as long as there is a next in the input stream
            let batch = match batch {
                //if it is a batch, proceed otherwise break
                Some(batch) => batch?,
                None => break,
            };

            for res in partitioner.partition_iter(batch, partition)? {
                let (partition_send, batch) = res?;
                // println!(
                //     "sending batch from partition {} to partition {}",
                //     partition, partition_send
                // );

                // println!("schema: {}", rbatch.schema());
                // println!("num rows: {}", rbatch.num_rows());
                //choose the output channel to send to, if we set this to 0 we will send everything to the first partition
                if let Some((input_channel, reservation)) =
                    output_channnels.get_mut(&partition_send)
                {
                    //this partition is the partition we are sending to
                    let size = batch.get_array_memory_size();
                    reservation.lock().try_grow(size)?;

                    if input_channel.send(Some(Ok(batch))).await.is_err() {
                        //if send is unsuccessful, shrink
                        reservation.lock().shrink(size);
                    }
                }
            }
        }
        println!("pull from input on partition {} done", partition);
        Ok(()) //success
    }

    //function to wait for a given input task
    async fn wait_for_task(input_task: SpawnedTask<Result<(), DataFusionError>>, txs: HashMap<usize, DistributionSender<MaybeNestedBatch>>) {
        // input_task.join().await;
        match input_task.join().await{
            Ok(_) => {
                for (i, tx) in txs {
                    tx.send(None).await.expect("send none");
                    // println!("sent none to {}", i);
                }
            },
            Err(e) => {
                let e = Arc::new(e);
                for (_, tx) in txs {
                    let err = Err(DataFusionError::Context("error".to_string(), Box::new(DataFusionError::External(Box::new(Arc::clone(&e))))));
                    tx.send(Some(err)).await.expect("send none");
                }
            },
        }
    }
}

impl MultiSemiJoinWrapper for RepartitionMultiSemiJoin {
    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableSemiJoinResultBatchStream, DataFusionError> {
        // println!("RepartitionMultiSemiJoin execute");
        self.execute_impl(partition, context)
    }

    fn schema(&self) -> &NestedSchemaRef {
        self.child.schema()
    }

    fn as_json(&self, output: &mut String) -> Result<(), std::fmt::Error> {
        self.child.as_json(output)
    }

    fn collect_metrics(&self, output_buffer: &mut String, indent: usize) {
        self.child.collect_metrics(output_buffer, indent)
    }

    fn guard(&self) -> &Arc<dyn ExecutionPlan> {
        self.child.guard()
    }

    fn children(&self) -> &[Arc<GroupBy>] {
        self.child.children()
    }
    fn semijoin_keys(&self) -> &Vec<Vec<usize>> {
        self.child.semijoin_keys()
    }

    fn partitioned(&self) -> bool {
        self.child.partitioned()
    }

    fn id(&self) -> usize {
        self.child.id()
    }
}

struct MsjRepartitionStream {
    //total number of input partitions that will be sending batches to this output channel
    num_input_partitions: usize,

    //channels that have finished sending to this output channel
    num_input_partitions_processed: usize,

    //output channel for each partition
    input: DistributionReceiver<MaybeNestedBatch>,

    //schema
    schema: NestedSchemaRef,

    reservation: SharedMemoryReservation,

    #[allow(dead_code)]
    drop_helper: Arc<Vec<SpawnedTask<()>>>,
}

impl Stream for MsjRepartitionStream {
    type Item = Result<SemiJoinResultBatch, DataFusionError>;

    //poll next function taken from repartition stream
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        loop {
            match self.input.recv().poll_unpin(cx) {
                //look at state for poll_unpin
                Poll::Ready(Some(Some(v))) => {
                    //if resultbatch gotten
                    if let Ok(batch) = &v {
                        self.reservation
                            .lock()
                            .shrink(batch.get_array_memory_size());
                    }

                    return Poll::Ready(Some(v));
                }
                Poll::Ready(Some(None)) => {
                    //if no resultbatch is none
                    self.num_input_partitions_processed += 1;

                    if self.num_input_partitions == self.num_input_partitions_processed {
                        // all input partitions have finished sending batches
                        return Poll::Ready(None);
                    } else {
                        // other partitions still have data to send
                        continue;
                    }
                }
                Poll::Ready(None) => {
                    //if no result
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    //if pending
                    return Poll::Pending;
                }
            }
        }
    }
}

impl MultiSemiJoinBatchStream for MsjRepartitionStream {
    fn schema(&self) -> &NestedSchemaRef {
        &self.schema
    }
}



#[cfg(test)]

mod tests {

    use datafusion::{
        arrow::{
            array::{Int8Array, RecordBatch, UInt8Array},
            datatypes::{DataType, Field, Schema},
            error::ArrowError,
        },
        physical_plan::memory::MemoryExec,
    };
    use futures::StreamExt;
    use std::error::Error;

    use crate::yannakakis::data::SemiJoinResultBatch;

    use super::*;

    /// | a | b  | c |
    /// | - | -- | - |
    /// | 1 | 1  | 1 |
    /// | 1 | 2  | 2 |
    /// | 1 | 3  | 3 |
    /// | 1 | 4  | 4 |
    /// | 1 | 5  | 5 |
    /// | 1 | 6  | 1 |
    /// | 1 | 7  | 2 |
    /// | 1 | 8  | 3 |
    /// | 1 | 9  | 4 |
    /// | 1 | 10 | 5 |
    fn example_batch() -> Result<RecordBatch, ArrowError> {
        let schema = Arc::new(Schema::new(vec![
            Field::new("a", DataType::UInt8, false),
            Field::new("b", DataType::Int8, false),
            Field::new("c", DataType::UInt8, false),
        ]));
        let a = UInt8Array::from(vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1]);
        let b = Int8Array::from(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let c = UInt8Array::from(vec![1, 2, 3, 4, 5, 1, 2, 3, 4, 5]);
        RecordBatch::try_new(schema.clone(), vec![Arc::new(a), Arc::new(b), Arc::new(c)])
    }

    /// | a | b  | c |
    /// | - | -- | - |
    /// | 1 | 1  | 1 |
    /// | 1 | 2  | 2 |
    /// | 1 | 3  | 3 |
    /// | 1 | 4  | 4 |
    /// | 1 | 5  | 5 |
    /// | 1 | 6  | 1 |
    /// | 1 | 7  | 2 |
    /// | 1 | 8  | 3 |
    /// | 1 | 9  | 4 |
    /// | 1 | 10 | 5 |
    fn example_guard() -> Result<Arc<dyn ExecutionPlan>, DataFusionError> {
        let batch = example_batch()?;
        let schema = batch.schema();
        let partition = vec![batch];
        Ok(Arc::new(MemoryExec::try_new(&[partition], schema, None)?))
    }

    // // test wether the repartitionshredded operator makes the same output as a normal msj operator
    // #[tokio::test]
    // async fn test_execute_repartition_shredded() -> Result<(), Box<dyn Error>> {
    //     let guard1 = example_guard().unwrap();
    //     let guard2 = example_guard().unwrap();

    //     let semijoin1 = MultiSemiJoin::new(guard1, vec![], vec![]);

    //     let repartition = RepartitionMultiSemiJoin::new(guard2, vec![], vec![]);

    //     let result1 = semijoin1.execute(0, Arc::new(TaskContext::default()))?;

    //     let result2 = repartition.execute(0, Arc::new(TaskContext::default()))?;

    //     let batches1 = result1
    //         .collect::<Vec<Result<SemiJoinResultBatch, DataFusionError>>>()
    //         .await;

    //     let batches2 = result2
    //         .collect::<Vec<Result<SemiJoinResultBatch, DataFusionError>>>()
    //         .await;

    //     assert_eq!(batches1.len(), batches2.len());

    //     Ok(())
    // }

    #[tokio::test]
    async fn test_state_creation() -> Result<(), Box<dyn Error>> {
        let guard = example_guard().unwrap();
        let repartition = RepartitionMultiSemiJoin::new(guard, vec![], vec![], 0, 0);

        let context = Arc::new(TaskContext::default());

        let state = repartition.statetest(context);

        let word: Vec<String> = state.collect().await;

        // let state = state.lock();

        assert_eq!(word[0], "Test");

        Ok(())
    }
}
