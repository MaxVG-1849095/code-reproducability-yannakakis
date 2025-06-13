#!/bin/bash

root_dir="../../"
parquet_data="$root_dir/benchmarks/stats-ceb/parquet-zstd-lowercase"
out_folder="./output" # non-existing output folder
timings="timings.csv"
plan="./17/17.json"


repetitions=1

rm ./results.csv
rm -rf ./output/

# make timings file
touch $timings
#write header
echo "duration(µs),method,variant,query" > $timings

# adding the --manifest-path flag causes cargo to ignore the rust-toolchain.toml file
# so we need to specify the toolchain manually

# Run all stats-ceb queries in release mode, with 10 repetitions
## --release eventually !
cargo +nightly run --release  --manifest-path="../../intermediate_to_df_plan/Cargo.toml" --bin exec_ir_plans -- --plans "$plan" --data "$parquet_data" -o "$out_folder" -t $timings --repetitions $repetitions

