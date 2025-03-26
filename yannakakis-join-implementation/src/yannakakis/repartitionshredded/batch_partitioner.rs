use std::sync::Arc;

use datafusion::{arrow::array::{ArrayRef, RecordBatch, UInt32Array}, error::DataFusionError};

use crate::yannakakis::data::{Idx, NestedBatch, NestedColumn, NestedRel, NonSingularNestedColumn, SemiJoinResultBatch, SingularNestedColumn};
use datafusion::arrow::compute::take;

use datafusion::common::hash_utils::{self, create_hashes};
// partitioner for batches based on a partitioning
pub struct MsjBatchPartitioner {
    state: MsjBatchPartitionerState,
    id: usize,
    timer: datafusion::physical_plan::metrics::Time,
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
        msj_id: usize,
        timer: datafusion::physical_plan::metrics::Time,
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

        Ok(Self {
            state,
            id: msj_id,
            timer,
        })
    }

    pub fn partition_iter(
        &mut self,
        batch: SemiJoinResultBatch,
        partition: usize,
        msj_id: usize,
        nested_combined: &Option<Arc<NestedRel>>,
        nested_offsets: &Vec<usize>,
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
                        let timer = self.timer.timer();
                        let column = val.column(*partition_key);
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

                        //done with hashing
                        timer.done();

                        //vector containing the rebuilt batches
                        let mut batches: Vec<SemiJoinResultBatch> = Vec::new();
                        let mut batchindices: Vec<usize> = Vec::new();
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
                            // println!(
                            //     "-----\n[MSJREP PRINT]\nmsj id {}, original batch: {:?}\n batch {}: {:?}\n-----",
                            //     msj_id, val, i, new_batch
                            // );
                            //if the batch's reguar columns are not empty, add it to output otherwise we skip it
                            if new_batch.num_rows() > 0 {
                                batches.push(new_batch);
                                batchindices.push(i);
                            } else {
                                // println!("empty batch");
                            }
                        }
                        return Ok(Box::new(
                            batches
                                .into_iter()
                                .zip(batchindices.into_iter())
                                .map(|(batch, idx)| Ok((idx, batch))),
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
                        let mut batchindices: Vec<usize> = Vec::new();


                        //unwrapping nested_combined since it should always be some in the nestedbatch case
                        let nested_combined = nested_combined.as_ref().unwrap();

                        //rebuild a batch for each partition
                        for i in 0..num_partitions {
                            let arr: Vec<u32> = indices[i].iter().map(|x| *x as u32).collect();
                            // let arr: Sel = Sel::new(arr);

                            let schema = val.schema().clone();

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

                            if regular_cols.iter().any(|col| col.len() == 0) {
                                //if any of the regular columns is empty, skip this partition
                                continue;
                            }
                            let mut inner_cols_final: Vec<NestedColumn> = Vec::new();
                            for col in val.inner.nested_cols.iter() {
                                // println!("nested_offsets: {:?}", nested_offsets);
                                let c = take_rows_from_nestedcol(col, arr.as_ref(), nested_combined.clone(), nested_offsets[partition])?;
                                // println!("-------\n[MSJREP PRINT]\n-----\nmsj {} nested_combined_clone: {:?}\nnested_offsets: {:?}\n current nested column: {:?}\n-------\n", msj_id,nested_combined.clone(), nested_offsets ,c);
                                inner_cols_final.push(c);
                            }

                            let new_batch = SemiJoinResultBatch::Nested(NestedBatch::new(
                                schema,
                                regular_cols,
                                inner_cols_final,
                            ));
                            println!("-----\n[MSJREP PRINT]\n-----\nmsj {} original batch:\n {:?} \n nested_data: \n {:?}\n+++++\n new batch for partition {}:\n {:?}\n-----\n-----",msj_id, val, nested_combined,i, new_batch);
                            if new_batch.num_rows() > 0 {
                                batches.push(new_batch);
                                batchindices.push(i);
                            } else {
                                // println!("empty batch");
                            }
                        }

                        return Ok(Box::new(
                            batches
                                .into_iter()
                                .zip(batchindices.into_iter())
                                .map(|(batch, idx)| Ok((idx, batch))),
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


/// Create new [NestedColumn] by taking the rows in `nestedcol` at the positions in `row_ids`.
/// # Panics
/// Panics if an index in `row_ids` is out of bounds for `nestedcol`.
#[inline(always)]
pub fn take_rows_from_nestedcol(
    nestedcol: &NestedColumn,
    row_ids: &[Idx],
    nested_data: Arc<NestedRel>,
    nested_offset: usize,
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
            // let new_weights = take_unnest(&ns_nestedcol.weights, row_ids)?;
            let new_weights = ns_nestedcol.weights.clone();
            let mut new_hols = take_unnest(&ns_nestedcol.hols, row_ids)?;
            //add offset to new_hols //! this only changes the outside hols, not the nested ones
            for i in 0..new_hols.len() {
                new_hols[i] += nested_offset as u32;
            }
            // let new_data = ns_nestedcol.clone_data(); // ! changed this to make a deep copy of the data
            // println!("data in take_rows_from_nestedcol: {:?}", ns_nestedcol.data);
            let new_data = nested_data.clone();

            let nestedcol = NonSingularNestedColumn {
                weights: new_weights,
                hols: new_hols,
                // data: ns_nestedcol.data.clone(), // clone arc = cheap,
                data: new_data, // clone data = expensive
            };
            Ok(NestedColumn::NonSingular(nestedcol))
        }
    }
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