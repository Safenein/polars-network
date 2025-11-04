from __future__ import annotations

from pathlib import Path
from typing import Sequence

import polars as pl
from polars._typing import IntoExpr

PLUGIN_PATH: Path
__version__: str


def _plugin_expr(function_name: str, args: Sequence[pl.Expr]) -> pl.Expr: ...


def _plugin_agg(function_name: str, args: Sequence[pl.Expr]) -> pl.Expr: ...


def _to_expr(value: IntoExpr) -> pl.Expr: ...


class CidrNamespace:
    _expr: pl.Expr

    def __init__(self, expr: pl.Expr) -> None: ...

    def contains(self, other: IntoExpr) -> pl.Expr: ...

    def subnet_of(self, other: IntoExpr) -> pl.Expr: ...

    def contains_any(self, other: IntoExpr) -> pl.Expr: ...

    def subnet_of_any(self, other: IntoExpr) -> pl.Expr: ...

    def is_root(self) -> pl.Expr: ...

    def network_address(self) -> pl.Expr: ...

    def broadcast_address(self) -> pl.Expr: ...

    def netmask(self, binary: IntoExpr = False) -> pl.Expr: ...

    def version(self) -> pl.Expr: ...

    def supernet(self) -> pl.Expr: ...


__all__: list[str]

Expr = pl.Expr
Expr.cidr: CidrNamespace = ...  # type: ignore[misc]
