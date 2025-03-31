use core::panic;
use std::{sync::Arc, u32};

use datafusion::{arrow, error::DataFusionError};

use crate::yannakakis::data::{NestedColumn, NestedRel, NestedSchema, SingularNestedColumn};

pub struct NestedCombiner {
    inner_cols: Vec<NestedColumn>,

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
        let inner_cols = vec![empty_nestedcol; num_input_partitions];
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
        if index != 0{
            self.offsets[index] = inner_col.num_rows();
        }
        self.inner_cols[index] = inner_col;
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
        //combine all inner columns
        let mut final_inner_col = self.inner_cols[0].clone();
        // println!("=======\ninitial final inner col: {:?}\n========", final_inner_col);
        let mut curr_offset;
        match final_inner_col {
            NestedColumn::Singular(ref s) => {
                panic!("Singular column in non-singular column");
                curr_offset = s.weights.len() as u32;
            }
            NestedColumn::NonSingular(ref ns) => {
                curr_offset = ns.data.next.as_ref().unwrap().iter().len() as u32;
                // println!("\nns data next \n{:?}\n", );
            }
        }
        if self.offsets.len() > 1{
            self.offsets[1] = curr_offset as usize;
        }
        // self.offsets[1] = curr_offset as usize;
        //append each inner column to the final inner column
        for i in 1..self.inner_cols.len() {
            let inner_col = &self.inner_cols[i];
            match final_inner_col {
                NestedColumn::Singular(ref mut s) => {
                    match inner_col {
                        NestedColumn::Singular(ref inner_s) => {
                            panic!("Singular column in non-singular column");
                            s.weights.extend(inner_s.weights.iter());
                        }
                        NestedColumn::NonSingular(ref inner_ns) => {
                            let mut new_weights = s.weights.clone();
                            new_weights.extend(inner_ns.weights.iter());
                            s.weights = new_weights;
                        }
                    }
                }
                NestedColumn::NonSingular(ref mut final_nested) => {
                    match inner_col {
                        NestedColumn::Singular(ref inner_s) => {
                            panic!("Singular column in non-singular column");
                            final_nested.weights.extend(inner_s.weights.iter());
                        }
                        NestedColumn::NonSingular(ref inner_nested) => {
                            
                            if !inner_nested.is_highest_level(){
                                // println!("append other recursive");
                                // println!("final nested before append\n: {:?} \n\n other: \n {:?} \n\n", final_nested, inner_nested);
                                final_nested.append_other_recursive(inner_nested, curr_offset as usize);
                                // println!("final nested after append: {:?}", final_nested);
                            }
                            let next_offset = final_nested.append_other_top_level(inner_nested, curr_offset as usize);
                            // println!("final nested after append: {:?}", final_nested);
                            // println!("BEFORE OFFSETS: {:?}", self.offsets);
                            if i+1 != self.inner_cols.len(){ 
                                println!("IN IF STATEMENT TEST");
                                self.offsets[i+1] = next_offset;
                                curr_offset = self.offsets[i+1] as u32;
                            }
                        }
                    }
                }
            }
        }
        self.final_inner_col = final_inner_col.clone();
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

    pub fn get_inners(&self) -> &Vec<NestedColumn> {
        &self.inner_cols
    }

    pub async fn wait_for_ready(&self) {
        while !self.ready {
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }
    }

    pub fn print_content(&self) {
        if !self.ready{
            println!("NestedCombiner not ready yet");
        }
        else{
            println!(
                "*******\n[PRINT CONTENT]\ninner cols:\n {:?}, \n\nready:\n {:?}, \n\nfinal inner col:\n {:?}, \n\npresent partitions:\n {:?}, \n\noffsets: {:?}\n*******\n",
                self.inner_cols, self.ready, self.final_inner_col, self.present_partitions, self.offsets
            );
        }
    }

    pub fn is_ready(&self) -> bool {
        self.ready
    }
}