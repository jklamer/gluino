mod ser;
mod spec;
mod util;

pub fn change_data() {
    println!("Today")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(4, 4);
    }
}
