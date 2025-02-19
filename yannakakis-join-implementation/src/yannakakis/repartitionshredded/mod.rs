// //! Implementation of the repartitionexec operator for shredded records, implemented to partition data calculated via multisemijoin

use std::{pin::Pin, sync::Arc};

use datafusion::{
    error::DataFusionError,
    execution::{RecordBatchStream, SendableRecordBatchStream, TaskContext},
    physical_plan::{metrics::MetricsSet, stream::RecordBatchStreamAdapter, ExecutionPlan},
};

use super::{
    data::{GroupedRelRef, NestedSchema, NestedSchemaRef},
    groupby::GroupBy,
    multisemijoin::{MultiSemiJoin, MultiSemiJoinStream, SendableSemiJoinResultBatchStream},
};
// // use datafusion::physical_plan::Partitioning;

use std::fmt::Debug;
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
    fn set_partitioned(&mut self, partitioned: bool);
    fn id(&self) -> usize;
    fn set_id(&mut self, id: usize);
}

//repartitionexec operator to be placed on top of a multisemijoin
#[derive(Debug)]
pub struct RepartitionMultiSemiJoin {
    //child operator to be repartitioned, to be converted to a vector of multisemijoin operators
    child: MultiSemiJoin,

    //amount of partitions
    partitions: usize,
    //partitioning scheme
    //TODO: implement partitioning scheme from datafusion (the way repartitionexec does it)
    // partitioning: Partitioning
}

impl RepartitionMultiSemiJoin {
    //create new repartitionexec operator, also creating the necessary multisemijoin operator(s)
    pub fn new(
        guard: Arc<dyn ExecutionPlan>,
        children: Vec<Arc<GroupByWrapperEnum>>,
        equijoin_keys: Vec<Vec<(usize, usize)>>,
    ) -> Self {
        println!("RepartitionMultiSemiJoin new");
        Self {
            child: MultiSemiJoin::new(guard, children, equijoin_keys),
            partitions: 0,
        }
    }

    pub fn child(&self) -> &MultiSemiJoin {
        &self.child
    }

    // execute function for repartitionexec operator, it is supposed to execute the child operator(s) and repartition the data it receives
    pub fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableSemiJoinResultBatchStream, DataFusionError> {
        // println!("RepartitionMultiSemiJoin execute on partition {}", partition);
        // println!("RepartitionMultiSemiJoin execute with id {} on partition {}", self.id(), partition);

        let guard_partitions = self.child.guard_partition_count();

        if guard_partitions == 1 {
            //if there is only one partition, no need to do anything special
            let child_stream = self.child.execute(partition, context)?;
            return Ok(child_stream);
        } else {
            //if there are multiple partitions, we need to combine the streams of the different partitions (or eventually repartition them into a specified amount)
            let child_stream = self.child.execute(partition, context)?;
            // let mut child_streams = Vec::new();
            // for part in 0..guard_partitions {
            //     let child_stream = self.child.execute(part, context)?;
            //     child_streams.push(Box::pin(child_stream));
            // }
            // let child_stream = combine_streams(self.child.schema().clone(), child_streams);
            return Ok(child_stream);
        }
    }
}

// fn combine_multisemijoin_streams(
//     schema: Arc<datafusion::arrow::datatypes::Schema>,
//     streams: Vec<Pin<Box<MultiSemiJoinStream>>>,
// ) -> Pin<Box<MultiSemiJoinStream>> {
//     let mock_obj = streams[0].example_from_self();

//     //combine all guard_streams
//     let mut guard_streams = Vec::new();
//     for stream in streams {
//         guard_streams.push(stream.guard_stream());
//     }
//     let guard_streams_combined = combine_streams(schema.clone(), guard_streams);
//     let schema = mock_obj.schema();
//     let materialized_children_futs = mock_obj.materialized_children_futs();
//     let semijoin_keys = mock_obj.semijoin_keys();
//     let semijoin_metrics = mock_obj.semijoin_metrics();
//     let hashes_buffer = mock_obj.hashes_buffer();
//     //turn it into a MultiSemiJoinStream and return
//     Box::pin(MultiSemiJoinStream::new(
//         schema.clone(),
//         guard_streams_combined,
//         materialized_children_futs.to_vec(),
//         semijoin_keys.clone(),
//         semijoin_metrics.clone(),
//         (*hashes_buffer).clone(),
//         None,
//     ))
// }

//function to combine an array of recordbatchstreams into one TODO: make this for msjstreams
fn combine_streams(
    schema: Arc<datafusion::arrow::datatypes::Schema>,
    streams: Vec<Pin<Box<dyn RecordBatchStream + Send>>>,
) -> Pin<Box<dyn RecordBatchStream + Send>> {
    //combine all streams (becomes selectall object so no recordbatch)
    let streams_combined = futures::stream::select_all(streams);
    //turn it into a recordbatchstream and return
    Box::pin(RecordBatchStreamAdapter::new(schema, streams_combined))
}

impl MultiSemiJoinWrapper for RepartitionMultiSemiJoin {
    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableSemiJoinResultBatchStream, DataFusionError> {
        // println!("RepartitionMultiSemiJoin execute");
        self.execute(partition, context)
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

    fn set_partitioned(&mut self, partitioned: bool) {
        self.child.set_partitioned(partitioned)
    }

    fn id(&self) -> usize {
        self.child.id()
    }

    fn set_id(&mut self, id: usize) {
        self.child.set_id(id)
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

    // test wether the repartitionshredded operator makes the same output as a normal msj operator
    #[tokio::test]
    async fn test_execute_repartition_shredded() -> Result<(), Box<dyn Error>> {
        let guard1 = example_guard().unwrap();
        let guard2 = example_guard().unwrap();

        let semijoin1 = MultiSemiJoin::new(guard1, vec![], vec![]);

        let repartition = RepartitionMultiSemiJoin::new(guard2, vec![], vec![]);

        let result1 = semijoin1.execute(0, Arc::new(TaskContext::default()))?;

        let result2 = repartition.execute(0, Arc::new(TaskContext::default()))?;

        let batches1 = result1
            .collect::<Vec<Result<SemiJoinResultBatch, DataFusionError>>>()
            .await;

        let batches2 = result2
            .collect::<Vec<Result<SemiJoinResultBatch, DataFusionError>>>()
            .await;

        assert_eq!(batches1.len(), batches2.len());

        Ok(())
    }
}
