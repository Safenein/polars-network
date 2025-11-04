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


def _plugin_agg(function_name: str, args: Sequence[pl.Expr]) -> pl.Expr:
    return register_plugin_function(
        plugin_path=PLUGIN_PATH,
        function_name=function_name,
        args=args,
        is_elementwise=False,
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

    def network_address(self) -> pl.Expr:
        """Return an expression with the network address of each CIDR in ``self``."""
        return _plugin_expr("cidr_network_address", (self._expr,))

    def broadcast_address(self) -> pl.Expr:
        """Return an expression with the broadcast address of each CIDR in ``self``."""
        return _plugin_expr("cidr_broadcast_address", (self._expr,))

    def netmask(self, binary: IntoExpr = False) -> pl.Expr:
        """Return an expression with the CIDR prefix length or IPv4 mask when ``binary`` is ``True``."""
        return _plugin_expr("cidr_netmask", (self._expr, _to_expr(binary)))

    def version(self) -> pl.Expr:
        """Return an expression indicating whether each CIDR is IPv4 or IPv6."""
        return _plugin_expr("cidr_version", (self._expr,))

    def supernet(self) -> pl.Expr:
        """Return a group-by aggregation producing the minimal supernet per group."""
        return _plugin_agg("cidr_supernet", (self._expr,))


__all__ = ["CidrNamespace", "__version__"]
