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

#[polars_expr(output_type=Boolean)]
pub fn cidr_contains_any(inputs: &[Series]) -> PolarsResult<Series> {
    polars_ensure!(
        inputs.len() == 2,
        ComputeError: "cidr.contains_any expects 2 arguments (expression, cidr list expression or literal)"
    );

    let series = inputs[0].str()?;
    let len = series.len();
    let name = series.name().clone();
    let subnets = resolve_network_list_argument(&inputs[1], "subnets", len)?;

    let mut builder = BooleanChunkedBuilder::new(name, len);
    for (idx, value) in series.into_iter().enumerate() {
        match (parse_optional_network(value), subnets.values_at(idx)) {
            (Some(network), Some(candidate_subnets)) => {
                let contains_any = candidate_subnets
                    .iter()
                    .any(|candidate| network_contains(&network, candidate));
                builder.append_value(contains_any)
            }
            _ => builder.append_null(),
        }
    }

    Ok(builder.finish().into_series())
}

#[polars_expr(output_type=Boolean)]
pub fn cidr_subnet_of_any(inputs: &[Series]) -> PolarsResult<Series> {
    polars_ensure!(
        inputs.len() == 2,
        ComputeError: "cidr.subnet_of_any expects 2 arguments (expression, cidr list expression or literal)"
    );

    let series = inputs[0].str()?;
    let len = series.len();
    let name = series.name().clone();
    let supernets = resolve_network_list_argument(&inputs[1], "supernets", len)?;

    let mut builder = BooleanChunkedBuilder::new(name, len);
    for (idx, value) in series.into_iter().enumerate() {
        match (parse_optional_network(value), supernets.values_at(idx)) {
            (Some(network), Some(candidate_supernets)) => {
                let is_subnet = candidate_supernets
                    .iter()
                    .any(|candidate| network_contains(candidate, &network));
                builder.append_value(is_subnet)
            }
            _ => builder.append_null(),
        }
    }

    Ok(builder.finish().into_series())
}

#[polars_expr(output_type=Boolean)]
pub fn cidr_is_root(inputs: &[Series]) -> PolarsResult<Series> {
    polars_ensure!(
        inputs.len() == 1,
        ComputeError: "cidr.is_root expects 1 argument (expression)"
    );

    let series = inputs[0].str()?;
    let len = series.len();
    let name = series.name().clone();
    let networks = series.into_iter().map(parse_optional_network).collect::<Vec<_>>();

    let mut builder = BooleanChunkedBuilder::new(name, len);
    for (idx, current) in networks.iter().enumerate() {
        match current {
            Some(network) => {
                let mut has_parent = false;
                for (other_idx, candidate) in networks.iter().enumerate() {
                    if other_idx == idx {
                        continue;
                    }

                    if let Some(candidate_network) = candidate {
                        if candidate_network != network
                            && network_contains(candidate_network, network)
                        {
                            has_parent = true;
                            break;
                        }
                    }
                }

                builder.append_value(!has_parent);
            }
            None => builder.append_null(),
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

        let network = value.parse::<IpNetwork>().map_err(
            |err| polars_err!(ComputeError: "invalid {} CIDR '{}': {}", arg_name, value, err),
        )?;

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

enum NetworkListArgument {
    Literal(Vec<IpNetwork>),
    Column(Vec<IpNetwork>),
    Series(Vec<Option<Vec<IpNetwork>>>),
}

impl NetworkListArgument {
    fn values_at(&self, idx: usize) -> Option<&[IpNetwork]> {
        match self {
            NetworkListArgument::Literal(values) | NetworkListArgument::Column(values) => {
                Some(values.as_slice())
            }
            NetworkListArgument::Series(rows) => rows
                .get(idx)
                .and_then(|row| row.as_ref().map(|values| values.as_slice())),
        }
    }
}

fn parse_optional_network(value: Option<&str>) -> Option<IpNetwork> {
    value.and_then(|text| text.parse::<IpNetwork>().ok())
}

fn resolve_network_list_argument(
    series: &Series,
    arg_name: &str,
    expected_len: usize,
) -> PolarsResult<NetworkListArgument> {
    if let Ok(list) = series.list() {
        return resolve_list_argument(list, arg_name, expected_len);
    }

    if series.str().is_ok() {
        return resolve_string_argument_as_list(series, arg_name);
    }

    let dtype = series.dtype();
    polars_bail!(
        ComputeError: "{} argument must be a literal or expression containing CIDR strings or lists (got {:?})",
        arg_name,
        dtype
    )
}

fn resolve_list_argument(
    list: &ListChunked,
    arg_name: &str,
    expected_len: usize,
) -> PolarsResult<NetworkListArgument> {
    let len = list.len();

    if len == 1 {
        let value_series = list
            .get_as_series(0)
            .ok_or_else(|| polars_err!(ComputeError: "{} argument cannot be null", arg_name))?;

        let networks = parse_literal_network_list(&value_series, arg_name)?;
        return Ok(NetworkListArgument::Literal(networks));
    }

    polars_ensure!(
        len == expected_len,
        ComputeError: "{} argument must be a literal or expression with {} rows (got {})",
        arg_name,
        expected_len,
        len
    );

    let mut rows = Vec::with_capacity(len);
    for row_series in list.clone().into_iter() {
        match row_series {
            Some(inner) => rows.push(parse_expression_network_list(inner)),
            None => rows.push(None),
        }
    }

    Ok(NetworkListArgument::Series(rows))
}

fn resolve_string_argument_as_list(
    series: &Series,
    arg_name: &str,
) -> PolarsResult<NetworkListArgument> {
    let chunked = series.str()?;

    if chunked.len() == 1 {
        let value = chunked
            .get(0)
            .ok_or_else(|| polars_err!(ComputeError: "{} argument cannot be null", arg_name))?;

        let network = value.parse::<IpNetwork>().map_err(
            |err| polars_err!(ComputeError: "invalid {} CIDR '{}': {}", arg_name, value, err),
        )?;

        return Ok(NetworkListArgument::Literal(vec![network]));
    }

    let networks = chunked
        .into_iter()
        .filter_map(parse_optional_network)
        .collect::<Vec<_>>();

    Ok(NetworkListArgument::Column(networks))
}

fn parse_literal_network_list(series: &Series, arg_name: &str) -> PolarsResult<Vec<IpNetwork>> {
    let chunked = series.str()?;
    let mut networks = Vec::with_capacity(chunked.len());

    for value in chunked.into_iter() {
        let text = value.ok_or_else(
            || polars_err!(ComputeError: "{} list argument cannot contain null values", arg_name),
        )?;

        let network = text.parse::<IpNetwork>().map_err(
            |err| polars_err!(ComputeError: "invalid {} CIDR '{}': {}", arg_name, text, err),
        )?;

        networks.push(network);
    }

    Ok(networks)
}

fn parse_expression_network_list(series: Series) -> Option<Vec<IpNetwork>> {
    let chunked = series.str().ok()?;
    let mut networks = Vec::with_capacity(chunked.len());

    for value in chunked.into_iter() {
        match value {
            Some(text) => match text.parse::<IpNetwork>() {
                Ok(network) => networks.push(network),
                Err(_) => return None,
            },
            None => continue,
        }
    }

    Some(networks)
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
