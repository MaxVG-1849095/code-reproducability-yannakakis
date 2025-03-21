// ! metrics for tracking the performance of the repartition operator

use datafusion::physical_plan::metrics::{self, ExecutionPlanMetricsSet, MetricBuilder};

#[derive(Clone, Debug)]
pub(super) struct MsjRepartitionMetrics{
    //total time spent by this operator
    pub total_time: metrics::Time,
    
    pub fetch_time: metrics::Time,

    pub repartition_time: metrics::Time,

    pub send_time: Vec<metrics::Time>,

}

impl MsjRepartitionMetrics {
    pub fn new(
        input_partition: usize,
        num_output_partitions: usize,
        metrics: &ExecutionPlanMetricsSet,
    ) -> Self {
        let total_time = MetricBuilder::new(metrics).elapsed_compute(input_partition);
        let fetch_time = MetricBuilder::new(metrics).subset_time("fetch", input_partition);
        let repartition_time = MetricBuilder::new(metrics).subset_time("repartition", input_partition);
        // Time in nanos for sending resulting batches to channels
        let send_time = (0..num_output_partitions)
            .map(|output_partition| {
                let label =
                    metrics::Label::new("outputPartition", output_partition.to_string());
                MetricBuilder::new(metrics)
                    .with_label(label)
                    .subset_time("send_time", input_partition)
            })
            .collect();


        Self {
            total_time,
            fetch_time,
            repartition_time,
            send_time,
        }
    }
}