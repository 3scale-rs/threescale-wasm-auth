# Proxy-WASM Authorization extension using 3scale

![Clippy check](https://github.com/3scale-rs/threescale-wasm-auth/workflows/Clippy%20check/badge.svg)
![Security audit](https://github.com/3scale-rs/threescale-wasm-auth/workflows/Security%20audit/badge.svg)
![Continuous integration](https://github.com/3scale-rs/threescale-wasm-auth/workflows/Continuous%20integration/badge.svg)

This is just a simple in-progress integration of 3scale Authorization for proxy-wasm.

To run the demo:

1. Edit lds.conf in compose/envoy to fill in service data (ids, tokens, rules, ...).
2. Optionally edit compose/envoy/envoy.yaml to point the 3scale SaaS cluster to your 3scale (backend) instance.
3. Run `make build` to build the WebAssembly extension.
4. Run `make up` to run the docker-compose environment.

### Known issues

- We just do a simple authrep and look for a 200 status code rather than parsing responses, so we don't yet know whether an actual rate limiting condition happened.
- Valid apps configured only apply if you set no backend, and they check for a mapping rule but they don't report and they don't have limits applied.
