# EvoSim README Amendment — Stress Damage Selection

**Approved design correction — 2026-09-16**

The stress-damage rule in the authoritative EvoSim design is corrected as follows:

When accumulated heat stress reaches the current stress threshold, structural damage selects the **weakest eligible structural bond** rather than selecting a random bond.

The ordering remains:

1. Select the weakest eligible **non-genome** structural bond.
2. Genome bonds remain protected while eligible non-genome structural bonds remain.
3. Once all eligible non-genome structural bonds have been exhausted, genome bonds may become vulnerable.

The weakest-bond rule is intentional. Natural selection is expected to determine the evolutionary consequences of this physical damage mechanism; no additional randomization mechanism is required merely to produce variability.

The previous README wording that required a random non-genome bond is superseded by this correction.
