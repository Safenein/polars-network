use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
use polars::prelude::*;
use std::net::{Ipv4Addr, Ipv6Addr};
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

#[polars_expr(output_type=String)]
pub fn cidr_network_address(inputs: &[Series]) -> PolarsResult<Series> {
    polars_ensure!(
        inputs.len() == 1,
        ComputeError: "cidr.network_address expects 1 argument (expression)"
    );

    let series = inputs[0].str()?;
    let len = series.len();
    let name = series.name().clone();
    let mut builder = StringChunkedBuilder::new(name, len);

    for value in series.into_iter() {
        match parse_optional_network(value) {
            Some(network) => {
                let addr = network_address_string(&network);
                builder.append_value(&addr);
            }
            None => builder.append_null(),
        }
    }

    Ok(builder.finish().into_series())
}

#[polars_expr(output_type=String)]
pub fn cidr_broadcast_address(inputs: &[Series]) -> PolarsResult<Series> {
    polars_ensure!(
        inputs.len() == 1,
        ComputeError: "cidr.broadcast_address expects 1 argument (expression)"
    );

    let series = inputs[0].str()?;
    let len = series.len();
    let name = series.name().clone();
    let mut builder = StringChunkedBuilder::new(name, len);

    for value in series.into_iter() {
        match parse_optional_network(value) {
            Some(network) => {
                let addr = broadcast_address_string(&network);
                builder.append_value(&addr);
            }
            None => builder.append_null(),
        }
    }

    Ok(builder.finish().into_series())
}

#[polars_expr(output_type=Int64)]
pub fn cidr_netmask(inputs: &[Series]) -> PolarsResult<Series> {
    polars_ensure!(
        inputs.len() == 1 || inputs.len() == 2,
        ComputeError: "cidr.netmask expects 1 or 2 arguments (expression, optional binary flag)"
    );

    let series = inputs[0].str()?;
    let len = series.len();
    let name = series.name().clone();

    let binary = if inputs.len() == 2 {
        resolve_bool_argument(&inputs[1], "binary", len)?
    } else {
        BoolArgument::Literal(false)
    };

    let mut results: Vec<Option<i64>> = Vec::with_capacity(len);
    for (idx, value) in series.into_iter().enumerate() {
        let entry = match (parse_optional_network(value), binary.value_at(idx)) {
            (Some(network), Some(is_binary)) => {
                if is_binary {
                    match network {
                        IpNetwork::V4(net) => Some(i64::from(u32::from(net.mask()))),
                        IpNetwork::V6(_) => None,
                    }
                } else {
                    Some(i64::from(network.prefix()))
                }
            }
            (Some(_), None) => None,
            _ => None,
        };

        results.push(entry);
    }

    let chunked = Int64Chunked::from_iter(results);
    Ok(chunked.with_name(name).into_series())
}

#[polars_expr(output_type=Int64)]
pub fn cidr_version(inputs: &[Series]) -> PolarsResult<Series> {
    polars_ensure!(
        inputs.len() == 1,
        ComputeError: "cidr.version expects 1 argument (expression)"
    );

    let series = inputs[0].str()?;
    let len = series.len();
    let name = series.name().clone();

    let mut values = Vec::with_capacity(len);
    for value in series.into_iter() {
        let entry = parse_optional_network(value).map(|network| match network {
            IpNetwork::V4(_) => 4,
            IpNetwork::V6(_) => 6,
        });
        values.push(entry.map(i64::from));
    }

    let chunked = Int64Chunked::from_iter(values);
    Ok(chunked.with_name(name).into_series())
}

#[polars_expr(output_type=String)]
pub fn cidr_supernet(inputs: &[Series]) -> PolarsResult<Series> {
    polars_ensure!(
        inputs.len() == 1,
        ComputeError: "cidr.supernet expects 1 argument (expression)"
    );

    let series = &inputs[0];
    let name = series.name().clone();

    match series.dtype() {
        DataType::String => {
            let chunked = series.str()?;
            let mut networks = Vec::with_capacity(chunked.len());
            for value in chunked.into_iter() {
                if let Some(network) = parse_optional_network(value) {
                    networks.push(network);
                }
            }

            let result = minimal_supernet(&networks).map(|net| net.to_string());
            let chunked = StringChunked::from_iter([result]);
            Ok(chunked.with_name(name).into_series())
        }
        dtype => polars_bail!(
            ComputeError: "cidr.supernet expects UTF-8 values (got {:?})",
            dtype
        ),
    }
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

enum BoolArgument {
    Literal(bool),
    Series(Vec<Option<bool>>),
}

impl BoolArgument {
    fn value_at(&self, idx: usize) -> Option<bool> {
        match self {
            BoolArgument::Literal(value) => Some(*value),
            BoolArgument::Series(values) => values.get(idx).copied().flatten(),
        }
    }
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

fn resolve_bool_argument(
    series: &Series,
    arg_name: &str,
    expected_len: usize,
) -> PolarsResult<BoolArgument> {
    let chunked = if series.dtype() == &DataType::Boolean {
        series.bool()?.clone()
    } else {
        polars_bail!(
            ComputeError: "{} argument must be a literal or expression containing booleans (got {:?})",
            arg_name,
            series.dtype()
        )
    };

    if chunked.len() == 1 {
        let value = chunked.get(0).ok_or_else(|| {
            polars_err!(ComputeError: "{} argument cannot be null", arg_name)
        })?;

        return Ok(BoolArgument::Literal(value));
    }

    polars_ensure!(
        chunked.len() == expected_len,
        ComputeError: "{} argument must be a literal or expression with {} rows (got {})",
        arg_name,
        expected_len,
        chunked.len()
    );

    Ok(BoolArgument::Series(chunked.into_iter().collect()))
}

fn minimal_supernet(networks: &[IpNetwork]) -> Option<IpNetwork> {
    if networks.is_empty() {
        return None;
    }

    let mut ipv4 = Vec::new();
    let mut ipv6 = Vec::new();

    for network in networks {
        match network {
            IpNetwork::V4(v4) => ipv4.push(v4.clone()),
            IpNetwork::V6(v6) => ipv6.push(v6.clone()),
        }
    }

    match (ipv4.is_empty(), ipv6.is_empty()) {
        (false, true) => minimal_supernet_ipv4(&ipv4).map(IpNetwork::V4),
        (true, false) => minimal_supernet_ipv6(&ipv6).map(IpNetwork::V6),
        _ => None,
    }
}

fn minimal_supernet_ipv4(networks: &[Ipv4Network]) -> Option<Ipv4Network> {
    if networks.is_empty() {
        return None;
    }

    let mut min_addr = u32::MAX;
    let mut max_addr = 0_u32;

    for network in networks {
        let network_addr = u32::from(network.network());
        let broadcast_addr = u32::from(network.broadcast());
        min_addr = min_addr.min(network_addr);
        max_addr = max_addr.max(broadcast_addr);
    }

    let diff = min_addr ^ max_addr;
    let prefix = diff.leading_zeros() as u8;

    let mask = ipv4_prefix_mask(prefix);
    let network_u32 = min_addr & mask;

    Ipv4Network::new(Ipv4Addr::from(network_u32), prefix).ok()
}

fn minimal_supernet_ipv6(networks: &[Ipv6Network]) -> Option<Ipv6Network> {
    if networks.is_empty() {
        return None;
    }

    let mut min_addr = u128::MAX;
    let mut max_addr = 0_u128;

    for network in networks {
        let network_addr = u128::from(network.network());
        let broadcast_addr = u128::from(ipv6_broadcast_address(network));
        min_addr = min_addr.min(network_addr);
        max_addr = max_addr.max(broadcast_addr);
    }

    let diff = min_addr ^ max_addr;
    let prefix = diff.leading_zeros() as u8;

    let mask = ipv6_prefix_mask(prefix);
    let network_u128 = min_addr & mask;

    Ipv6Network::new(Ipv6Addr::from(network_u128), prefix).ok()
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

fn network_address_string(network: &IpNetwork) -> String {
    match network {
        IpNetwork::V4(net) => net.network().to_string(),
        IpNetwork::V6(net) => net.network().to_string(),
    }
}

fn broadcast_address_string(network: &IpNetwork) -> String {
    match network {
        IpNetwork::V4(net) => net.broadcast().to_string(),
        IpNetwork::V6(net) => ipv6_broadcast_address(net).to_string(),
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

fn ipv6_broadcast_address(network: &Ipv6Network) -> Ipv6Addr {
    let mask = ipv6_prefix_mask(network.prefix());
    let host_mask = !mask;
    Ipv6Addr::from(u128::from(network.network()) | host_mask)
}

