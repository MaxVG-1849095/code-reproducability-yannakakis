#!/bin/bash

# Execute queries in DuckDB, save profiling output as json
# Usage: execute_queries_duckdb.sh <queries> <db> <outfolder> <repetitions>
# e.g., execute_queries_duckdb.sh queries/ imdb.db plans/ 5

queries_folder=$1
database=$2
plans_folder=$3
repetitions=$4

duckdb_path="../duckdb/build/release/duckdb"

# Exit if <queries_folder> does not exist
if [ ! -d "$queries_folder" ]; then
    echo "Directory <queries_folder> does not exist."
    exit 1
fi

# Exit if <database> does not exist
if [ ! -f "$database" ]; then
    echo "Database $database does not exist."
    exit 1
fi

# Exit if <plans_folder> already exists
if [ -d "$plans_folder" ]; then
    echo "Directory $plans_folder already exists. Please remove it or choose another name."
    exit 1
fi

mkdir -p "$plans_folder"

tmp="tmp.sql"

echo "SET threads TO 1;" >> $tmp
echo "PRAGMA enable_profiling=json;" >> $tmp
# SET explain_output = 'all'; >> $tmp
# SET profiling_mode = 'detailed'; >> $tmp


for query in "$queries_folder"*.sql
do
    basename=$(basename -- "$query")
    qname="${basename%.*}"
    mkdir "$plans_folder/$qname"

    for i in $(seq 1 "$repetitions"); do
        echo "PRAGMA profile_output = '${plans_folder}/${qname}/run_${i}.json';" >> $tmp
        cat "$query" >> $tmp
    done
done 

"$duckdb_path" "$database" << EOF
.read '$tmp'
EOF


rm $tmp

echo "Done executing queries in DuckDB."

