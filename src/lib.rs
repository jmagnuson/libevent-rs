#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_access() {
        assert_eq!(EVENT_LOG_MSG, 1);
        assert_eq!(IPPORT_RESERVED, 1024);
    }
}
