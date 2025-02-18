#!/bin/sh

# Run clean.sh first to clean up previous results.
# Then run this script to generate new results.

root=$(pwd)


# # Run all benchmark queries in DuckDB
repetitions=1  # Number of repetitions for each query
cd "$root"/duckdb_scripts || exit
./execute_queries_duckdb.sh "$root"/benchmarks/ce/preprocessed_queries/ ./ce.db "$root"/query_plans/ce_duckdb/2_original_with_aliases $repetitions


# Remove intermediate projections and filters from DuckDB query plans
cd "$root"/duckdb_scripts || exit
./remove_intermediate_filters_and_projections.sh "$root"/query_plans/ce_duckdb/2_original_with_aliases "$root"/query_plans/ce_duckdb/3_no_filters_and_projections

# Generate .html for files in 2_* folders. It is used during analysis.
cd "$root"/duckdb_scripts || exit
./visualize_duckdb_plans.sh "$root"/query_plans/ce_duckdb/2_original_with_aliases

# Convert all DuckDB query plans to an intermediate representation (IR)
cd "$root"/query_plans/ce_duckdb || exit
./create_IR_plans.sh

# Categorize binary plans into well-behaved and non well-behaved
cd "$root"/2phase_nsa || exit
mkdir ./categories
python categorize_IR_plans.py -i "$root"/query_plans/ce_duckdb/3_IR_no_filters_and_projections/ -o categories/ce_categories.csv

# Convert binary plans to 2NSA plans
for benchmark in ce_duckdb
do
    yann_folder="$root"/query_plans/"$benchmark"/yannakakis
    binary_folder="$root"/query_plans/"$benchmark"/3_IR_no_filters_and_projections/
    cd "$root"/2phase_nsa || exit
    python generate_semijoin_plans.py -i "$binary_folder" -o "$yann_folder"
    cd "$root"/query_plans || exit
    "$root"/query_plans/yannakakis_from_template.sh "$yann_folder" "$binary_folder"
done