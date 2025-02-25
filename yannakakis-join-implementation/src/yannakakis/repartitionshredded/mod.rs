// Implementation of the repartitionexec operator for shredded records, implemented to partition data calculated via multisemijoin

use std::task::{Context, Poll};
use std::{pin::Pin, sync::Arc};

use datafusion::execution::memory_pool::MemoryReservation;
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

use super::data::SemiJoinResultBatch;
use super::multisemijoin::MultiSemiJoinBatchStream;
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
    fn new(input: Arc<dyn ExecutionPlan>, context: Arc<TaskContext>) -> Self {
        eprintln!("RepartitionExecState new");
        let num_input_partitions = input.output_partitioning().partition_count();
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

        //TODO: add spawned_tasks

        // goal is to launch 1 task per input partition, these tasks gather input via a helper function and send it to the output channel
        // each task has its own waiter, which is used to wait for the task to finish
        let mut spawned_tasks = Vec::with_capacity(num_input_partitions);
        eprintln!("{}", num_input_partitions);
        for i in 0..num_input_partitions {
            let spawned_task = SpawnedTask::spawn(async move {
                eprintln!("SpawnedTask {}", i);
            });
            spawned_tasks.push(spawned_task);

        }

        Self {
            channels: channels,
            debugTester: "Test".to_string(),
            abort_helper: Arc::new(Vec::new()),
        }
    }

    pub fn debug_tester() {
        println!("RepartitionExecState debugTester");
    }

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
    ) -> Result<(), DataFusionError> {
        let mut input_stream = input.execute(partition, context)?;

        loop {
            let batch = input_stream.next().await; //as long as there is a next in the input stream
            let batch = match batch {
                //if it is a batch, proceed otherwise break
                Some(batch) => batch?,
                None => break,
            };

            if let Some((input_channel, reservation)) = output_channnels.get_mut(&partition) {
                let size = batch.get_array_memory_size();
                reservation.lock().try_grow(size)?;

                if input_channel.send(Some(Ok(batch))).await.is_err() {
                    //if send is unsuccessful, shrink
                    reservation.lock().shrink(size);
                }
            }
        }

        Ok(()) //success
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
    fn children(&self) -> &[Arc<GroupByWrapperEnum>];
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
    //partitioning scheme
    //TODO: implement partitioning scheme from datafusion (the way repartitionexec does it)
    // partitioning: Partitioning
}

impl RepartitionMultiSemiJoin {
    //create new repartitionexec operator, also creating the necessary multisemijoin operator
    pub fn new(
        guard: Arc<dyn ExecutionPlan>,
        children: Vec<Arc<GroupByWrapperEnum>>,
        equijoin_keys: Vec<Vec<(usize, usize)>>,
        id: usize,
    ) -> Self {
        // println!("RepartitionMultiSemiJoin new");
        let guardpartitions = guard.output_partitioning().partition_count();
        Self {
            state: Default::default(),
            child: Arc::new(MultiSemiJoin::new(guard, children, equijoin_keys, id)),
            partitions: guardpartitions,
        }
    }

    //try to make new repartitionexec
    pub fn try_new(
        guard: Arc<dyn ExecutionPlan>,
        children: Vec<Arc<GroupByWrapperEnum>>,
        equijoin_keys: Vec<Vec<(usize, usize)>>,
        id: usize,
    ) -> Result<Self, DataFusionError> {
        let guardpartitions = guard.output_partitioning().partition_count();
        Ok(Self {
            state: Default::default(), //for now, just add a default value, it will be created later on in the execute function (get_or_init)
            child: Arc::new(MultiSemiJoin::new(guard, children, equijoin_keys, id)),
            partitions: guardpartitions,
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
        let rep_state = Arc::clone(&self.state);
        let schema = self.child.schema().clone();
        let input = Arc::clone(&self.guard());
        let guard_clone = self.child.guard().clone();
        let contextclone = context.clone();

        println!("repartitionmsj execute");

        let stream = futures::stream::once(async move {
            let num_input_partitions = input.output_partitioning().partition_count();    
            //this is where the stream object to be returned should be created
            let state = rep_state
                .get_or_init(|| async move {
                    //create or initialize the state object
                    Mutex::new(RepartitionExecState::new(
                        guard_clone,
                        contextclone,
                    ))
                })
                .await;
            
            // let state = state.lock();
            //test block
            println!("repartitionmsj test block");

            let(mut output_channel, reservation, abort_helper) = {
                let mut state = state.lock();

                let(_input_channel, output_channel, reservation) = state
                .channels
                .remove(&partition)
                .expect("partition not used yet");

            (output_channel, reservation, state.abort_helper.clone())
            };
            Ok::<Pin<Box<dyn MultiSemiJoinBatchStream + Send>>, DataFusionError>(Box::pin(MsjRepartitionStream {
                num_input_partitions,
                num_input_partitions_processed: 0,
                input: output_channel.swap_remove(0),
                schema,
                reservation,
            }) as SendableSemiJoinResultBatchStream)
            
        })
        .try_flatten();
        // Ok(Box::pin(stream))

        let stream = MultiSemiJoinStreamAdapter::new(self.child.schema().clone(), stream);

        Ok(Box::pin(stream))

        // let child_stream = self.child.execute(partition, context)?;
        // return Ok(child_stream);
        // }
    }

    pub fn statetest(&self, context: Arc<TaskContext>) -> futures::stream::Once<impl std::future::Future<Output = String>> {
        let rep_state = Arc::clone(&self.state);
        let contextclone = context.clone();
        let childguard = self.child.guard().clone();
        let stream = futures::stream::once(async move {
            //this is where the stream object to be returned should be created
            let state = rep_state
                .get_or_init(|| async move {
                    //create or initialize the state object
                    Mutex::new(RepartitionExecState::new(
                        childguard,
                        contextclone,
                    ))
                })
                .await;

            let state = state.lock();

            eprintln!("repartitionmsj test block");
            eprintln!("{}", state.debugTester);

            // let child_stream = self.child.execute(partition, contextclone)?;
            // child_stream
            state.debugTester.clone()
        });

        stream
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

    fn children(&self) -> &[Arc<GroupByWrapperEnum>] {
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

    //input channel for each partition
    input: DistributionReceiver<MaybeNestedBatch>,

    //schema
    schema: NestedSchemaRef,

    reservation: SharedMemoryReservation,
}

impl Stream for MsjRepartitionStream {
    type Item = Result<SemiJoinResultBatch, DataFusionError>;


    //poll next function taken from repartition stream
    fn poll_next(mut self: Pin<&mut Self>, 
        cx: &mut Context) -> Poll<Option<Self::Item>> {
            loop {
                match self.input.recv().poll_unpin(cx) { //look at state for poll_unpin
                    Poll::Ready(Some(Some(v))) => { //if resultbatch gotten
                        if let Ok(batch) = &v {
                            self.reservation
                                .lock()
                                .shrink(batch.get_array_memory_size());
                        }
    
                        return Poll::Ready(Some(v));
                    }
                    Poll::Ready(Some(None)) => { //if no resultbatch is none
                        self.num_input_partitions_processed += 1;
    
                        if self.num_input_partitions == self.num_input_partitions_processed {
                            // all input partitions have finished sending batches
                            return Poll::Ready(None);
                        } else {
                            // other partitions still have data to send
                            continue;
                        }
                    }
                    Poll::Ready(None) => { //if no result
                        return Poll::Ready(None);
                    }
                    Poll::Pending => { //if pending
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

#[derive(Debug)]
pub enum GroupByWrapperEnum {
    RepartitionGroupBy(RepartitionGroupBy),
    Groupby(GroupBy),
}

pub trait GroupByWrapper: Debug + Send + Sync {
    fn schema(&self) -> &NestedSchemaRef;
    async fn materialize(
        &self,
        context: Arc<TaskContext>,
        partition: usize,
    ) -> Result<GroupedRelRef, DataFusionError>;
    fn child(&self) -> &Arc<dyn MultiSemiJoinWrapper>;
    fn metrics(&self) -> MetricsSet;
    fn as_json(&self, output: &mut String) -> Result<(), std::fmt::Error>;
    fn collect_metrics(&self, output_buffer: &mut String, indent: usize);
    fn group_on(&self) -> &[usize];
    fn partitioned(&self) -> bool;
    fn set_partitioned(&mut self, partitioned: bool);
}

#[derive(Debug)]
pub struct RepartitionGroupBy {
    child: GroupBy,
    partitions: usize,
}

impl RepartitionGroupBy {
    pub fn new(child: Arc<dyn MultiSemiJoinWrapper>, group_on: Vec<usize>) -> Self {
        let child = GroupBy::new(child, group_on);
        println!("RepartitionGroupBy new");
        Self {
            child,
            partitions: 0,
        }
    }

    pub fn child(&self) -> &GroupBy {
        &self.child
    }
}

impl GroupByWrapper for RepartitionGroupBy {
    fn schema(&self) -> &NestedSchemaRef {
        self.child.schema()
    }

    async fn materialize(
        &self,
        context: Arc<TaskContext>,
        partition: usize,
    ) -> Result<GroupedRelRef, DataFusionError> {
        println!("RepartitionGroupBy materialize on partition {}", partition);
        self.child.materialize(context, partition).await
    }

    fn child(&self) -> &Arc<dyn MultiSemiJoinWrapper> {
        self.child.child()
    }

    fn metrics(&self) -> MetricsSet {
        self.child.metrics()
    }

    fn as_json(&self, output: &mut String) -> Result<(), std::fmt::Error> {
        self.child.as_json(output)
    }

    fn collect_metrics(&self, output_buffer: &mut String, indent: usize) {
        self.child.collect_metrics(output_buffer, indent)
    }

    fn group_on(&self) -> &[usize] {
        self.child.group_on()
    }

    fn partitioned(&self) -> bool {
        self.child.partitioned()
    }

    fn set_partitioned(&mut self, partitioned: bool) {
        self.child.set_partitioned(partitioned)
    }
}

impl GroupByWrapper for GroupByWrapperEnum {
    fn schema(&self) -> &NestedSchemaRef {
        match self {
            GroupByWrapperEnum::RepartitionGroupBy(repartition) => repartition.schema(),
            GroupByWrapperEnum::Groupby(groupby) => groupby.schema(),
        }
    }

    async fn materialize(
        &self,
        context: Arc<TaskContext>,
        partition: usize,
    ) -> Result<GroupedRelRef, DataFusionError> {
        match self {
            GroupByWrapperEnum::RepartitionGroupBy(repartition) => {
                repartition.materialize(context, partition).await
            }
            GroupByWrapperEnum::Groupby(groupby) => groupby.materialize(context, partition).await,
        }
    }

    fn child(&self) -> &Arc<dyn MultiSemiJoinWrapper> {
        match self {
            GroupByWrapperEnum::RepartitionGroupBy(repartition) => repartition.child().child(),
            GroupByWrapperEnum::Groupby(groupby) => groupby.child(),
        }
    }

    fn metrics(&self) -> MetricsSet {
        match self {
            GroupByWrapperEnum::RepartitionGroupBy(repartition) => repartition.metrics(),
            GroupByWrapperEnum::Groupby(groupby) => groupby.metrics(),
        }
    }

    fn as_json(&self, output: &mut String) -> Result<(), std::fmt::Error> {
        match self {
            GroupByWrapperEnum::RepartitionGroupBy(repartition) => repartition.as_json(output),
            GroupByWrapperEnum::Groupby(groupby) => groupby.as_json(output),
        }
    }

    fn collect_metrics(&self, output_buffer: &mut String, indent: usize) {
        match self {
            GroupByWrapperEnum::RepartitionGroupBy(repartition) => {
                repartition.collect_metrics(output_buffer, indent)
            }
            GroupByWrapperEnum::Groupby(groupby) => groupby.collect_metrics(output_buffer, indent),
        }
    }

    fn group_on(&self) -> &[usize] {
        match self {
            GroupByWrapperEnum::RepartitionGroupBy(repartition) => repartition.group_on(),
            GroupByWrapperEnum::Groupby(groupby) => groupby.group_on(),
        }
    }

    fn partitioned(&self) -> bool {
        match self {
            GroupByWrapperEnum::RepartitionGroupBy(repartition) => repartition.partitioned(),
            GroupByWrapperEnum::Groupby(groupby) => groupby.partitioned(),
        }
    }

    fn set_partitioned(&mut self, partitioned: bool) {
        match self {
            GroupByWrapperEnum::RepartitionGroupBy(repartition) => {
                repartition.set_partitioned(partitioned)
            }
            GroupByWrapperEnum::Groupby(groupby) => groupby.set_partitioned(partitioned),
        }
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
        let repartition = RepartitionMultiSemiJoin::new(guard, vec![], vec![], 0);

        let context = Arc::new(TaskContext::default());

        let state = repartition.statetest(context);

        let word: Vec<String> = state.collect().await;

        // let state = state.lock();

        assert_eq!(word[0], "Test");


        Ok(())
    }
}
