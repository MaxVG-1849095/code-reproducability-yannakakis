//! The unnest operator, which is used to flatten a nested relation.
//! Has the same API and produces the same output as the [Flatten] operator, but the way they produce the output is different.
//! originally unnest.rs -max
use std::sync::Arc;

use crate::take::non_weighted_u32 as take_from_arrayref;
use crate::yannakakis::data::{
    Idx, NestedColumn, NestedRel, NestedSchema, NonSingularNestedColumn, SemiJoinResultBatch,
    SingularNestedColumn,
};
use crate::yannakakis::multisemijoin::SendableSemiJoinResultBatchStream;
use crate::yannakakis::repartitionshredded::MultiSemiJoinWrapper;
use crate::yannakakis::schema::YannakakisSchema;
use datafusion::arrow::array::{ArrayRef, RecordBatch};
use datafusion::arrow::datatypes::SchemaRef;
use datafusion::error::DataFusionError;
use datafusion::execution::{SendableRecordBatchStream, TaskContext};
use datafusion::physical_plan::metrics::{
    self, ExecutionPlanMetricsSet, MetricBuilder, MetricsSet,
};
use datafusion::physical_plan::stream::RecordBatchStreamAdapter;
use datafusion::physical_plan::{DisplayAs, DisplayFormatType, Distribution, ExecutionPlan};
use futures::StreamExt;

/// Metrics for the Unnest operator
#[derive(Clone, Debug)]
struct UnnestMetrics {
    /// Total time spent by unnest() function calls
    pub total_time: metrics::Time,

    /// Total time spent by unnesting nested columns
    pub unnest_nestedcols_time: metrics::Time,

    /// Total time spent by cloning arcs of joining columns
    pub clone_join_columns_time: metrics::Time,

    /// Number of input rows
    pub input_rows: metrics::Count,
    /// Number of output rows
    pub output_rows: metrics::Count,
}

impl UnnestMetrics {
    fn new(partition: usize, metrics: &ExecutionPlanMetricsSet) -> Self {
        let total_time = MetricBuilder::new(metrics).subset_time("unnest_time", partition);

        let unnest_nestedcols_time =
            MetricBuilder::new(metrics).subset_time("unnest_nestedcols", partition);
        let clone_join_columns_time =
            MetricBuilder::new(metrics).subset_time("duplicate_join_cols", partition);

        let input_rows = MetricBuilder::new(metrics).counter("input_rows", partition);
        let output_rows = MetricBuilder::new(metrics).counter("output_rows", partition);

        Self {
            total_time,
            unnest_nestedcols_time,
            clone_join_columns_time,
            input_rows,
            output_rows,
        }
    }
}

#[derive(Debug)]
pub struct Unnest {
    /// the operator that computes the [SemijoinResultBatch]s to flatten
    child: Arc<dyn MultiSemiJoinWrapper>,

    /// contains the completely unnested output schema
    schema: Arc<YannakakisSchema>,

    /// execution metrics
    metrics: ExecutionPlanMetricsSet,
}

impl Unnest {
    pub fn new(child: Arc<dyn MultiSemiJoinWrapper>) -> Self {
        let schema = Arc::new(YannakakisSchema::new(child.as_ref()));
        Self {
            child,
            schema,
            metrics: ExecutionPlanMetricsSet::new(),
        }
    }

    pub fn metrics(&self) -> MetricsSet {
        self.metrics.clone_inner()
    }

    /// Get a JSON representation of the Unnest node and all its descendants ([MultiSemiJoin] & [GroupBy]), including their metrics.
    /// The JSON representation is a string, without newlines
    pub fn as_json(&self) -> Result<String, std::fmt::Error> {
        fn write_metrics_as_json(
            metrics: &Option<MetricsSet>,
            output: &mut String,
        ) -> std::fmt::Result {
            write!(output, ", \"metrics\": [")?;

            let mut first = true;

            if let Some(metrics) = metrics {
                for metric in metrics.iter() {
                    if !first {
                        write!(output, ",")?;
                    }
                    write!(
                        output,
                        "{{ \"name\":\"{}\", \"value\": {} ",
                        metric.value().name(),
                        metric.value().as_usize()
                    )?;
                    if let Some(partition) = metric.partition() {
                        write!(output, ", \"partition\": {}", partition)?;
                    }
                    write!(output, "}}")?;
                    first = false;
                }
            }
            write!(output, "]")?;
            Ok(())
        }

        use std::fmt::Write;

        let mut output = String::with_capacity(5000);

        write!(output, "{{ \"operator\": \"Unnest\"")?;
        write_metrics_as_json(&Some(self.metrics()), &mut output)?;

        write!(output, ", \"children\": [")?;
        self.child.as_json(&mut output)?;

        write!(output, "]}}")?;

        Ok(output)
    }
}

impl DisplayAs for Unnest {
    fn fmt_as(
        &self,
        t: datafusion::physical_plan::DisplayFormatType,
        f: &mut std::fmt::Formatter,
    ) -> std::fmt::Result {
        match t {
            DisplayFormatType::Default | DisplayFormatType::Verbose => {
                write!(f, "Unnest")
            }
        }
    }
}

impl ExecutionPlan for Unnest {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn schema(&self) -> SchemaRef {
        self.schema.output_schema()
    }

    fn required_input_distribution(&self) -> Vec<Distribution> {
        // We are currently single-threaded.
        let n_children = self.children().len();
        vec![Distribution::SinglePartition; n_children]
    }

    fn output_partitioning(&self) -> datafusion::physical_plan::Partitioning {
        self.child.guard().output_partitioning()
    }

    fn output_ordering(&self) -> Option<&[datafusion::physical_expr::PhysicalSortExpr]> {
        None
    }

    fn children(&self) -> Vec<Arc<dyn ExecutionPlan>> {
        fn children_recursive(
            node: &dyn MultiSemiJoinWrapper,
            children_buffer: &mut Vec<Arc<dyn ExecutionPlan>>,
        ) {
            children_buffer.push(node.guard().clone());
            for grand_child in node.children() {
                children_recursive(grand_child.child().as_ref(), children_buffer);
            }
        }

        let mut children: Vec<Arc<dyn ExecutionPlan>> = Vec::new();
        children_recursive(self.child.as_ref(), &mut children);
        children
    }

    fn with_new_children(
        self: Arc<Self>,
        children: Vec<Arc<dyn ExecutionPlan>>,
    ) -> datafusion::error::Result<Arc<dyn ExecutionPlan>> {
        todo!()
    }

    fn execute(
        &self,
        partition: usize,
        context: Arc<TaskContext>,
    ) -> Result<SendableRecordBatchStream, DataFusionError> {
        let metrics = UnnestMetrics::new(partition, &self.metrics);

        // Get stream of SemiJoinResultBatches
        let batches = self.child.execute(partition, context)?;
        let yann_schema = &self.schema;

        // Flatten each batch, resulting in a stream of RecordBatches
        println!("unnesting batches for partition {}", partition);
        unnest_batches(batches, yann_schema.clone(), metrics)
    }
}

/// Maps a stream of [SemiJoinResultBatch]es to a stream of [RecordBatch]es by unnesting each batch.
#[inline(always)]
fn unnest_batches(
    batches: SendableSemiJoinResultBatchStream,
    output_schema: Arc<YannakakisSchema>, // the *flat* output schema
    metrics: UnnestMetrics,
) -> Result<SendableRecordBatchStream, DataFusionError> {
    let schema = output_schema.clone(); // clone for usage after closure

    let stream = batches.map(move |batch| {
        // println!("unnesting batch with {}rows", batch.num_rows());
        match batch{
            Ok(batch)=>{
                // println!("batch num rows: {}", batch.num_rows());
                unnest(batch, &output_schema, &metrics)
            }
            Err(e)=>{
                println!("error: {}", e);
                Err(e)
            }
        }
        });
    Ok(Box::pin(RecordBatchStreamAdapter::new(
        schema.output_schema(),
        stream,
    )))
}

struct UnnestResult {
    /// Already unnested arrays
    pub arrays: Vec<ArrayRef>,

    /// Nested columns that still need to be unnested.
    /// Stored in reversed order: the last element corresponds to the nested column that should be unnested first.
    /// This way, the vec can be used as a stack.
    pub nested_cols: Vec<NestedColumn>,
}

impl UnnestResult {
    fn new(capacity: usize) -> Self {
        Self {
            arrays: Vec::with_capacity(capacity),
            nested_cols: Vec::with_capacity(capacity), // possible overallocation here: there will never be more nested columns than columns in the flat output schema
        }
    }
}

/// Keep unnesting nested columns of `batch` until the result is a flat batch.
pub fn unnest(
    batch: SemiJoinResultBatch,
    output_schema: &YannakakisSchema,
    metrics: &UnnestMetrics,
) -> Result<RecordBatch, DataFusionError> {
    metrics.input_rows.add(batch.num_rows());
    let total_time = metrics.total_time.timer();

    let result: RecordBatch = match batch {
        SemiJoinResultBatch::Flat(batch) => batch,
        SemiJoinResultBatch::Nested(mut nestedbatch) => {
            // println!("unnesting batch");
            // Pre-allocate two buffers that will be used throughout the unnesting.
            let sum_weights = nestedbatch.total_weights().iter().sum::<Idx>();
            let mut buffer1 = Vec::<Idx>::with_capacity(sum_weights as usize);
            let mut buffer2 = Vec::<Idx>::with_capacity(sum_weights as usize);

            // Buffer for storing output arrays
            let n_cols = output_schema.n_unnest_fields();
            let mut unnest_result = UnnestResult::new(n_cols);

            // Fill output with data from regular fields
            let regular_fields = &nestedbatch.schema().regular_fields;
            for i in 0..regular_fields.fields().len() {
                let col = nestedbatch.regular_column(i);
                unnest_result.arrays.push(col.clone());
            }

            let unnest_timer = metrics.unnest_nestedcols_time.timer();

            // Unnest nested columns
            let n_nested_fields = nestedbatch.schema().nested_fields.len();
            (0..n_nested_fields).rev().for_each(|i| {
                unnest_result
                    .nested_cols
                    .push(match &mut nestedbatch.inner.nested_cols[i] {
                        NestedColumn::Singular(singular_nested_column) => {
                            NestedColumn::Singular(std::mem::take(singular_nested_column))
                        }
                        NestedColumn::NonSingular(non_singular_nested_column) => {
                            NestedColumn::NonSingular(std::mem::take(non_singular_nested_column))
                        }
                    });
            });

            unnest_nestedcols(
                &nestedbatch.schema(),
                &mut unnest_result,
                &mut buffer1,
                &mut buffer2,
            )?;

            unnest_timer.done();

            // Duplicate output columns for join columns
            let timer = metrics.clone_join_columns_time.timer();
            let batch = output_schema.build_batch(&mut unnest_result.arrays)?;
            timer.done();
            batch
        }
    };

    total_time.done();
    metrics.output_rows.add(result.num_rows());

    Ok(result)
}

fn unnest_singular_nestedcol(
    col: &SingularNestedColumn,
    result: &mut UnnestResult,
    buffer: &mut Vec<Idx>,
) -> Result<(), DataFusionError> {
    // Unnesting a singular nested column does not produce additional columns.
    // However, the already unnested columns (and remaining nested columns) must be updated according to the weights in `col`.

    buffer.clear();

    buffer.extend(
        col.weights()
            .iter()
            .enumerate()
            .flat_map(|(i, w)| std::iter::repeat(i as Idx).take(*w as usize)),
    );

    // Expand columns of previously unnested columns
    for col in result.arrays.iter_mut() {
        *col = take_from_arrayref::take(col.as_ref(), &buffer)?;
    }

    // Do the same for NestedColumns that still need to be unnested
    for nestedcol in result.nested_cols.iter_mut() {
        *nestedcol = take_rows_from_nestedcol(nestedcol, &buffer)?;
    }

    Ok(())
}

fn unnest_nonsingular_nestedcol(
    col: &NonSingularNestedColumn,
    result: &mut UnnestResult,
    buffer1: &mut Vec<Idx>,
    buffer2: &mut Vec<Idx>,
) -> Result<(), DataFusionError> {
    unnest_nestedrel(&col.data, col.hols(), result, buffer1, buffer2)
}

/// Create new `Vec<u32>` by taking the elements in `data` at the positions in `indices`.
/// # Panics
/// Panics if an index in `indices` is out of bounds for `data`.
#[inline(always)]
fn take(data: &[u32], indices: &[Idx]) -> Result<Vec<u32>, DataFusionError> {
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
            let new_weights = take(&s_nestedcol.weights, row_ids)?;
            let nestedcol = SingularNestedColumn {
                weights: new_weights,
            };
            Ok(NestedColumn::Singular(nestedcol))
        }
        NestedColumn::NonSingular(ns_nestedcol) => {
            let new_weights = take(&ns_nestedcol.weights, row_ids)?;
            let new_hols = take(&ns_nestedcol.hols, row_ids)?;
            // let new_data = ns_nestedcol.clone_data(); // ! changed this to make a deep copy of the data


            let nestedcol = NonSingularNestedColumn {
                weights: new_weights,
                hols: new_hols,
                data: ns_nestedcol.data.clone(), // clone arc = cheap
                // data: Arc::new(new_data),
            };

            Ok(NestedColumn::NonSingular(nestedcol))
        }
    }
}

#[inline(always)]
fn unnest_nestedrel(
    rel: &Arc<NestedRel>,
    ptrs: &[Idx],
    result: &mut UnnestResult,
    row_ids: &mut Vec<Idx>, // pre-allocated buffer for storing row_ids produced by iterating over LL of each hol-ptr in `ptrs`.
    ptr_positions: &mut Vec<Idx>, // pre-allocated buffer. For each row_id in `row_ids`, the position of the hol-ptr in `ptrs` that produced row_id.
) -> Result<(), DataFusionError> {
    row_ids.clear();
    ptr_positions.clear();

    for (i, ptr) in ptrs.iter().enumerate() {
        let old_len = row_ids.len();
        row_ids.extend(rel.iterate_linked_list(*ptr));

        // number of items produced by `ptr`
        let added = row_ids.len() - old_len;
        ptr_positions.extend(std::iter::repeat(i as Idx).take(added));
    }

    // Expand columns of previously unnested columns
    for col in result.arrays.iter_mut() {
        *col = take_from_arrayref::take(col.as_ref(), &ptr_positions)?;
    }

    // Do the same for NestedColumns that still need to be unnested
    for nestedcol in result.nested_cols.iter_mut() {
        *nestedcol = take_rows_from_nestedcol(nestedcol, &ptr_positions)?;
    }

    // Create result column for each regular field of `rel`
    for i in 0..rel.schema().regular_fields.fields().len() {
        let col = rel.regular_column(i);
        result
            .arrays
            .push(take_from_arrayref::take(col.as_ref(), &row_ids)?);
    }


    // If the [NestedRel] has itself nested columns, we have to unnest them too.
    for i in (0..rel.schema().nested_fields.len()).rev() {
        let nestedcol = take_rows_from_nestedcol(rel.nested_column(i), &row_ids)?;
        result.nested_cols.push(nestedcol);
    }

    unnest_nestedcols(&rel.schema, result, row_ids, ptr_positions)
}

/// Unnest all [NestedColumn]s of `schema.nested_fields`.
/// The actual nested columns to be unnested are popped from `result.nested_cols`.
fn unnest_nestedcols(
    schema: &NestedSchema,
    result: &mut UnnestResult,
    buffer1: &mut Vec<Idx>,
    buffer2: &mut Vec<Idx>,
) -> Result<(), DataFusionError> {
    let n_nestedcols = schema.nested_fields.len();

    for _ in 0..n_nestedcols {
        let col = result
            .nested_cols
            .pop()
            .expect("Expected a nested column to unnest");
        match col {
            NestedColumn::Singular(s_nestedcol) => {
                unnest_singular_nestedcol(&s_nestedcol, result, buffer1)?;
            }
            NestedColumn::NonSingular(ns_nestedcol) => {
                unnest_nonsingular_nestedcol(&ns_nestedcol, result, buffer1, buffer2)?;
            }
        }
    }

    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use std::{error::Error, vec};

//     use crate::yannakakis::{groupby::GroupBy, multisemijoin::MultiSemiJoin};
//     use datafusion::{
//         arrow::{
//             array::{Int32Array, UInt32Array},
//             datatypes::{DataType, Field, Schema},
//             util::pretty::pretty_format_batches,
//         },
//         execution::TaskContext,
//         physical_plan::{collect, memory::MemoryExec, ExecutionPlan},
//     };

//     use super::*;

//     fn binary_semijoin(
//         guard: Arc<dyn ExecutionPlan>,
//         child1: Arc<dyn ExecutionPlan>,
//         child1_groupby: Vec<usize>,
//         child1_join_cols: Vec<(usize, usize)>,
//     ) -> Result<MultiSemiJoin, DataFusionError> {
//         // Binary semijoin between R, S (guard=R, children=[S])

//         let s = MultiSemiJoin::new(child1, vec![], vec![]);
//         let s_grouped = Arc::new(GroupBy::new(s, child1_groupby));

//         let binary_semijoin = MultiSemiJoin::new(guard, vec![s_grouped], vec![child1_join_cols]);

//         Ok(binary_semijoin)
//     }

//     fn ternary_semijoin(
//         guard: Arc<dyn ExecutionPlan>,
//         child1: Arc<dyn ExecutionPlan>,
//         child1_groupby: Vec<usize>,
//         child1_join_cols: Vec<(usize, usize)>,
//         child2: Arc<dyn ExecutionPlan>,
//         child2_groupby: Vec<usize>,
//         child2_join_cols: Vec<(usize, usize)>,
//     ) -> Result<MultiSemiJoin, DataFusionError> {
//         // Ternary semijoin between R, S, and T (guard=R, children=[S,T])

//         let s = MultiSemiJoin::new(child1, vec![], vec![]);
//         let s_grouped = Arc::new(GroupBy::new(s, child1_groupby));
//         let t = MultiSemiJoin::new(child2, vec![], vec![]);
//         let t_grouped = Arc::new(GroupBy::new(t, child2_groupby));

//         let ternary_semijoin = MultiSemiJoin::new(
//             guard,
//             vec![s_grouped, t_grouped],
//             vec![child1_join_cols, child2_join_cols],
//         );

//         Ok(ternary_semijoin)
//     }

//     fn quaternary_semijoin(
//         guard: Arc<dyn ExecutionPlan>,
//         child1: Arc<dyn ExecutionPlan>,
//         child1_groupby: Vec<usize>,
//         child1_join_cols: Vec<(usize, usize)>,
//         child2: Arc<dyn ExecutionPlan>,
//         child2_groupby: Vec<usize>,
//         child2_join_cols: Vec<(usize, usize)>,
//         child3: Arc<dyn ExecutionPlan>,
//         child3_groupby: Vec<usize>,
//         child3_join_cols: Vec<(usize, usize)>,
//     ) -> Result<MultiSemiJoin, DataFusionError> {
//         // Quaternary semijoin between R, S, T, and U (guard=R, children=[S,T,U])

//         let s = MultiSemiJoin::new(child1, vec![], vec![]);
//         let s_grouped = Arc::new(GroupBy::new(s, child1_groupby));
//         let t = MultiSemiJoin::new(child2, vec![], vec![]);
//         let t_grouped = Arc::new(GroupBy::new(t, child2_groupby));
//         let u = MultiSemiJoin::new(child3, vec![], vec![]);
//         let u_grouped = Arc::new(GroupBy::new(u, child3_groupby));

//         let quaternary_semijoin = MultiSemiJoin::new(
//             guard,
//             vec![s_grouped, t_grouped, u_grouped],
//             vec![child1_join_cols, child2_join_cols, child3_join_cols],
//         );

//         Ok(quaternary_semijoin)
//     }

//     /// R(a,b), S(b,c), T(c,d)
//     ///
//     /// Right-deep semijoin tree: R ⋉ (S ⋉ T)
//     fn threeway_path_semijoin(
//         r: Arc<dyn ExecutionPlan>,
//         s: Arc<dyn ExecutionPlan>,
//         t: Arc<dyn ExecutionPlan>,
//     ) -> MultiSemiJoin {
//         let t = MultiSemiJoin::new(t, vec![], vec![]);
//         let t_grouped = Arc::new(GroupBy::new(t, vec![0]));

//         let s = MultiSemiJoin::new(s, vec![t_grouped], vec![vec![(1, 0)]]);
//         let s_grouped = Arc::new(GroupBy::new(s, vec![0]));

//         let r = MultiSemiJoin::new(r, vec![s_grouped], vec![vec![(1, 0)]]);
//         r
//     }

//     /// Create Arrow schema from column names and types.
//     /// If `nullable` is `Some`, then it should be a vector of booleans indicating whether each column is nullable.
//     /// If `nullable` is `None`, then all columns are non-nullable.
//     fn create_schema(names: &[&str], types: &[DataType], nullable: Option<&[bool]>) -> Arc<Schema> {
//         match nullable {
//             Some(nullable) => Arc::new(Schema::new(
//                 names
//                     .iter()
//                     .zip(types.iter())
//                     .zip(nullable.iter())
//                     .map(|((name, data_type), nullable)| {
//                         Field::new(name.to_string(), data_type.clone(), *nullable)
//                     })
//                     .collect::<Vec<_>>(),
//             )),
//             None => Arc::new(Schema::new(
//                 names
//                     .iter()
//                     .zip(types.iter())
//                     .map(|(name, data_type)| Field::new(name.to_string(), data_type.clone(), false))
//                     .collect::<Vec<_>>(),
//             )),
//         }
//     }

//     #[tokio::test]
//     async fn unnest_binary_semijoin_one_to_one() -> Result<(), Box<dyn Error>> {
//         use DataType::*;

//         let r_schema = create_schema(&["a", "b", "c"], &[UInt32, UInt32, UInt32], None);
//         let s_schema = create_schema(&["d", "e", "f"], &[UInt32, UInt32, UInt32], None);

//         let columns: Vec<ArrayRef> = vec![
//             Arc::new(UInt32Array::from(vec![1, 2, 3, 4, 5])),
//             Arc::new(UInt32Array::from(vec![1, 2, 3, 4, 5])),
//             Arc::new(UInt32Array::from(vec![1, 2, 3, 4, 5])),
//         ];

//         let r = vec![RecordBatch::try_new(r_schema.clone(), columns.clone())?];
//         let s = vec![RecordBatch::try_new(s_schema.clone(), columns)?];

//         let r = Arc::new(MemoryExec::try_new(&[r], r_schema, None)?);
//         let s = Arc::new(MemoryExec::try_new(&[s], s_schema, None)?);

//         // Join on 1, 2 and 3 (all) columns.
//         // For each case, the result is the same.
//         for i in 1..=3 {
//             let groupby = (0..i).collect();
//             let join_on = (0..i).map(|j| (j, j)).collect();
//             let plan = binary_semijoin(r.clone(), s.clone(), groupby, join_on)?;
//             let plan = Arc::new(Unnest::new(Arc::new(plan)));

//             let results = collect(plan, Arc::new(TaskContext::default())).await?;
//             assert!(results.len() == 1);
//             let batch = &results[0];

//             println!(
//                 "{}",
//                 pretty_format_batches(&[batch.clone()]).unwrap().to_string()
//             );

//             assert!(batch.num_columns() == 6);
//             assert!(batch
//                 .columns()
//                 .iter()
//                 .all(|col| col.as_ref() == &UInt32Array::from(vec![1, 2, 3, 4, 5])));
//         }

//         Ok(())
//     }

//     #[tokio::test]
//     async fn unnest_binary_semijoin_all_to_all() -> Result<(), Box<dyn Error>> {
//         use DataType::*;

//         let r_schema = create_schema(&["a", "b", "c"], &[UInt32, UInt32, UInt32], None);
//         let s_schema = create_schema(&["d", "e", "f"], &[UInt32, UInt32, UInt32], None);

//         let columns: Vec<ArrayRef> = vec![
//             Arc::new(UInt32Array::from(vec![1, 1, 1, 1, 1])),
//             Arc::new(UInt32Array::from(vec![1, 1, 1, 1, 1])),
//             Arc::new(UInt32Array::from(vec![1, 1, 1, 1, 1])),
//         ];

//         let r = vec![RecordBatch::try_new(r_schema.clone(), columns.clone())?];
//         let s = vec![RecordBatch::try_new(s_schema.clone(), columns)?];

//         let r = Arc::new(MemoryExec::try_new(&[r], r_schema, None)?);
//         let s = Arc::new(MemoryExec::try_new(&[s], s_schema, None)?);

//         // Join on 1, 2 and 3 (all) columns.
//         // For each case, the result is the same.
//         for i in 1..=3 {
//             let plan = binary_semijoin(
//                 r.clone(),
//                 s.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//             )?;
//             let plan = Arc::new(Unnest::new(Arc::new(plan)));

//             let results = collect(plan, Arc::new(TaskContext::default())).await?;
//             assert!(results.len() == 1);
//             let batch = &results[0];

//             assert!(batch.num_columns() == 6);
//             assert!(batch
//                 .columns()
//                 .iter()
//                 .all(|col| col.as_ref() == &UInt32Array::from(vec![1; 25])));
//         }

//         Ok(())
//     }

//     #[tokio::test]
//     async fn threeway_path_join_one_to_one() -> Result<(), Box<dyn Error>> {
//         use DataType::*;

//         let r_schema = create_schema(&["a", "b"], &[UInt32, UInt32], None);
//         let s_schema = create_schema(&["b", "c"], &[UInt32, UInt32], None);
//         let t_schema = create_schema(&["c", "d"], &[UInt32, UInt32], None);

//         let columns: Vec<ArrayRef> = vec![
//             Arc::new(UInt32Array::from(vec![1, 2, 3, 4, 5])),
//             Arc::new(UInt32Array::from(vec![1, 2, 3, 4, 5])),
//         ];

//         let r = vec![RecordBatch::try_new(r_schema.clone(), columns.clone())?];
//         let s = vec![RecordBatch::try_new(s_schema.clone(), columns.clone())?];
//         let t = vec![RecordBatch::try_new(t_schema.clone(), columns)?];

//         let r = Arc::new(MemoryExec::try_new(&[r], r_schema, None)?);
//         let s = Arc::new(MemoryExec::try_new(&[s], s_schema, None)?);
//         let t = Arc::new(MemoryExec::try_new(&[t], t_schema, None)?);

//         let plan = threeway_path_semijoin(r, s, t);
//         let plan = Arc::new(Unnest::new(Arc::new(plan)));

//         let results = collect(plan, Arc::new(TaskContext::default())).await?;

//         assert!(results.len() == 1);
//         let batch = &results[0];

//         assert!(batch.num_columns() == 6);
//         assert!(batch
//             .columns()
//             .iter()
//             .all(|col| col.as_ref() == &UInt32Array::from(vec![1, 2, 3, 4, 5])));

//         Ok(())
//     }

//     #[tokio::test]
//     async fn threeway_path_join_all_to_all() -> Result<(), Box<dyn Error>> {
//         use DataType::*;

//         let r_schema = create_schema(&["a", "b"], &[UInt32, UInt32], None);
//         let s_schema = create_schema(&["b", "c"], &[UInt32, UInt32], None);
//         let t_schema = create_schema(&["c", "d"], &[UInt32, UInt32], None);

//         let columns: Vec<ArrayRef> = vec![
//             Arc::new(UInt32Array::from(vec![1; 3])),
//             Arc::new(UInt32Array::from(vec![1; 3])),
//         ];

//         let r = vec![RecordBatch::try_new(r_schema.clone(), columns.clone())?];
//         let s = vec![RecordBatch::try_new(s_schema.clone(), columns.clone())?];
//         let t = vec![RecordBatch::try_new(t_schema.clone(), columns)?];

//         let r = Arc::new(MemoryExec::try_new(&[r], r_schema, None)?);
//         let s = Arc::new(MemoryExec::try_new(&[s], s_schema, None)?);
//         let t = Arc::new(MemoryExec::try_new(&[t], t_schema, None)?);

//         let plan = threeway_path_semijoin(r, s, t);
//         let plan = Arc::new(Unnest::new(Arc::new(plan)));

//         let results = collect(plan, Arc::new(TaskContext::default())).await?;

//         assert!(results.len() == 1);
//         let batch = &results[0];

//         assert!(batch.num_columns() == 6);
//         assert!(batch
//             .columns()
//             .iter()
//             .all(|col| col.as_ref() == &UInt32Array::from(vec![1; 3 * 3 * 3])));

//         Ok(())
//     }

//     #[tokio::test]
//     /// Test flattening a ternary semijoin between R, S, and T.
//     /// R(a,b,c), S(d,e,f), T(g,h,i)
//     /// The data is chosen such that R-S and R-T are one-to-one semijoins.
//     async fn unnest_ternary_semijoin_one_to_one() -> Result<(), DataFusionError> {
//         use DataType::*;

//         let r_schema = create_schema(&["a", "b", "c"], &[Int32, Int32, Int32], None);
//         let s_schema = create_schema(&["d", "e", "f"], &[Int32, Int32, Int32], None);
//         let t_schema = create_schema(&["g", "h", "i"], &[Int32, Int32, Int32], None);

//         let columns: Vec<ArrayRef> = vec![
//             Arc::new(Int32Array::from(vec![1, 2, 3, 4, 5])),
//             Arc::new(Int32Array::from(vec![1, 2, 3, 4, 5])),
//             Arc::new(Int32Array::from(vec![1, 2, 3, 4, 5])),
//         ];

//         let r = vec![RecordBatch::try_new(r_schema.clone(), columns.clone())?];
//         let s = vec![RecordBatch::try_new(s_schema.clone(), columns.clone())?];
//         let t = vec![RecordBatch::try_new(t_schema.clone(), columns)?];

//         let r = Arc::new(MemoryExec::try_new(&[r], r_schema, None)?);
//         let s = Arc::new(MemoryExec::try_new(&[s], s_schema, None)?);
//         let t = Arc::new(MemoryExec::try_new(&[t], t_schema, None)?);

//         // Join on 1, 2 and 3 (all) columns.
//         // For each case, the result is the same.
//         for i in 1..=3 {
//             let plan = ternary_semijoin(
//                 r.clone(),
//                 s.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//                 t.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//             )?;
//             let plan = Arc::new(Unnest::new(Arc::new(plan)));
//             let results = collect(plan, Arc::new(TaskContext::default())).await?;
//             assert!(results.len() == 1);
//             let batch = &results[0];
//             println!(
//                 "{}",
//                 pretty_format_batches(&[batch.clone()]).unwrap().to_string()
//             );

//             assert!(batch.num_columns() == 9);
//             assert!(batch
//                 .columns()
//                 .iter()
//                 .all(|col| col.as_ref() == &Int32Array::from(vec![1, 2, 3, 4, 5])));
//         }

//         Ok(())
//     }

//     #[tokio::test]
//     /// Test flattening a ternary semijoin between R, S, and T.
//     /// R(a,b,c), S(d,e,f), T(g,h,i)
//     /// The data is chosen such that R-S and R-T are all_to_all joins (cartesian product).
//     async fn unnest_ternary_semijoin_all_to_all() -> Result<(), DataFusionError> {
//         use DataType::*;

//         let r_schema = create_schema(&["a", "b", "c"], &[Int32, Int32, Int32], None);
//         let s_schema = create_schema(&["d", "e", "f"], &[Int32, Int32, Int32], None);
//         let t_schema = create_schema(&["g", "h", "i"], &[Int32, Int32, Int32], None);

//         let columns: Vec<ArrayRef> = vec![
//             Arc::new(Int32Array::from(vec![1, 1])),
//             Arc::new(Int32Array::from(vec![1, 1])),
//             Arc::new(Int32Array::from(vec![1, 1])),
//         ];

//         let r = vec![RecordBatch::try_new(r_schema.clone(), columns.clone())?];
//         let s = vec![RecordBatch::try_new(s_schema.clone(), columns.clone())?];
//         let t = vec![RecordBatch::try_new(t_schema.clone(), columns)?];

//         let r = Arc::new(MemoryExec::try_new(&[r], r_schema, None)?);
//         let s = Arc::new(MemoryExec::try_new(&[s], s_schema, None)?);
//         let t = Arc::new(MemoryExec::try_new(&[t], t_schema, None)?);

//         // Join on 1, 2 and 3 (all) columns.
//         // For each case, the result is the same.
//         for i in 1..=3 {
//             let plan = ternary_semijoin(
//                 r.clone(),
//                 s.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//                 t.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//             )?;
//             let plan = Arc::new(Unnest::new(Arc::new(plan)));
//             let results = collect(plan, Arc::new(TaskContext::default())).await?;
//             assert!(results.len() == 1);
//             let batch = &results[0];

//             println!(
//                 "{}",
//                 pretty_format_batches(&[batch.clone()]).unwrap().to_string()
//             );

//             assert!(batch.num_columns() == 9);
//             assert!(batch
//                 .columns()
//                 .iter()
//                 .all(|col| col.as_ref() == &Int32Array::from(vec![1; 8])));
//         }

//         Ok(())
//     }

//     #[tokio::test]
//     /// Test flattening a quaternary semijoin between R, S, T, and U.
//     /// Each relation has 4 columns.
//     /// The data is chosen such that R-S, R-T, and R-U are one-to-one joins
//     async fn unnest_quaternary_semijoin_one_to_one() -> Result<(), DataFusionError> {
//         use DataType::*;

//         let r_schema = create_schema(&["a", "b", "c", "d"], &[Int32, Int32, Int32, Int32], None);
//         let s_schema = create_schema(&["e", "f", "g", "h"], &[Int32, Int32, Int32, Int32], None);
//         let t_schema = create_schema(&["i", "j", "k", "l"], &[Int32, Int32, Int32, Int32], None);
//         let u_schema = create_schema(&["m", "n", "o", "p"], &[Int32, Int32, Int32, Int32], None);

//         let columns: Vec<ArrayRef> = vec![
//             Arc::new(Int32Array::from(vec![1, 2])),
//             Arc::new(Int32Array::from(vec![1, 2])),
//             Arc::new(Int32Array::from(vec![1, 2])),
//             Arc::new(Int32Array::from(vec![1, 2])),
//         ];

//         let r = vec![RecordBatch::try_new(r_schema.clone(), columns.clone())?];
//         let s = vec![RecordBatch::try_new(s_schema.clone(), columns.clone())?];
//         let t = vec![RecordBatch::try_new(t_schema.clone(), columns.clone())?];
//         let u = vec![RecordBatch::try_new(u_schema.clone(), columns)?];

//         let r = Arc::new(MemoryExec::try_new(&[r], r_schema, None)?);
//         let s = Arc::new(MemoryExec::try_new(&[s], s_schema, None)?);
//         let t = Arc::new(MemoryExec::try_new(&[t], t_schema, None)?);
//         let u = Arc::new(MemoryExec::try_new(&[u], u_schema, None)?);

//         // Join on 1, 2, and 3 columns.
//         // For each case, the result is the same.
//         for i in 1..=3 {
//             let plan = quaternary_semijoin(
//                 r.clone(),
//                 s.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//                 t.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//                 u.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//             )?;
//             let plan = Arc::new(Unnest::new(Arc::new(plan)));
//             let results = collect(plan, Arc::new(TaskContext::default())).await?;
//             assert!(results.len() == 1);
//             let batch = &results[0];
//             assert!(batch.num_columns() == 16); // 4 columns * 4 tables
//             assert!(batch.num_rows() == 2);
//             assert!(batch
//                 .columns()
//                 .iter()
//                 .all(|col| col.as_ref() == &Int32Array::from(vec![1, 2])));
//         }

//         Ok(())
//     }

//     #[tokio::test]
//     /// Test flattening a quaternary semijoin between R, S, T, and U.
//     /// Each relation has 4 columns.
//     /// The data is chosen such that R-S, R-T, and R-U are all_to_all joins (cartesian product).
//     async fn unnest_quaternary_semijoin_all_to_all() -> Result<(), DataFusionError> {
//         use DataType::*;

//         let r_schema = create_schema(&["a", "b", "c", "d"], &[Int32, Int32, Int32, Int32], None);
//         let s_schema = create_schema(&["e", "f", "g", "h"], &[Int32, Int32, Int32, Int32], None);
//         let t_schema = create_schema(&["i", "j", "k", "l"], &[Int32, Int32, Int32, Int32], None);
//         let u_schema = create_schema(&["m", "n", "o", "p"], &[Int32, Int32, Int32, Int32], None);

//         let columns: Vec<ArrayRef> = vec![
//             Arc::new(Int32Array::from(vec![1, 1])),
//             Arc::new(Int32Array::from(vec![1, 1])),
//             Arc::new(Int32Array::from(vec![1, 1])),
//             Arc::new(Int32Array::from(vec![1, 1])),
//         ];

//         let r = vec![RecordBatch::try_new(r_schema.clone(), columns.clone())?];
//         let s = vec![RecordBatch::try_new(s_schema.clone(), columns.clone())?];
//         let t = vec![RecordBatch::try_new(t_schema.clone(), columns.clone())?];
//         let u = vec![RecordBatch::try_new(u_schema.clone(), columns)?];

//         let r = Arc::new(MemoryExec::try_new(&[r], r_schema, None)?);
//         let s = Arc::new(MemoryExec::try_new(&[s], s_schema, None)?);
//         let t = Arc::new(MemoryExec::try_new(&[t], t_schema, None)?);
//         let u = Arc::new(MemoryExec::try_new(&[u], u_schema, None)?);

//         // Join on 1, 2, and 3 columns.
//         // For each case, the result is the same.
//         for i in 1..=3 {
//             let plan = quaternary_semijoin(
//                 r.clone(),
//                 s.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//                 t.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//                 u.clone(),
//                 (0..i).collect(),
//                 (0..i).map(|j| (j, j)).collect(),
//             )?;
//             let plan = Arc::new(Unnest::new(Arc::new(plan)));
//             let results = collect(plan, Arc::new(TaskContext::default())).await?;
//             assert!(results.len() == 1);
//             let batch = &results[0];
//             assert!(batch.num_columns() == 16); // 4 columns * 4 tables
//             assert!(batch.num_rows() == 16);
//             assert!(batch
//                 .columns()
//                 .iter()
//                 .all(|col| col.as_ref() == &Int32Array::from(vec![1; 16])));
//         }

//         Ok(())
//     }

//     #[tokio::test]
//     async fn ternary_semijoin_ontopof_quaternary() -> Result<(), DataFusionError> {
//         use DataType::*;

//         // quaternary semijoin (guard=R, children=[S,T])
//         let r_schema = create_schema(&["a", "b"], &[Int32, Int32], None);
//         let s_schema = create_schema(&["b", "c"], &[Int32, Int32], None);
//         let t_schema = create_schema(&["b", "d"], &[Int32, Int32], None);
//         let v_schema = create_schema(&["b", "e"], &[Int32, Int32], None);

//         let columns: Vec<ArrayRef> = vec![
//             Arc::new(Int32Array::from(vec![1, 1])),
//             Arc::new(Int32Array::from(vec![1, 1])),
//         ];

//         let r = vec![RecordBatch::try_new(r_schema.clone(), columns.clone())?];
//         let s = vec![RecordBatch::try_new(s_schema.clone(), columns.clone())?];
//         let t = vec![RecordBatch::try_new(t_schema.clone(), columns.clone())?];
//         let v = vec![RecordBatch::try_new(v_schema.clone(), columns.clone())?];

//         let r = Arc::new(MemoryExec::try_new(&[r], r_schema, None)?);
//         let s = Arc::new(MemoryExec::try_new(&[s], s_schema, None)?);
//         let t = Arc::new(MemoryExec::try_new(&[t], t_schema, None)?);
//         let v = Arc::new(MemoryExec::try_new(&[v], v_schema, None)?);

//         let quaternary = quaternary_semijoin(
//             r,
//             s,
//             vec![0],
//             vec![(0, 0)],
//             t,
//             vec![0],
//             vec![(1, 0)],
//             v,
//             vec![0],
//             vec![(1, 0)],
//         )?;

//         // add ternary semijoin on top of it (guard=guard, children=[(R⋉S⋉T), U])

//         let guard_schema = create_schema(&["x", "y"], &[Int32, Int32], None);
//         let guard = vec![RecordBatch::try_new(guard_schema.clone(), columns.clone())?];
//         let guard = Arc::new(MemoryExec::try_new(&[guard], guard_schema, None)?);

//         let quaternary_grouped = Arc::new(GroupBy::new(quaternary, vec![0]));

//         let u_schema = create_schema(&["b", "f"], &[Int32, Int32], None);
//         let u = vec![RecordBatch::try_new(u_schema.clone(), columns.clone())?];
//         let u = Arc::new(MemoryExec::try_new(&[u], u_schema, None)?);
//         let u = MultiSemiJoin::new(u, vec![], vec![]);
//         let u_grouped = Arc::new(GroupBy::new(u, vec![0]));

//         let ternary = MultiSemiJoin::new(
//             guard,
//             vec![quaternary_grouped, u_grouped],
//             vec![vec![(0, 0)], vec![(0, 0)]],
//         );

//         let plan = Arc::new(Unnest::new(Arc::new(ternary)));

//         let results = collect(plan, Arc::new(TaskContext::default())).await?;

//         assert!(results.len() == 1);
//         let batch = &results[0];

//         println!(
//             "{}",
//             pretty_format_batches(&[batch.clone()]).unwrap().to_string()
//         );

//         // 6 relations, 2 rows each, all rows join with all rows
//         assert!(batch.num_columns() == 6 * 2);
//         assert!(batch.num_rows() == 2 * 2 * 2 * 2 * 2 * 02);
//         assert!(batch
//             .columns()
//             .iter()
//             .all(|col| col.as_ref() == &Int32Array::from(vec![1; 2 * 2 * 2 * 2 * 2 * 2])));

//         Ok(())
//     }

//     #[tokio::test]
//     async fn unnest_deep_nesting() -> Result<(), DataFusionError> {
//         use DataType::*;

//         let r_schema = create_schema(&["r1", "r2"], &[Int32, Int32, Int32, Int32], None);
//         let s_schema = create_schema(&["s1", "s2"], &[Int32, Int32, Int32, Int32], None);
//         let t_schema = create_schema(&["t1", "t2"], &[Int32, Int32, Int32, Int32], None);
//         let u_schema = create_schema(&["u1", "u2"], &[Int32, Int32, Int32, Int32], None);
//         let v_schema = create_schema(&["v1", "v2"], &[Int32, Int32, Int32, Int32], None);
//         let w_schema = create_schema(&["w1", "w2"], &[Int32, Int32, Int32, Int32], None);
//         let x_schema = create_schema(&["x1", "x2"], &[Int32, Int32, Int32, Int32], None);
//         let y_schema = create_schema(&["y1", "y2"], &[Int32, Int32, Int32, Int32], None);
//         let z_schema = create_schema(&["z1", "z2"], &[Int32, Int32, Int32, Int32], None);

//         let columns: Vec<ArrayRef> = vec![
//             Arc::new(Int32Array::from(vec![1, 1])),
//             Arc::new(Int32Array::from(vec![1, 1])),
//         ];

//         let r = vec![RecordBatch::try_new(r_schema.clone(), columns.clone())?];
//         let s = vec![RecordBatch::try_new(s_schema.clone(), columns.clone())?];
//         let t = vec![RecordBatch::try_new(t_schema.clone(), columns.clone())?];
//         let u = vec![RecordBatch::try_new(u_schema.clone(), columns.clone())?];
//         let v = vec![RecordBatch::try_new(v_schema.clone(), columns.clone())?];
//         let w = vec![RecordBatch::try_new(w_schema.clone(), columns.clone())?];
//         let x = vec![RecordBatch::try_new(x_schema.clone(), columns.clone())?];
//         let y = vec![RecordBatch::try_new(y_schema.clone(), columns.clone())?];
//         let z = vec![RecordBatch::try_new(z_schema.clone(), columns.clone())?];

//         let r = Arc::new(MemoryExec::try_new(&[r], r_schema, None)?);
//         let s = Arc::new(MemoryExec::try_new(&[s], s_schema, None)?);
//         let t = Arc::new(MemoryExec::try_new(&[t], t_schema, None)?);
//         let u = Arc::new(MemoryExec::try_new(&[u], u_schema, None)?);
//         let v = Arc::new(MemoryExec::try_new(&[v], v_schema, None)?);
//         let w = Arc::new(MemoryExec::try_new(&[w], w_schema, None)?);
//         let x = Arc::new(MemoryExec::try_new(&[x], x_schema, None)?);
//         let y = Arc::new(MemoryExec::try_new(&[y], y_schema, None)?);
//         let z = Arc::new(MemoryExec::try_new(&[z], z_schema, None)?);

//         // // W ⋉ (Y ⋉ Z)  (guard=W, children=[Y,Z])
//         let y = MultiSemiJoin::new(y, vec![], vec![]);
//         let y_grouped = Arc::new(GroupBy::new(y, vec![0]));
//         let z = MultiSemiJoin::new(z, vec![], vec![]);
//         let z_grouped = Arc::new(GroupBy::new(z, vec![0]));
//         let w = MultiSemiJoin::new(
//             w,
//             vec![y_grouped, z_grouped],
//             vec![vec![(0, 0)], vec![(0, 0)]],
//         );

//         // T ⋉ (V ⋉ W ⋉ X)   (guard=T, children=[V,W,X])
//         let v = MultiSemiJoin::new(v, vec![], vec![]);
//         let v_grouped = Arc::new(GroupBy::new(v, vec![0]));
//         let w_grouped = Arc::new(GroupBy::new(w, vec![0]));
//         let x = MultiSemiJoin::new(x, vec![], vec![]);
//         let x_grouped = Arc::new(GroupBy::new(x, vec![0]));
//         let t = MultiSemiJoin::new(
//             t,
//             vec![v_grouped, w_grouped, x_grouped],
//             vec![vec![(0, 0)], vec![(0, 0)], vec![(0, 0)]],
//         );

//         // R ⋉ (S ⋉ T ⋉ U)   (guard=R, children=[S,T,U])
//         let s = MultiSemiJoin::new(s, vec![], vec![]);
//         let s_grouped = Arc::new(GroupBy::new(s, vec![0]));
//         let t_grouped = Arc::new(GroupBy::new(t, vec![0]));
//         let u = MultiSemiJoin::new(u, vec![], vec![]);
//         let u_grouped = Arc::new(GroupBy::new(u, vec![0]));

//         let root = MultiSemiJoin::new(
//             r,
//             vec![s_grouped, t_grouped, u_grouped],
//             vec![vec![(0, 0)]; 3],
//         );
//         let flatten = Arc::new(Unnest::new(Arc::new(root)));

//         let results = collect(flatten.clone(), Arc::new(TaskContext::default())).await?;
//         assert!(results.len() == 1);
//         let batch = &results[0];

//         assert!(batch.num_columns() == 18); // 2*9 (9 tables with 2 columns each)
//         assert!(
//             batch.num_rows() == 512,
//             "{}",
//             format!("Expected 512 rows, got {}", batch.num_rows())
//         ); // 2^9  (9 tables with two rows each & ajoins are cartesian products)

//         assert!(batch
//             .columns()
//             .iter()
//             .all(|col| col.as_ref() == &Int32Array::from(vec![1; 512])));

//         Ok(())
//     }
// }
