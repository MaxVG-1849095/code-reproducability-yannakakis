// ! metrics for tracking the performance of the repartition operator

use datafusion::physical_plan::metrics::{self, ExecutionPlanMetricsSet, MetricBuilder};



#[derive(Clone, Debug)]
pub(super) struct MsjRepartitionMetrics{
    //total time spent by this operator
    pub total_time: metrics::Time,

    pub first_batch_time : metrics::Time,
    //time spent fetching child stream
    pub fetch_time: metrics::Time,
    //total time for pull_from_input
    pub repartition_time: metrics::Time,

    pub send_time: metrics::Time,
    //time spent waiting for the barrier
    pub barrier_time_1: metrics::Time, 
    //time spent waiting for the barrier
    pub barrier_time_2: metrics::Time,
    //time spent combining the nested columns
    pub combine_timer: metrics::Time,

    pub lock_time: metrics::Time,
}

impl MsjRepartitionMetrics {
    pub fn new(
        input_partition: usize,
        num_output_partitions: usize,
        metrics: &ExecutionPlanMetricsSet,
    ) -> Self {
        let total_time = MetricBuilder::new(metrics).elapsed_compute(input_partition);
        let first_batch_time = MetricBuilder::new(metrics).subset_time("first_batch_time", input_partition);
        let fetch_time = MetricBuilder::new(metrics).subset_time("fetch_time", input_partition);
        let repartition_time = MetricBuilder::new(metrics).subset_time("repartition_time", input_partition);
        // Time in nanos for sending resulting batches to channels
        let send_time = MetricBuilder::new(metrics).subset_time("send_time", input_partition);
        let barrier_time_1 = MetricBuilder::new(metrics).subset_time("barrier_time_1", input_partition);
        let barrier_time_2 = MetricBuilder::new(metrics).subset_time("barrier_time_2", input_partition);
        let combine_timer = MetricBuilder::new(metrics).subset_time("combine_timer", input_partition);
        let lock_time = MetricBuilder::new(metrics).subset_time("lock_time", input_partition);

        Self {
            total_time,
            first_batch_time,
            fetch_time,
            repartition_time,
            send_time,
            barrier_time_1,
            barrier_time_2,
            combine_timer,
            lock_time,
        }
    }
}