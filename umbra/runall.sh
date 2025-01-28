#!/usr/bin/env bash

# Generate umbra databases for TPCH, JOB, CE, LE
# scripts/dbgen.sh

# Run umbra benchmarks for TPCH, JOB, CE, LE (you can adjust umbra settings as environment variables within the script)
# You can find results under results/benchmark.csv
# You can adjust the number of repetitions per query in scripts/benchmark.sql
# scripts/benchmark_umbra_default.sh
# scripts/benchmark_umbra_chained.sh
scripts/benchmark_umbra_le.sh
# scripts/benchmark_umbra_interpreted.sh
