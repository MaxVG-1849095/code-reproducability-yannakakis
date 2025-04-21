// Implementation of the repartitionexec operator for shredded records, implemented to partition data calculated via multisemijoin

use std::task::{Context, Poll};
use std::{pin::Pin, sync::Arc};

use batch_partitioner::MsjBatchPartitioner;
use datafusion::arrow;
use datafusion::arrow::array::{ArrayRef, RecordBatch, UInt32Array};

use datafusion::datasource::empty;
use tokio::sync::Barrier;

use datafusion::execution::memory_pool::MemoryReservation;
use datafusion::physical_plan::metrics::ExecutionPlanMetricsSet;

use datafusion::{
    error::DataFusionError,
    execution::{memory_pool::MemoryConsumer, TaskContext},
    physical_plan::ExecutionPlan,
};
use futures::{FutureExt, Stream, StreamExt, TryStreamExt};
use nested_combiner::NestedCombiner;
use nested_combiner::NestedCombinerWrapper;
use tokio::time;

use super::data::{
    Idx, NestedColumn, NestedRel, NestedSchema, NonSingularNestedColumn, SemiJoinResultBatch,
    SingularNestedColumn,
};

use super::multisemijoin::MultiSemiJoinBatchStream;
use super::{
    data::{NestedBatch, NestedSchemaRef},
    groupby::GroupBy,
    multisemijoin::{MultiSemiJoin, SendableSemiJoinResultBatchStream},
};
use crate::yannakakis::multisemijoin::MultiSemiJoinStreamAdapter;

use distributor_channels::{channels, DistributionReceiver, DistributionSender};
use hashbrown::HashMap;
use parking_lot::Mutex;
use spawned_task::SpawnedTask;

use std::fmt::Debug;

use metrics::MsjRepartitionMetrics;

mod batch_partitioner;
mod distributor_channels;
mod metrics;
mod nested_combiner;
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

    //vector with 1 nestedcolumn for each partition, meant to be combined into one and used in rebuilding the batches
    inner_cols: Vec<NestedColumn>,

    dummy: i32,
}

impl RepartitionExecState {
    fn new(
        input: Arc<MultiSemiJoin>,
        context: Arc<TaskContext>,
        repartition_key: usize,
        metrics: ExecutionPlanMetricsSet,
    ) -> Self {
        let num_input_partitions = input.guard_partition_count();
        let num_output_partitions = input.guard_partition_count();
        let (send_channels, receive_channels) = {
            //only the preserve_order = false route has been implemented for now
            let (send_channels, receive_channels) = channels(num_input_partitions);
            let send_channels = send_channels
                .into_iter()
                .map(|item| vec![item; num_input_partitions])
                .collect::<Vec<_>>(); //turn into 2D vector
            let receive_channels = receive_channels
                .into_iter()
                .map(|item| vec![item])
                .collect::<Vec<_>>(); //turn into 2D vector

            (send_channels, receive_channels)
        };

        let mut channels = HashMap::with_capacity(send_channels.len()); // init hashmap with amount of partitions
        for (partition, (send_channel, receive_channel)) in
            send_channels.into_iter().zip(receive_channels).enumerate()
        {
            let reservation = Arc::new(Mutex::new(
                MemoryConsumer::new(format!("{}[{partition}]", "RepartitionExec"))
                    .register(context.memory_pool()),
            ));
            channels.insert(partition, (send_channel, receive_channel, reservation));
        }

        // goal is to launch 1 task per input partition, these tasks gather input via a helper function and send it to the output channel
        // each task has its own waiter, which is used to wait for the task to finish
        let mut spawned_tasks = Vec::with_capacity(num_input_partitions);
        let child_id = input.id();

        // println!("num input partitions: {}", num_input_partitions);
        let nested_combiner = Arc::new(Mutex::new(NestedCombinerWrapper::new(
            num_input_partitions,
            input.children().len(),
        )));
        let barrier = Arc::new(Barrier::new(num_input_partitions));
        for i in 0..num_input_partitions {
            let channels_in: HashMap<_, _> = channels
                .iter()
                .map(
                    |(partition, (send_channels, _receive_channels, reservations))| {
                        (*partition, (send_channels[i].clone(), reservations.clone()))
                    },
                )
                .collect();

            //TODO: add metrics
            let r_metrics = MsjRepartitionMetrics::new(i, num_output_partitions, &metrics);

            let input_task = SpawnedTask::spawn(RepartitionMultiSemiJoin::pull_from_input(
                Arc::clone(&input),
                i,
                channels_in.clone(),
                Arc::clone(&context),
                repartition_key,
                r_metrics,
                child_id,
                Arc::clone(&nested_combiner),
                barrier.clone(),
            ));

            let wait_for_task = SpawnedTask::spawn(RepartitionMultiSemiJoin::wait_for_task(
                input_task,
                channels_in
                    .into_iter()
                    .map(|(partition, (send_channel, _reservation))| (partition, send_channel))
                    .collect(),
            ));
            spawned_tasks.push(wait_for_task);
        }

        Self {
            channels: channels,
            debugTester: "Test".to_string(),
            abort_helper: Arc::new(spawned_tasks),
            inner_cols: Vec::with_capacity(num_input_partitions),
            dummy: 0,
        }
    }

    pub fn debug_tester() {
        println!("RepartitionExecState debugTester");
    }
}

type LazyRepState = Arc<tokio::sync::OnceCell<Mutex<RepartitionExecState>>>; // oncecell to make sure we only initialize once, mutex to make sure we can (correctly) access it from multiple threads

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

//repartitionexec operator to be placed on top of a multisemijoin
#[derive(Debug)]
pub struct RepartitionMultiSemiJoin {
    //state for te repartitionexec operator, containing the channels for each partition and the abort helper
    state: LazyRepState,
    //child operator to be repartitioned, to be converted to a vector of multisemijoin operators
    child: Arc<MultiSemiJoin>,
    //amount of input partitions
    partitions: usize,
    //partition key to be used for repartitioning
    partition_key: usize,
    //metrics
    metrics: ExecutionPlanMetricsSet,

    barrier: Arc<Barrier>,
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
        let barrier = Arc::new(Barrier::new(guardpartitions));
        Self {
            state: Default::default(),
            child: Arc::new(MultiSemiJoin::new(guard, children, equijoin_keys, id)),
            partitions: guardpartitions,
            partition_key: partition_key,
            metrics: ExecutionPlanMetricsSet::new(),
            barrier: barrier,
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
        let barrier = Arc::new(Barrier::new(guardpartitions));
        Ok(Self {
            state: Default::default(), //for now, just add a default value, it will be created later on in the execute function (get_or_init)
            child: Arc::new(MultiSemiJoin::new(guard, children, equijoin_keys, id)),
            partitions: guardpartitions,
            partition_key: partition_key,
            metrics: ExecutionPlanMetricsSet::new(),
            barrier: barrier,
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
        let metrics = self.metrics.clone();

        // println!("repartitionmsj execute");
        //create stream object to be returned
        let stream = futures::stream::once(async move {
            let num_input_partitions = input.output_partitioning().partition_count();
            //create or initialize the state object
            let state = rep_state
                .get_or_init(|| async move {
                    //create or initialize the state object
                    Mutex::new(RepartitionExecState::new(
                        child,
                        contextclone,
                        rep_key,
                        metrics,
                    ))
                })
                .await;
            // let state = state.lock();
            //test block
            // println!("repartitionmsj test block");

            //retrieve the output channel relevant to the partition
            let (mut receive_channel, reservation, abort_helper) = {
                let mut state = state.lock();

                let (_send_channel, receive_channel, reservation) = state
                    .channels
                    .remove(&partition)
                    .expect("partition not used yet");

                (receive_channel, reservation, state.abort_helper.clone())
            };
            Ok::<Pin<Box<dyn MultiSemiJoinBatchStream + Send>>, DataFusionError>(Box::pin(
                MsjRepartitionStream {
                    num_input_partitions,
                    num_input_partitions_processed: 0,
                    input: receive_channel.swap_remove(0), //retrieve 0th element from output channel so we don't have a vec
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

    //function to pull data from an input plan and feed it to output channels
    //pull data from the input, feeding it to the output channels
    async fn pull_from_input(
        input: Arc<MultiSemiJoin>,
        partition: usize,
        mut send_channnels: HashMap<
            usize,
            (
                DistributionSender<MaybeNestedBatch>,
                SharedMemoryReservation,
            ),
        >,
        context: Arc<TaskContext>,
        repartition_key: usize,
        metrics: MsjRepartitionMetrics,
        msj_id: usize,
        nested_combiner: Arc<Mutex<NestedCombinerWrapper>>,
        barrier: Arc<Barrier>,
    ) -> Result<(), DataFusionError>
    where
        NestedCombiner: Send + Sync,
    {
        // println!("pull from input on partition {}", partition);
        //start fetch time
        let timer = metrics.fetch_time.timer();
        let mut input_stream = input.execute(partition, context)?;
        timer.done();

        let num_outputs = send_channnels.len();
        // println!("num outputs: {}", num_outputs);
        let mut partitioner = MsjBatchPartitioner::try_new(
            num_outputs,
            0,
            repartition_key,
            msj_id,
            metrics.repartition_time.clone(),
        )?;

        // ! sync needed between threads, the partitioner needs the nested columns of all partitions to be present in order to join it --> barrier

        //get first batch from input stream, this will be used to create nested columns!
        let batch = input_stream.next().await; //as long as there is a next in the input stream
        let batch = match batch {
            //if it is a batch, proceed otherwise break
            Some(batch) => {
                let b = batch?;
                let b_clone = b.clone();
                match b {
                    SemiJoinResultBatch::Flat(_) => {
                        // println!("flat batch");
                        b
                    }
                    SemiJoinResultBatch::Nested(nested_batch) => {
                        // println!("nested batch");
                        let mut n_b = nested_batch.clone();
                        while Self::any_regular_col_empty(&n_b) {
                            let new_batch = input_stream.next().await;
                            match new_batch {
                                Some(batch) => {
                                    let b = batch?;
                                    match b {
                                        SemiJoinResultBatch::Flat(_) => {
                                            println!("flat batch, ERROR");
                                            // this should not happen, we should only get nested batches
                                            panic!("flat batch in repartition where its shouldnt be");
                                            break;
                                        }
                                        SemiJoinResultBatch::Nested(nested_batch) => {
                                            // println!("nested batch");
                                            n_b = nested_batch.clone();
                                        }
                                    }
                                }
                                None => {
                                    println!("partition {} has no batch", partition);
                                    //the partition is completely empty
                                    nested_combiner.lock().add_empty_inner_col(partition);
                                    barrier.wait().await; //wait for all partitions to be present and to have added their inner columns
                                    barrier.wait().await; // second barrier to wait for the combine to finish
                                    return (Ok(()));
                                }
                            }
                        }
                    SemiJoinResultBatch::Nested(n_b)
                    }
                }
            },
            None => {
                println!("partition {} has no batch", partition);
                //the partition is completely empty
                nested_combiner.lock().add_empty_inner_col(partition);
                barrier.wait().await; //wait for all partitions to be present and to have added their inner columns
                barrier.wait().await; // second barrier to wait for the combine to finish
                return (Ok(()));
            }
        };
        let children_len = input.children().len();
        let batch_clone = batch.clone();
        // println!("\n in msj {} partition {}\nbatch: {:?}\n", msj_id,partition,batch);
        match batch_clone {
            SemiJoinResultBatch::Flat(_) => {
                // println!("flat batch");
                barrier.wait().await; //wait for all partitions to be present and to have added their inner columns
                barrier.wait().await; // second barrier to wait for the combine to finish
            }
            SemiJoinResultBatch::Nested(nested_batch) => {

                // println!("\n------\nmsjrep {}\nadding inner col to nested combiner from partition {} \n inner_col: {:?}\n------", msj_id,partition, nested_batch.inner.nested_cols[0]);
                for i in 0..children_len {
                    //for each child, add the inner column to the combiner
                    nested_combiner.lock().add_inner_col(
                        nested_batch.inner.nested_cols[i].clone(),
                        partition,
                        i,
                    );
                    if nested_batch.inner.get_total_weights().is_some() {
                        nested_combiner.lock().add_total_weights(
                            nested_batch.inner.get_total_weights().unwrap().to_vec(),
                            partition,
                        );
                    }
                }
                barrier.wait().await; //wait for all partitions to be present and to have added their inner columns
                                      // println!("-----\n partition {}\n nested batch regular cols: {:?}\n nested batch nested cols: {:?}\n-----", partition,nested_batch.inner.regular_cols, nested_batch.inner.nested_cols);
                if nested_combiner.lock().combined() { //first partition to arrive here has to combine
                    nested_combiner.lock().check_singular_non_singular();
                    // we know we can call combine since a barrier made sure that all partitions were present
                    let _ = nested_combiner.lock().combine();
                }
                barrier.wait().await; // second barrier to wait for the combine to finish
            }
        }

        let nested_data = nested_combiner.lock().get_final_inner_col_data().clone();
        let total_weights = nested_combiner.lock().get_final_total_weights().clone();
        // println!(
        //     "nested data length: {} in partition {} for id {}",
        //     nested_data.len(),
        //     partition,
        //     msj_id
        // );
        // println!("nested data: {:?}", nested_data);

        let offsets = nested_combiner.lock().get_offsets().clone();
        let mut first_iter = true;
        // loop to pull data from input and send it to the output channels
        loop {
            //get batch from input stream, break the loop if there is no next
            // let timer = metrics.fetch_time.timer();
            let batch = if !first_iter {
                match input_stream.next().await {
                    Some(Ok(batch)) => batch,
                    Some(Err(e)) => return Err(e),
                    None => break,
                }
            } else {
                first_iter = false;
                batch.clone()
            };

            // timer.done();
            for res in partitioner.partition_iter(
                batch,
                partition,
                msj_id,
                &nested_data,
                &offsets,
                &total_weights,
            )? {
                let (partition_send, batch) = res?;
                let timer = metrics.send_time[partition_send].timer();
                //choose the output channel to send to, if we set this to 0 we will send everything to the first partition
                if let Some((send_channel, reservation)) = send_channnels.get_mut(&partition_send) {
                    //this partition is the partition we are sending to
                    let size = batch.get_array_memory_size();
                    reservation.lock().try_grow(size)?;
                    
                    if send_channel.send(Some(Ok(batch))).await.is_err() {
                        //if send is unsuccessful, shrink
                        reservation.lock().shrink(size);
                    }
                }
                timer.done();
            }
        }
        // nested_combiner.lock().print_content();
        // println!("pull from input on partition {} done", partition);
        Ok(()) //success
    }

    //function to wait for a given input task
    async fn wait_for_task(
        input_task: SpawnedTask<Result<(), DataFusionError>>,
        send_channels: HashMap<usize, DistributionSender<MaybeNestedBatch>>,
    ) {
        // input_task.join().await;
        match input_task.join().await {
            Ok(_) => {
                for (i, send_channel) in send_channels {
                    send_channel.send(None).await;
                    // println!("sent none to {}", i);
                }
            }
            Err(e) => {
                let e = Arc::new(e);
                for (_, send_channel) in send_channels {
                    let err = Err(DataFusionError::Context(
                        "error".to_string(),
                        Box::new(DataFusionError::External(Box::new(Arc::clone(&e)))),
                    ));
                    send_channel.send(Some(err)).await.expect("send none");
                }
            }
        }
    }

    fn any_regular_col_empty(
        nested_batch: &NestedBatch,
    ) -> bool {
        for i in 0..nested_batch.inner.regular_cols.len() {
            if nested_batch.inner.regular_cols[i].is_empty() {
                return true;
            }
        }
        false
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
