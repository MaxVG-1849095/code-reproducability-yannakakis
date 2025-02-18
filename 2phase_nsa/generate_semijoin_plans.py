# Rewrite all binary plans in <INFOLDER> into semijoin plans and write them to <OUTFOLDER>.
#
# Usage: python generate_semijoin_plans.py -i <INFOLDER> -o <OUTFOLDER>

from binary_plan.util import ir_file_to_binary_plan

# from binary_plan.rewrite import make_well_behaved
from binary_plan.rewrite_costbased import make_well_behaved
from binary_plan.to_semijoin_plan import MultiSemiJoin
from binary_plan.binary_plan import BinaryJoinNode, LeafNode, is_well_behaved


def count_relations_in_binaryplan(plan: BinaryJoinNode | LeafNode):
    if isinstance(plan, LeafNode):
        return 1
    return count_relations_in_binaryplan(plan.left_child) + count_relations_in_binaryplan(
        plan.right_child
    )


def count_relations_in_semijoinplan(plan: MultiSemiJoin):
    rels_in_children = sum([count_relations_in_semijoinplan(child) for child in plan.children])
    return rels_in_children + 1  # +1 for the guard


def rewrite_plan(infile: str, outfile: str | None):
    if os.path.exists(infile):
        try:
            binary_plan = ir_file_to_binary_plan(infile)
            n_rels = count_relations_in_binaryplan(binary_plan)
            if is_well_behaved(binary_plan):
                well_behaved_plan = binary_plan
            else:
                well_behaved_plan = make_well_behaved(binary_plan)

            assert n_rels == count_relations_in_binaryplan(
                well_behaved_plan
            ), "Number of relations changed during rewriting to well-behaved plan."

            semijoin_plan = MultiSemiJoin.from_wellbehaved_plan(well_behaved_plan)

            assert n_rels == count_relations_in_semijoinplan(
                semijoin_plan
            ), "Number of relations changed during rewriting to semijoin plan."

            if outfile is not None:
                with open(outfile, "w") as f:
                    f.write(semijoin_plan.to_yannakakis_template())

        except Exception as e:
            print(f"{infile}: {e}")


def rewrite_plans_in_dir(indir: str, outdir: str):
    for query_dir in os.listdir(indir):
        query_dir_full = os.path.join(indir, query_dir)
        if not os.path.isdir(query_dir_full):
            continue

        infile = os.path.join(query_dir_full, "run_1.json")
        outfile = os.path.join(outdir, query_dir + ".template")

        rewrite_plan(infile, outfile)


if __name__ == "__main__":
    import argparse
    import os

    parser = argparse.ArgumentParser(description="Convert binary plans to semijoin plans.")
    parser.add_argument("-i", "--infolder", help="Input folder with binary plans", required=True)
    parser.add_argument(
        "-o",
        "--outfolder",
        help="Non-existing output folder for writing semijoin plans",
        required=True,
    )
    args = parser.parse_args()

    # Check that input folder exists
    if not os.path.exists(args.infolder):
        print(f"Input folder {args.infolder} does not exist.")
        exit(1)

    if os.path.isfile(args.infolder):
        # testing purpose
        rewrite_plan(args.infolder, None)
        exit(0)

    # Check that output folder does not exist
    if os.path.exists(args.outfolder):
        print(f"Output folder {args.outfolder} already exists.")
        exit(1)

    os.makedirs(args.outfolder)

    rewrite_plans_in_dir(args.infolder, args.outfolder)
