use core::panic;
use std::{sync::Arc, u32};

use datafusion::{arrow, error::DataFusionError};

use crate::yannakakis::data::{NestedColumn, NestedRel, NestedSchema, SingularNestedColumn};

// struct to wrap multple nested combiners together
pub struct NestedCombinerWrapper {
    nested_combiners: Vec<NestedCombiner>,

    ready: bool,

    final_inner_cols: Vec<NestedColumn>,

    total_weights: Vec<Vec<u32>>,

    final_total_weights: Vec<u32>,

    combined: bool,
}

impl NestedCombinerWrapper {
    pub fn new(num_input_partitions: usize, num_combiners: usize) -> Self {
        let mut nested_combiners = vec![];
        for _ in 0..num_combiners {
            let nested_combiner = NestedCombiner::new(num_input_partitions);
            nested_combiners.push(nested_combiner);
        }
        let final_inner_cols = vec![];
        Self {
            nested_combiners,
            ready: false,
            final_inner_cols,
            total_weights: vec![vec![]; num_input_partitions],
            final_total_weights: vec![],
            combined:false,
        }
    }
    // add inner column to a specific child nested combiner
    pub fn add_inner_col(&mut self, inner_col: NestedColumn, index: usize, child_index: usize) {
        if self.ready {
            return;
        }

        // println!("\n\n\n\n\nADDING INNER COL FOR PARTITION {} AND CHILD {}\n {:?}\n\n\n\n\n", index, child_index, inner_col);

        self.nested_combiners[child_index].add_inner_col(inner_col, index);
    }

    // add empty inner column to all child nested combiners
    pub fn add_empty_inner_col(&mut self, index: usize) {
        if self.ready {
            return;
        }
        // println!("\n\n\n\n\nADDING EMPTY INNER COL FOR PARTITION {} AND CHILD {}\n {:?}\n\n\n\n\n", index, child_index, inner_col);
        for i in 0..self.nested_combiners.len() {
            self.nested_combiners[i].add_none_inner_col(index);
        }
    }

    pub fn combine(&mut self) {
        //turn 2D weights into 1D
        let mut total_weights = vec![];
        for i in 0..self.total_weights.len() {
            for j in 0..self.total_weights[i].len() {
                total_weights.push(self.total_weights[i][j]);
            }
        }
        for i in 0..self.nested_combiners.len() {
            let mut nested_combiner = &mut self.nested_combiners[i];
            let inner_col = nested_combiner.combine();
            match inner_col {
                Ok(inner_col) => {
                    self.final_inner_cols.push(inner_col);
                }
                Err(e) => {
                    panic!("Error combining nested columns: {:?}", e);
                }
            }
        }
        self.final_total_weights = total_weights;
        self.ready = true;
    }

    pub fn get_final_inner_col_data(&self) -> Vec<Option<Arc<NestedRel>>> {
        let mut final_inner_col_data = vec![];
        for i in 0..self.final_inner_cols.len() {
            match &self.final_inner_cols[i] {
                NestedColumn::NonSingular(ns) => final_inner_col_data.push(Some(ns.data.clone())),
                _ => final_inner_col_data.push(None),
            }
        }
        final_inner_col_data
    }

    pub fn get_final_total_weights(&self) -> Vec<u32> {
        self.final_total_weights.clone()
    }

    pub fn get_offsets(&self) -> Vec<Vec<usize>> {
        let mut offsets = vec![];
        for i in 0..self.nested_combiners.len() {
            offsets.push(self.nested_combiners[i].get_offsets().clone());
        }
        offsets
    }

    pub fn check_singular_non_singular(&self) {
        for i in 0..self.nested_combiners.len() {
            self.nested_combiners[i].check_singular_non_singular();
        }
    }

    pub fn add_total_weights(&mut self, total_weights: Vec<u32>, index: usize) {
        if self.ready {
            return;
        }
        if index >= self.total_weights.len() {
            panic!("Index out of bounds");
        }
        self.total_weights[index] = total_weights;
    }

    pub fn print_content(&self) {
        if !self.ready {
            println!("NestedCombinerWrapper not ready yet");
        } else {
            for i in 0..self.nested_combiners.len(){
                self.nested_combiners[i].print_content();
            }
        }
    }

    pub fn combined(&mut self) -> bool {
        if !self.combined {
            self.combined = true;
            return true;
        }
        false
    }
}

// object to combine multiple nested columns together
pub struct NestedCombiner {
    inner_cols: Vec<Option<NestedColumn>>,

    ready: bool,

    final_inner_col: NestedColumn,

    present_partitions: Vec<usize>,

    offsets: Vec<usize>,
}

impl NestedCombiner {
    pub fn new(num_input_partitions: usize) -> Self {
        //empty nestedcolumn obj
        let empty_schema = NestedSchema::empty();
        let empty_schema = Arc::new(empty_schema);
        let empty_nestedcol = NestedColumn::make_empty(empty_schema);
        let inner_cols = vec![Some(empty_nestedcol); num_input_partitions];
        let present_partitions = vec![usize::MAX; num_input_partitions];
        let offsets = vec![0; num_input_partitions];
        Self {
            inner_cols: inner_cols,
            ready: false,
            final_inner_col: NestedColumn::Singular(SingularNestedColumn { weights: vec![] }),
            present_partitions: present_partitions,
            offsets: offsets,
        }
    }

    pub fn add_inner_col(&mut self, inner_col: NestedColumn, index: usize) {
        if (self.ready) {
            return;
        }
        if index != 0 {
            self.offsets[index] = inner_col.num_rows();
        }
        self.inner_cols[index] = Some(inner_col);
        self.present_partitions[index] = index;
    }

    pub fn add_none_inner_col(&mut self, index: usize) {
        if (self.ready) {
            return;
        }
        self.inner_cols[index] = None;
        self.present_partitions[index] = index;
    }

    pub fn combine(&mut self) -> Result<NestedColumn, DataFusionError> {
        if self.ready {
            return Ok(self.final_inner_col.clone());
        }

        //check if all partitions are present
        for i in 0..self.present_partitions.len() {
            if self.present_partitions[i] != i {
                // println!("RETURNING EARLY IN COMBINE, NOT ALL PARTITIONS PRESENT");
                //return error
                return Err(DataFusionError::Internal(
                    "Not all partitions are present in the NestedCombiner".to_string(),
                ));
            }
        }

        println!("[][][]Combining nested columns");

        let mut curr_offset= 0;
        let mut final_inner_col: Option<NestedColumn> = None;
        // self.offsets[1] = curr_offset as usize;
        //append each inner column to the final inner column
        for i in 0..self.inner_cols.len() {
            let inner_col = &self.inner_cols[i];
            match final_inner_col {
                Some(NestedColumn::Singular(ref mut s)) => match inner_col {
                    Some(NestedColumn::Singular(ref inner_s)) => {
                        s.weights.extend(inner_s.weights.iter());
                    }
                    Some(NestedColumn::NonSingular(ref inner_ns)) => {
                        let mut new_weights = s.weights.clone();
                        new_weights.extend(inner_ns.weights.iter());
                        s.weights = new_weights;
                    }
                    None => {
                        //nothing needs to change
                        continue;
                    }
                },
                Some(NestedColumn::NonSingular(ref mut final_nested)) => {
                    match inner_col {
                        Some(NestedColumn::Singular(ref inner_s)) => {
                            final_nested.weights.extend(inner_s.weights.iter());
                        }
                        Some(NestedColumn::NonSingular(ref inner_nested)) => {
                            if !inner_nested.is_highest_level() {
                                // println!("append other recursive");
                                // println!(" \n\n\n -----\nfinal nested before append\n: {:?} \n\n other: \n {:?} \n\n", final_nested, inner_nested);
                                final_nested
                                    .append_other_2nd_level(inner_nested, curr_offset as usize);
                                // println!("final nested after append: {:?}\n -----\n\n", final_nested);
                            }
                            let next_offset = final_nested
                                .append_other_top_level(inner_nested, curr_offset as usize);
                            // println!("final nested after append: {:?}", final_nested);
                            // println!("BEFORE OFFSETS: {:?}", self.offsets);
                            if i + 1 != self.inner_cols.len() {
                                self.offsets[i + 1] = next_offset;
                                curr_offset = self.offsets[i + 1] as u32;
                            }
                        }
                        None => {
                            self.offsets[i] = curr_offset as usize;
                        }
                    }
                }
                None => {
                    println!("IN NONE STATEMENT TEST");
                    //combine all inner columns
                    final_inner_col = self.inner_cols[i].clone();
                    // println!("=======\ninitial final inner col: {:?}\n========", final_inner_col);
                    
                    match final_inner_col {
                        Some(NestedColumn::Singular(ref s)) => {
                            curr_offset = s.weights.len() as u32;
                        }
                        Some(NestedColumn::NonSingular(ref ns)) => {
                            curr_offset = ns.data.next.as_ref().unwrap().iter().len() as u32;
                            // println!("\nns data next \n{:?}\n", );
                        }
                        None => {
                            curr_offset = 0;
                        }
                    }
                    if self.offsets.len() > i+1 {
                        self.offsets[i+1] = curr_offset as usize;
                    }
                }
            }
        }
        self.final_inner_col = final_inner_col.clone().expect("final_inner_col is None");
        self.ready = true;

        println!("print end of combined:");
        // self.print_content();

        Ok(self.final_inner_col.clone())
    }

    pub fn get_final_inner_col(&self) -> &NestedColumn {
        &self.final_inner_col
    }

    pub fn get_final_inner_col_data(&self) -> Option<Arc<NestedRel>> {
        match &self.final_inner_col {
            NestedColumn::NonSingular(ns) => Some(ns.data.clone()),
            _ => None,
        }
    }
    pub fn get_offsets(&self) -> &Vec<usize> {
        &self.offsets
    }

    pub fn get_inners(&self) -> &Vec<Option<NestedColumn>> {
        &self.inner_cols
    }

    pub async fn wait_for_ready(&self) {
        while !self.ready {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    pub fn print_content(&self) {
        if !self.ready {
            println!("NestedCombiner not ready yet");
        } else {
            println!(
                "*******\n[PRINT CONTENT]\ninner cols:\n {:?}, \n\nready:\n {:?}, \n\nfinal inner col:\n {:?}, \n\npresent partitions:\n {:?}, \n\noffsets: {:?}\n*******\n",
                self.inner_cols, self.ready, self.final_inner_col, self.present_partitions, self.offsets
            );
        }
    }

    pub fn check_singular_non_singular(&self) {
        let mut singular = 0;
        let mut non_singular = 0;
        for i in 0..self.inner_cols.len() {
            match &self.inner_cols[i] {
                Some(NestedColumn::Singular(_)) => singular += 1,
                Some(NestedColumn::NonSingular(_)) => non_singular += 1,
                None => {}
            }
        }
        if singular > 0 && non_singular > 0 {
            panic!("Both singular and non-singular columns present");
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}
