pub mod spec;
mod fingerprint;
pub mod serde;
pub mod spec_parsing;
#[cfg(test)]
mod test_utils;
mod util;
mod compiled_spec_visitor_pattern;

pub fn change_data() {
    println!("Today")
}
