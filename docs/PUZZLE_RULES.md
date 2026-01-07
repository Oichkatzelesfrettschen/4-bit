# Puzzle Rules (Keen/KenKen)

This document defines the core puzzle rules for the Keen/KenKen-inspired game.

Core rules
- Grid is N x N (N in 3..9 for digits 1..N).
- Each row contains each digit exactly once.
- Each column contains each digit exactly once.
- Grid is partitioned into cages (blocks) with a target and operation.
- Digits in a cage must combine to the target using the operation.
- Operations: +, -, *, /.
- Subtraction and division cages are size 2 only; order does not matter.
- Repeated digits in a cage are allowed if they are not in the same row or column.

Terminology
- Cage: outlined block of cells with a target and operation.
- Note/pencil mark: candidate digits for a cell.
References
- Simon Tatham's Portable Puzzle Collection (Keen rules): https://www.chiark.greenend.org.uk/~sgtatham/puzzles/doc/keen.html (retrieved 2026-01-07)
- KenKen official site (How to Play UI conventions): https://www.kenkenpuzzle.com/ (retrieved 2026-01-07)
- KeenForAndroid README (naming and inspiration): https://raw.githubusercontent.com/Yegie/KeenForAndroid/master/README (retrieved 2026-01-07)
