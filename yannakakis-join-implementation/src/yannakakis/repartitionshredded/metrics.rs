// ! metrics for tracking the performance of the repartition operator

use datafusion::physical_plan::metrics::{self, ExecutionPlanMetricsSet, MetricBuilder};

#[derive(Clone, Debug)]
pub(super) struct RepartitionMetrics{
    //total time spent by this operator
    pub total_time: metrics::Time,
    

}