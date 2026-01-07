# Game Design (Keen/KenKen Puzzle)

Summary
- A logic puzzle game based on KenKen/Keen rules, inspired by KeenForAndroid.
- Focus on fast iteration, clean UX, and verifiable puzzle quality.

Design pillars
- Fairness: every puzzle has a unique solution.
- Clarity: constraints visible; no hidden rules.
- Flow: fast entry, quick notes, undo/redo.
- Depth: scalable sizes and difficulty without gimmicks.

Core loop
- Choose size and difficulty.
- Solve cages with row/column constraints.
- Use notes and hints; complete puzzle.
- Record time, hints, and errors for progress tracking.
Modes
- Quick Play: random puzzle with selectable size/difficulty.
- Daily: one puzzle per day with global seed.
- Campaign: curated progression by mechanic and size.
- Custom: size, operation set, and hint policy.

UX conventions
- Tap/click cell to enter digits; notes for candidates.
- Instant constraint feedback and optional error highlighting.
- Undo/redo and restart are always available.

Quality gates
- Generator must guarantee unique solution.
- Validator must prove correctness for completed grids.
- Hints must be derivable from logical steps, not brute force.

Licensing note
- No code or assets copied from GPL projects; rules are public and documented.

Open questions
- Confirm which "Steve" reference is required for attribution.

References
- https://raw.githubusercontent.com/Yegie/KeenForAndroid/master/README
- https://www.chiark.greenend.org.uk/~sgtatham/puzzles/doc/keen.html
- https://www.kenkenpuzzle.com/
