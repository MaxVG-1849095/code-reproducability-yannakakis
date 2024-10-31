# Convert all plans in a folder (generated by DuckDB or Postgres) to Plan objects: a Plan object is a DBMS-independent representation of a query plan.
# Usage: python to_intermediate_plan.py [-h] [-q QUERIES_FOLDER] [-o OUTPUT_FOLDER] [--dbms {duckdb,psql}]

from intermediate_plan.plan import Plan
from intermediate_plan.parse_duckdb_plan import parse_duckdb_plan
from intermediate_plan.parse_duckdb_plan_with_selfjoins import parse_duckdb_plan_with_selfjoins
from intermediate_plan.parse_psql_plan import parse_psql_plan
import os
import argparse


def parse_plans_in_folder(queries_folder: str, convert_fn, destination_folder: str):
    if not os.path.exists(destination_folder):
        os.makedirs(destination_folder)

    for root, dirs, files in os.walk(queries_folder):
        relative_path = os.path.relpath(root, queries_folder)
        dest_dir = os.path.join(destination_folder, relative_path)

        if not os.path.exists(dest_dir):
            os.makedirs(dest_dir)

        for file in files:
            try:
                file_path = os.path.join(root, file)
                if file.endswith("run_1.json"):
                    plan: Plan = convert_fn(file_path)
                    plan.try_make_valid()
                    new_file_path = os.path.join(dest_dir, os.path.splitext(file)[0] + ".json")
                    with open(new_file_path, "w") as json_file:
                        json_file.write(plan.to_json())
            except Exception as e:
                print("Error parsing plan: ", os.path.join(root, file), e)


if __name__ == "__main__":
    import json

    parser = argparse.ArgumentParser(
        description="Convert all plans in a folder (generated by DuckDB or Postgres) to Plan objects: a Plan object is a DBMS-independent representation of a query plan."
    )
    parser.add_argument(
        "-q",
        "--queries_folder",
        type=str,
        required=True,
        help="The folder where the queries and plans are stored",
    )
    parser.add_argument(
        "-o",
        "--output_folder",
        required=True,
        type=str,
        help="The folder where the converted plans will be stored",
    )
    parser.add_argument(
        "--dbms",
        choices=["duckdb", "psql"],
        help="The DBMS system used to generate the plans.",
    )
    parser.add_argument(
        "-a",
        "--aliases",
        required=False,
        type=str,
        help="Optional .json file with alias -> table_name mapping. Should be provided when DuckDB query has self-joins! Ignored when --dmbs=psql",
    )
    args = parser.parse_args()

    if args.dbms == "duckdb":
        if args.aliases is not None:
            with open(args.aliases, "r") as f:
                aliases = json.load(f)
                convert_fn = lambda x: parse_duckdb_plan_with_selfjoins(x, aliases)
        else:
            convert_fn = parse_duckdb_plan
    elif args.dbms == "psql":
        convert_fn = parse_psql_plan
    else:
        print("Invalid dbms system: ", args.dbms, ". Options: 'duckdb', 'psql'")
        exit(1)

    parse_plans_in_folder(args.queries_folder, convert_fn, args.output_folder)
