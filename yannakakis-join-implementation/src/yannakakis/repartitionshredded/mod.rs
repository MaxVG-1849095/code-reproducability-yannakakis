//! Implementation of the repartitionexec operator for shredded records, implemented to partition data calculated via multisemijoin



use std::{fmt::write, sync::Arc};

use datafusion::{error::DataFusionError, execution::TaskContext, physical_plan::{DisplayAs, DisplayFormatType, ExecutionPlan}};

use super::multisemijoin::{self, MultiSemiJoin, SendableSemiJoinResultBatchStream};
// use datafusion::physical_plan::Partitioning;



#[derive(Debug)]
pub struct RepartitionShredded {
    //child operator to be repartitioned
    child: MultiSemiJoin,
    
    //amount of partitions
    partitions: usize,

    //partitioning scheme
    //TODO: implement partitioning scheme from datafusion (the way repartitionexec does it)
    // partitioning: Partitioning
}

impl RepartitionShredded {
    pub fn new(child: MultiSemiJoin) -> Self {
        Self {
            child,
            partitions: 0,
        }
    }

    pub fn child(&self) -> &MultiSemiJoin {
        &self.child
    }

    pub fn execute(&self, partition: usize, context: Arc<TaskContext>) -> Result<SendableSemiJoinResultBatchStream, DataFusionError> {
        let child_stream = self.child.execute(partition, context)?;
        Ok(child_stream)

    }
}

// impl DisplayAs for RepartitionShredded {
//     fn fmt_as(
//         &self,
//         t: DisplayFormatType,
//         f: &mut std::fmt::Formatter,
//     ) -> std::fmt::Result {
//         match t {
//             DisplayFormatType::Default => write!(f,  RepartitionShredded"),
//             DisplayFormatType::Verbose => write!(f,  RepartitionShredded"),
//         }
//     }
// }

// impl ExecutionPlan for RepartitionShredded {
//     fn as_any(&self) -> &dyn std::any::Any {
//         self
//     }

//     fn schema(&self) -> datafusion::arrow::datatypes::SchemaRef {
//         todo!()
//     }

//     fn output_partitioning(&self) -> datafusion::physical_plan::Partitioning {
//         todo!()
//     }

//     fn output_ordering(&self) -> Option<&[datafusion::physical_expr::PhysicalSortExpr]> {
//         todo!()
//     }

//     fn children(&self) -> Vec<std::sync::Arc<dyn ExecutionPlan>> {
//         vec![self.child.clone()]
//     }

//     fn with_new_children(
//         self: std::sync::Arc<Self>,
//         children: Vec<std::sync::Arc<dyn ExecutionPlan>>,
//     ) -> datafusion::error::Result<std::sync::Arc<dyn ExecutionPlan>> {
//         let new_self = RepartitionShredded {
//             child: children[0].clone(),
//         };
//         Ok(Arc::new(new_self))
//     }

//     fn execute(
//         &self,
//         partition: usize,
//         context: std::sync::Arc<datafusion::execution::TaskContext>,
//     ) -> datafusion::error::Result<datafusion::execution::SendableRecordBatchStream> {
//         let child_stream = self.child.execute(partition, context)?;
//         Ok(child_stream)
//     }
// }


#[cfg(test)]

mod tests {

    use std::error::Error;
    use datafusion::{
        arrow::{
            array::{Int8Array, RecordBatch, UInt8Array},
            datatypes::{DataType, Field, Schema},
            error::ArrowError,
        },
        physical_plan::memory::MemoryExec,
    };
    use futures::StreamExt;

    use crate::yannakakis::{data::SemiJoinResultBatch, groupby::GroupBy};

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

    /// Groupby R(a,b,c) on `groupby_cols`
    fn example_child(group_columns: Vec<usize>) -> Result<Arc<GroupBy>, DataFusionError> {
        let leaf = MultiSemiJoin::new(example_guard()?, vec![], vec![]);
        let groupby = GroupBy::new(leaf, group_columns);
        Ok(Arc::new(groupby))
    }

    fn example_msj() -> Result<MultiSemiJoin, DataFusionError> {
        let guard = example_guard()?;
        let child = example_child(vec![0])?;
        Ok(MultiSemiJoin::new(guard, vec![child], vec![vec![(0, 0)]]))
    }

    /// MultiSemiJoin::new with valid equijoin keys.
    /// Should not panic
    #[test]
    fn test_multisemijoin_new() {
        // R(a,b,c)
        let guard = example_guard().unwrap();
        // R(a,b,c) groupby a
        let child_a = example_child(vec![0]).unwrap();
        // R(a,b,c) groupby (a,b)
        let child_ab = example_child(vec![0, 1]).unwrap();

        let _semijoin = MultiSemiJoin::new(guard.clone(), vec![child_a], vec![vec![(0, 0)]]);
        let _semijoin = MultiSemiJoin::new(guard, vec![child_ab], vec![vec![(0, 0), (1, 1)]]);
    }


    // test wether the repartitionshredded operator makes the same output as a normal msj operator
    #[tokio::test]
    async fn test_execute_repartition_shredded() -> Result<(), Box<dyn Error>>{
        let guard1= example_guard().unwrap();
        let guard2= example_guard().unwrap();

        let semijoin1 = MultiSemiJoin::new(guard1, vec![], vec![]);
        let semijoin2 = MultiSemiJoin::new(guard2, vec![], vec![]);

        let repartition = RepartitionShredded::new(semijoin2);

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