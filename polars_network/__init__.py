from __future__ import annotations

from pathlib import Path
from typing import TYPE_CHECKING

import polars as pl
from polars._typing import IntoExpr
from polars.api import register_expr_namespace
from polars.plugins import register_plugin_function

from . import polars_network as _native

if TYPE_CHECKING:  # pragma: no cover
    from typing import Sequence


PLUGIN_PATH = Path(_native.__file__).parent
__version__ = _native.__version__


def _plugin_expr(function_name: str, args: Sequence[pl.Expr]) -> pl.Expr:
    return register_plugin_function(
        plugin_path=PLUGIN_PATH,
        function_name=function_name,
        args=args,
        is_elementwise=True,
    )


def _to_expr(value: IntoExpr) -> pl.Expr:
    if isinstance(value, pl.Expr):
        return value
    return pl.lit(value)


@register_expr_namespace("cidr")
class CidrNamespace:
    def __init__(self, expr: pl.Expr):
        self._expr = expr

    def contains(self, other: IntoExpr) -> pl.Expr:
        """Return a boolean expression indicating whether ``self`` contains ``other``."""
        return _plugin_expr("cidr_contains", (self._expr, _to_expr(other)))

    def subnet_of(self, other: IntoExpr) -> pl.Expr:
        """Return a boolean expression indicating whether ``self`` is a subnet of ``other``."""
        return _plugin_expr("cidr_subnet_of", (self._expr, _to_expr(other)))

    def contains_any(self, other: IntoExpr) -> pl.Expr:
        """Return a boolean expression indicating whether ``self`` contains any CIDR in ``other``."""
        return _plugin_expr("cidr_contains_any", (self._expr, _to_expr(other)))

    def subnet_of_any(self, other: IntoExpr) -> pl.Expr:
        """Return a boolean expression indicating whether ``self`` is a subnet of any CIDR in ``other``."""
        return _plugin_expr("cidr_subnet_of_any", (self._expr, _to_expr(other)))

    def is_root(self) -> pl.Expr:
        """Return a boolean expression indicating whether ``self`` is not contained in any other CIDR within the column."""
        return _plugin_expr("cidr_is_root", (self._expr,))


__all__ = ["CidrNamespace", "__version__"]
