# EvoSim README Amendment — Stress Damage Selection

**Approved design correction — 2026-09-17**

The stress-damage rule is corrected back to the approved random-selection behavior.

When accumulated heat stress reaches the current stress threshold, structural damage selects a **random eligible non-genome structural bond**.

The ordering is:

1. Select a random eligible **non-genome** structural bond.
2. Genome bonds remain protected while eligible non-genome structural bonds remain.
3. Once all eligible non-genome structural bonds have been exhausted, genome bonds may become vulnerable.

The physical genome-bond distinction is derived from the existing genome-cavity authority. The qualifying cavity boundary identifies the physical genome bonds; no fixed material recipe, unit count, construction index, or other legacy proxy is introduced.

The previous 2026-09-16 weakest-bond amendment is superseded by this correction.
