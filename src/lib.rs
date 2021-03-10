#![cfg_attr(feature_gate_test, feature(test))]
#![cfg_attr(feature_gate_unsafe_op_in_unsafe_fn, feature(unsafe_op_in_unsafe_fn))]

mod configuration;
mod proxy;
mod upstream;
mod util;

#[cfg(test)]
mod test {
    #[test]
    fn it_works() {
        assert!(true);
    }
}
