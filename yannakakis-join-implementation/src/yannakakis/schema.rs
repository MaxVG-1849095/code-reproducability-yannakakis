//! Schema of the [Flatten] node.
//!
use std::sync::Arc;

use datafusion::arrow::{
    array::{ArrayRef, RecordBatch},
    datatypes::{FieldRef, Schema, SchemaRef},
};

use crate::yannakakis::{multisemijoin::MultiSemiJoin, repartitionshredded::GroupByWrapper};
use super::repartitionshredded::MultiSemiJoinWrapper;
use super::repartitionshredded::GroupByWrapperEnum;

#[derive(Debug, Clone)]
pub(super) enum FieldType {
    /// Field that is *not* generated by flattening.
    /// This will always be a duplicate of an [FieldType::ENUMERATED] field (because they are equi-joined).
    /// Holds a reference to that field.
    DUPLICATE(usize),
    /// Field that is generated by flattening.
    ENUMERATED(FieldRef),
}

/// Output schema of the [Flatten] node.
#[derive(Debug, Clone)]
pub struct YannakakisSchema {
    /// Output schema of the [Flatten] node.
    output_schema: SchemaRef,

    /// Output schema of the [Flatten] node in encoded form.
    ///
    /// The result of a join contains all duplicates of the join columns.
    /// However, Yannakakis (the Flatten operator) will generate only one copy of each join column.
    /// Therefore, the join columns must be duplicated afterwards to obtain the correct output schema.
    ///
    /// `schema_inner` encodes duplicate fields (i.e., join columns) as follows:
    /// - For each field that is generated by the Flatten operator, there is a corresponding [FieldType::ENUMERATED] field.
    /// - For each field that is *not* generated the Flatten operator, there is a corresponding [FieldType::DUPLICATE] field
    ///   that holds the index of the corresponding [FieldType::ENUMERATED] field in this vector.
    schema_inner: Vec<FieldType>,

    /// No NestedSchema has more than one nested field,
    /// which means that all semijoins are unary / binary.
    multisemijoin_flag: bool,
}

impl YannakakisSchema {
    /// No NestedSchema has more than one nested field,
    /// which means that all semijoins are unary / binary.
    pub fn all_multisemijoins_unary_or_binary(&self) -> bool {
        self.multisemijoin_flag
    }

    /// Number of fields that are generated by flattening.
    /// Duplicates of join columns are excluded!
    pub fn n_unnest_fields(&self) -> usize {
        self.schema_inner
            .iter()
            .filter(|f| matches!(f, FieldType::ENUMERATED(_)))
            .count()
    }

    /// Output schema of the [Flatten] node.
    /// Includes all duplicates of join columns.
    pub fn output_schema(&self) -> SchemaRef {
        self.output_schema.clone()
    }

    pub fn build_batch(
        &self,
        unnested_columns: &mut Vec<ArrayRef>, // columns generated by unnesting (or flattening)
    ) -> Result<RecordBatch, datafusion::arrow::error::ArrowError> {
        let schema = self.output_schema();

        let mut output: Vec<ArrayRef> = Vec::with_capacity(schema.fields().len());

        let mut unnested_iter = unnested_columns.drain(..);

        for field in &self.schema_inner {
            match field {
                FieldType::DUPLICATE(idx) => {
                    // output array was already computed, duplicate it!
                    output.push(output[*idx].clone());
                }
                FieldType::ENUMERATED(_) => {
                    output.push(
                        unnested_iter
                            .next()
                            .expect("Not enough arrays generated by unnesting"),
                    );
                }
            }
        }

        RecordBatch::try_new(schema, output)
    }

    pub fn new(root: &dyn MultiSemiJoinWrapper) -> Self {
        /// Fills `encoded_schema` and `full_schema`.
        /// Returns whether a MultiSemiJoin with >= 2 children (excl. guard) was encountered.
        fn helper(
            node: &dyn MultiSemiJoinWrapper,
            offset: usize,
            encoded_schema: &mut Vec<FieldType>,
            full_schema: &mut Vec<FieldRef>,
        ) -> bool {
            fn n_attrs_in_subtree(node: &dyn MultiSemiJoinWrapper) -> usize {
                node.schema().regular_fields.fields().len()
                    + node
                        .children()
                        .iter()
                        .map(|child| n_attrs_in_subtree(child.child().as_ref()))
                        .sum::<usize>()
            }

            let mut multisemijoin = node.children().len() >= 2;

            node.children()
                .iter()
                .zip(node.semijoin_keys())
                .fold(0, |acc, (child, guard_keys)| {
                    let mut keys = guard_keys.iter();
                    let group_on = child.group_on();
                    let groupby_input_fields = child.child().schema().regular_fields.fields();

                    for (i, field) in groupby_input_fields.iter().enumerate() {
                        if group_on.contains(&i) {
                            // This is a groupby field, and is already in encoded_schema.
                            // We need to duplicate it.
                            let idx = keys
                                .next()
                                .expect("Each groupby field should be in semijoin_keys");
                            encoded_schema.push(FieldType::DUPLICATE(offset + *idx));
                        } else {
                            // This is not a groupby field, so we need to enumerate it.
                            encoded_schema.push(FieldType::ENUMERATED(field.clone()));
                        }
                        full_schema.push(field.clone());
                    }

                    let multisemijoin_in_child = helper(
                        child.child().as_ref(),
                        offset + acc + node.schema().regular_fields.fields().len(),
                        encoded_schema,
                        full_schema,
                    );

                    multisemijoin |= multisemijoin_in_child;

                    acc + n_attrs_in_subtree(child.child().as_ref())
                });
            multisemijoin
        }

        let mut full_schema: Vec<FieldRef> = Vec::new();
        let mut encoded_schema: Vec<FieldType> = Vec::new();

        for regular_field in root.schema().regular_fields.fields() {
            encoded_schema.push(FieldType::ENUMERATED(regular_field.clone()));
            full_schema.push(regular_field.clone());
        }

        let multisemijoin_flag = !helper(root, 0, &mut encoded_schema, &mut full_schema);

        Self {
            output_schema: Arc::new(Schema::new(full_schema)),
            schema_inner: encoded_schema,
            multisemijoin_flag,
        }
    }
}
