#!/usr/bin/env python3
"""Extract a deterministic direct-call map from one Python source file.

The map is lexical evidence. It resolves calls to functions defined in the
same source file when the name is unambiguous. Dynamic dispatch, imports,
reflection, decorators, and higher-order calls remain external boundaries.
"""

from __future__ import annotations

import argparse
import ast
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path


@dataclass(frozen=True)
class FunctionDefinition:
    """One named function or method in lexical source order."""

    qualified_name: str
    line_number: int
    node: ast.FunctionDef | ast.AsyncFunctionDef


class DefinitionCollector(ast.NodeVisitor):
    """Collect every function definition with its class and nesting scope."""

    def __init__(self) -> None:
        self.scope: list[str] = []
        self.definitions: list[FunctionDefinition] = []

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        """Include the class name in qualified method names."""
        self.scope.append(node.name)
        self.generic_visit(node)
        self.scope.pop()

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        """Record a synchronous function and recurse into nested definitions."""
        self._record_and_visit(node)

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        """Record an asynchronous function and recurse into nested definitions."""
        self._record_and_visit(node)

    def _record_and_visit(self, node: ast.FunctionDef | ast.AsyncFunctionDef) -> None:
        qualified_name = ".".join([*self.scope, node.name])
        self.definitions.append(FunctionDefinition(qualified_name, node.lineno, node))
        self.scope.append(node.name)
        self.generic_visit(node)
        self.scope.pop()


class DirectCallCollector(ast.NodeVisitor):
    """Collect calls in one function body without absorbing nested bodies."""

    def __init__(
        self,
        current_function: str,
        local_functions: dict[str, str],
        module_functions: dict[str, str],
    ) -> None:
        self.current_function = current_function
        self.local_functions = local_functions
        self.module_functions = module_functions
        self.internal_calls: set[tuple[str, str]] = set()
        self.external_calls: set[tuple[str, str]] = set()

    def visit_FunctionDef(self, node: ast.FunctionDef) -> None:
        """Leave nested definitions for their own caller context."""
        del node

    def visit_AsyncFunctionDef(self, node: ast.AsyncFunctionDef) -> None:
        """Leave nested definitions for their own caller context."""
        del node

    def visit_ClassDef(self, node: ast.ClassDef) -> None:
        """Leave nested classes for their own caller context."""
        del node

    def visit_Call(self, node: ast.Call) -> None:
        """Classify a direct call before visiting its argument expressions."""
        called_name = call_name(node.func)
        if isinstance(node.func, ast.Name):
            callee = self.local_functions.get(called_name)
            if callee is None:
                callee = self.module_functions.get(called_name)
            if callee is not None:
                self.internal_calls.add((self.current_function, callee))
            else:
                self.external_calls.add((self.current_function, called_name))
        else:
            self.external_calls.add((self.current_function, called_name))
        self.generic_visit(node)


def call_name(node: ast.expr) -> str:
    """Return a compact readable callee representation for one AST node."""
    if isinstance(node, ast.Name):
        return node.id
    if isinstance(node, ast.Attribute):
        return f".{node.attr}"
    return "<expression>"


def direct_nested_functions(
    definition: FunctionDefinition,
    definitions_by_node: dict[int, FunctionDefinition],
) -> dict[str, str]:
    """Return immediate nested functions available by unqualified name."""
    local_functions = {}
    for statement in definition.node.body:
        if isinstance(statement, (ast.FunctionDef, ast.AsyncFunctionDef)):
            nested = definitions_by_node[id(statement)]
            local_functions[statement.name] = nested.qualified_name
    return local_functions


def build_callgraph(source: str, filename: str) -> str:
    """Build a deterministic lexical call-graph report for source text."""
    tree = ast.parse(source, filename=filename)
    definition_collector = DefinitionCollector()
    definition_collector.visit(tree)
    definitions = definition_collector.definitions
    definitions_by_node = {id(definition.node): definition for definition in definitions}
    by_simple_name: dict[str, list[str]] = defaultdict(list)
    for definition in definitions:
        by_simple_name[definition.node.name].append(definition.qualified_name)
    module_functions = {
        name: qualified_names[0]
        for name, qualified_names in by_simple_name.items()
        if len(qualified_names) == 1
    }

    internal_calls: set[tuple[str, str]] = set()
    external_calls: set[tuple[str, str]] = set()
    module_collector = DirectCallCollector("<module>", {}, module_functions)
    for statement in tree.body:
        module_collector.visit(statement)
    internal_calls.update(module_collector.internal_calls)
    external_calls.update(module_collector.external_calls)
    for definition in definitions:
        collector = DirectCallCollector(
            definition.qualified_name,
            direct_nested_functions(definition, definitions_by_node),
            module_functions,
        )
        for statement in definition.node.body:
            collector.visit(statement)
        internal_calls.update(collector.internal_calls)
        external_calls.update(collector.external_calls)

    lines = [
        f"# call graph: {filename}",
        "# method: Python AST direct-call extraction",
        f"# functions defined: {len(definitions)}",
        f"# internal edges: {len(internal_calls)}",
        f"# external call sites: {len(external_calls)}",
        "# boundary: imports, dynamic dispatch, reflection, decorators, and higher-order calls remain external.",
        "## internal edges",
    ]
    lines.extend(f"{caller} -> {callee}" for caller, callee in sorted(internal_calls))
    lines.append("## external calls (dedup by caller,name)")
    lines.extend(f"{caller} -> [external] {callee}" for caller, callee in sorted(external_calls))
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    """Parse command arguments and write the requested call graph."""
    parser = argparse.ArgumentParser(
        description="Extract a lexical direct-call map from Python source"
    )
    parser.add_argument("source", type=Path, help="Python source file to parse")
    parser.add_argument("output", type=Path, help="Call graph text output")
    args = parser.parse_args()

    try:
        source = args.source.read_text(encoding="utf-8")
        output = build_callgraph(source, str(args.source))
    except (OSError, SyntaxError) as error:
        parser.error(str(error))

    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(output, encoding="utf-8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
