use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use polars::prelude::*;
use pyo3::prelude::*;
use pyo3_polars::derive::polars_expr;

pub fn register(_module: &Bound<'_, PyModule>) -> PyResult<()> {
    Ok(())
}

#[polars_expr(output_type=Boolean)]
pub fn cidr_contains(inputs: &[Series]) -> PolarsResult<Series> {
    polars_ensure!(
        inputs.len() == 2,
        ComputeError: "cidr.contains expects 2 arguments (expression, cidr expression or literal)"
    );

    let series = inputs[0].str()?;
    let len = series.len();
    let name = series.name().clone();
    let needle = resolve_network_argument(&inputs[1], "needle", len)?;

    let mut builder = BooleanChunkedBuilder::new(name, len);
    for (idx, value) in series.into_iter().enumerate() {
        match (parse_optional_network(value), needle.value_at(idx)) {
            (Some(network), Some(needle_network)) => {
                builder.append_value(network_contains(&network, needle_network))
            }
            _ => builder.append_null(),
        }
    }

    Ok(builder.finish().into_series())
}

#[polars_expr(output_type=Boolean)]
pub fn cidr_subnet_of(inputs: &[Series]) -> PolarsResult<Series> {
    polars_ensure!(
        inputs.len() == 2,
        ComputeError: "cidr.subnet_of expects 2 arguments (expression, cidr expression or literal)"
    );

    let series = inputs[0].str()?;
    let len = series.len();
    let name = series.name().clone();
    let supernet = resolve_network_argument(&inputs[1], "supernet", len)?;

    let mut builder = BooleanChunkedBuilder::new(name, len);
    for (idx, value) in series.into_iter().enumerate() {
        match (parse_optional_network(value), supernet.value_at(idx)) {
            (Some(network), Some(supernet_network)) => {
                builder.append_value(network_contains(supernet_network, &network))
            }
            _ => builder.append_null(),
        }
    }

    Ok(builder.finish().into_series())
}

enum NetworkArgument {
    Literal(IpNetwork),
    Series(Vec<Option<IpNetwork>>),
}

impl NetworkArgument {
    fn value_at(&self, idx: usize) -> Option<&IpNetwork> {
        match self {
            NetworkArgument::Literal(network) => Some(network),
            NetworkArgument::Series(values) => values.get(idx).and_then(|value| value.as_ref()),
        }
    }
}

fn resolve_network_argument(
    series: &Series,
    arg_name: &str,
    expected_len: usize,
) -> PolarsResult<NetworkArgument> {
    let chunked = series.str()?;

    if chunked.len() == 1 {
        let value = chunked
            .get(0)
            .ok_or_else(|| polars_err!(ComputeError: "{} argument cannot be null", arg_name))?;

        let network = value.parse::<IpNetwork>().map_err(|err| {
            polars_err!(ComputeError: "invalid {} CIDR '{}': {}", arg_name, value, err)
        })?;

        return Ok(NetworkArgument::Literal(network));
    }

    polars_ensure!(
        chunked.len() == expected_len,
        ComputeError: "{} argument must be a literal or expression with {} rows (got {})",
        arg_name,
        expected_len,
        chunked.len()
    );

    let parsed_values = chunked
        .into_iter()
        .map(parse_optional_network)
        .collect::<Vec<_>>();

    Ok(NetworkArgument::Series(parsed_values))
}

fn parse_optional_network(value: Option<&str>) -> Option<IpNetwork> {
    value.and_then(|text| text.parse::<IpNetwork>().ok())
}

fn network_contains(supernet: &IpNetwork, subnet: &IpNetwork) -> bool {
    match (supernet, subnet) {
        (IpNetwork::V4(super_v4), IpNetwork::V4(sub_v4)) => contains_ipv4(super_v4, sub_v4),
        (IpNetwork::V6(super_v6), IpNetwork::V6(sub_v6)) => contains_ipv6(super_v6, sub_v6),
        _ => false,
    }
}

fn contains_ipv4(supernet: &Ipv4Network, subnet: &Ipv4Network) -> bool {
    if supernet.prefix() > subnet.prefix() {
        return false;
    }

    let mask = ipv4_prefix_mask(supernet.prefix());
    u32::from(supernet.network()) == (u32::from(subnet.network()) & mask)
}

fn contains_ipv6(supernet: &Ipv6Network, subnet: &Ipv6Network) -> bool {
    if supernet.prefix() > subnet.prefix() {
        return false;
    }

    let mask = ipv6_prefix_mask(supernet.prefix());
    u128::from(supernet.network()) == (u128::from(subnet.network()) & mask)
}

fn ipv4_prefix_mask(prefix: u8) -> u32 {
    if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - u32::from(prefix))
    }
}

fn ipv6_prefix_mask(prefix: u8) -> u128 {
    if prefix == 0 {
        0
    } else {
        u128::MAX << (128 - u32::from(prefix))
    }
}
