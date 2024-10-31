# Rewrite non-well-behaved plans into well-behaved plans,
# if there are multiple rewritings possible, return the one with the lowest cost.

from .binary_plan import BinaryJoinNode, LeafNode, attrs, is_well_behaved
from .binary_plan import ll, la
from typing import Generator

Plan = BinaryJoinNode | LeafNode

# ----------------------------------------------------------------------------------

# Set of well-behaved plans.
# In this set, there are no two plans with the same left-most leaf attribute.

# EQUIVALENT TO:
# Set of join trees.
# In this set, there are no two join trees with the same root.


class WellBehavedPlans:
    def __init__(self):
        """Initialize an empty set of well-behaved plans."""
        # Since there are no two plans with the same left-most leaf attribute,
        # we can use a dictionary to store for each unique left-most leaf attribute the cheapest plan and its cost.
        # key = name of the left-most leaf attribute
        # value = (plan, cost)
        self.plans: dict[str, tuple[Plan, int]] = {}

    @staticmethod
    def new_from_leaf(plan: LeafNode):
        plans = WellBehavedPlans()
        plans.add(plan, 0)  # leaf node has cost 0
        return plans

    def add(self, plan: Plan, cost: int):
        """
        Add well-behaved plan with `cost`.
        If a plan with the same left-most leaf attribute already exists, keep the one with the lowest cost.
        """
        key = ll(plan).relation_name

        if key in self.plans:
            _, existing_cost = self.plans[key]
            if cost < existing_cost:
                self.plans[key] = (plan, cost)
        else:
            self.plans[key] = (plan, cost)

    def iter(self) -> Generator[tuple[Plan, int], None, None]:
        """Returns all well-behaved plans with their cost."""
        for plan, cost in self.plans.values():
            yield plan, cost

    def len(self):
        return len(self.plans)


# ----------------------------------------------------------------------------------


def make_well_behaved(plan: Plan) -> Plan:
    """Rewrite a non-well-behaved plan into a well-behaved plan with minimal build penalty."""
    candidates = get_well_behaved_plans(plan)
    best_plan, _ = min(candidates.iter(), key=lambda x: x[1])
    assert is_well_behaved(best_plan), "Error: rewritten plan is not well-behaved."
    return best_plan


def get_well_behaved_plans(plan: Plan) -> WellBehavedPlans:
    """Generate a set of well-behaved plans for the given plan."""
    if isinstance(plan, LeafNode):
        return WellBehavedPlans.new_from_leaf(plan)

    left = get_well_behaved_plans(plan.left_child)
    right = get_well_behaved_plans(plan.right_child)
    return merge(left, right, plan.join_attributes)


def merge(left: WellBehavedPlans, right: WellBehavedPlans, join_on: set[int]) -> WellBehavedPlans:
    merged_plans = WellBehavedPlans()

    for left_plan, left_cost in left.iter():
        for right_plan, right_cost in right.iter():
            # try to merge left into right
            merged = merge_left_into_right(left_plan, right_plan, join_on)
            if merged is not None:
                cost = left_cost + right_cost + ll(left_plan).ec
                merged_plans.add(merged, cost)

            # try to merge right into left
            merged = merge_right_into_left(left_plan, right_plan, join_on)
            if merged is not None:
                cost = left_cost + right_cost
                merged_plans.add(merged, cost)

    return merged_plans


def merge_left_into_right(left: Plan, right: Plan, join_on: set[int]) -> Plan | None:
    """
    Given two well-behaved plans left & right that join on attributes in `join_on`.
    Try to merge `left` into `right` such that the resulting plan is also well-behaved.
    Returns None if this is not possible.

    Merging happens as follows:
    - Search for the topmost node in `right` that contains all join attributes in `join_on`, and
    - subsequently attach `left` to that node as the right-most child.
    """
    return try_attach(left, right, join_on)


def merge_right_into_left(left: Plan, right: Plan, join_on: set[int]) -> Plan | None:
    """
    Given two well-behaved plans left & right that join on attributes in `join_on`.
    Try to merge `right` into `left` such that the resulting plan is also well-behaved.
    Returns None if this is not possible.


    Merging happens as follows:
    - Search for the topmost node in `left` that contains all join attributes in `join_on`, and
    - subsequently attach `right` to that node as the right-most child.
    """
    return try_attach(right, left, join_on)


def try_attach(plan: Plan, to: Plan, join_on: set[int]) -> Plan | None:
    """
    Given two well-behaved plans `plan` and `to` that join on attributes in `join_on`.
    Find the top-most node `n` in `to` whose left-most leaf node contains all join attributes in `join_on`.
    Join `plan` with `n`. (`n` = probe side, `plan` = build side)
    Return the new plan if successful,
    None otherwise.
    """

    if not join_on.issubset(la(plan)):
        # Attaching `plan` to any node in `to` will always result in a non-well-behaved plan.
        return None

    # Check if we can attach `plan` to the root of `to`.
    if join_on.issubset(la(to)):
        # attach `plan` to the root of `to`
        return BinaryJoinNode(left_child=to, right_child=plan, join_attributes=join_on)

    if isinstance(to, LeafNode):
        return None

    # Determine whether to keep searching in the left or right child of `to`.

    if join_on.issubset(attrs(to.left_child)):
        # continue searching in the left child of `to`
        subresult = try_attach(plan, to.left_child, join_on)
        if subresult is None:
            return None
        return BinaryJoinNode(
            left_child=subresult, right_child=to.right_child, join_attributes=to.join_attributes
        )

    elif join_on.issubset(attrs(to.right_child)):
        # continue searching in the right child of `to`
        subresult = try_attach(plan, to.right_child, join_on)
        if subresult is None:
            return None
        return BinaryJoinNode(
            left_child=to.left_child, right_child=subresult, join_attributes=to.join_attributes
        )

    return None
