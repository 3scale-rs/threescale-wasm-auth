use prost::Message;
use prost_types::Struct;

#[derive(Clone, PartialEq, Message)]
pub struct Metadata {
    /// Key is the reverse DNS filter name, e.g. com.acme.widget. The envoy.*
    /// namespace is reserved for Envoy's built-in filters.
    #[prost(map = "string, message", tag = "1")]
    pub filter_metadata: ::std::collections::HashMap<std::string::String, Struct>,
}
