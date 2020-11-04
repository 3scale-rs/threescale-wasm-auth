# Proxy-WASM Authorization extension using 3scale

![Clippy check](https://github.com/3scale-rs/threescale-wasm-auth/workflows/Clippy%20check/badge.svg)
![Security audit](https://github.com/3scale-rs/threescale-wasm-auth/workflows/Security%20audit/badge.svg)
![Continuous integration](https://github.com/3scale-rs/threescale-wasm-auth/workflows/Continuous%20integration/badge.svg)

This is just a simple in-progress integration of 3scale Authorization for proxy-wasm.

To run the demo:

1. Edit lds.conf in compose/envoy to fill in service data (ids, tokens, rules, ...).
1.1 Optionally edit compose/envoy/envoy.yaml to point the 3scale SaaS cluster to your 3scale (backend) instance.
2. Run `make build` to build the WebAssembly extension.
3. Run `make up` to run the docker-compose environment.
4. Create a `secrets` file with the following contents:
```shell
export WEB_KEY=<a user_key for the service handling the web.app backend>
export ECHO_API_KEY=<a user_key for the service handling the echo-api.app backend>
```
5. Run `source secrets`.
6. Run `make curl-web.app` or `make curl-echo-api.app`.
6.1 Optionally specify a path to hit a specific pattern rule: `make SVC_PATH=products/1/sales curl-web.app` (N.B. no initial slash!)

If you set up limits, those should be respected by this plug-in, and reporting
should be happening and visible in your 3scale dashboard.

### Known issues

- We currently only really support passing in a user_key type of secret as a header.
- We just do a simple authrep and look for a 200 status code rather than parsing responses, so we don't yet know whether an actual rate limiting condition happened.
- Some configuration items are not being used at all, since it is a very early WIP.

## Next features

- We plan to add several features for completeness and support loading the
configuration from the 3scale APIs rather than embedding it.
- We might add support for communicating with the 3scale Istio Adapter/Authorizer via gRPC/HTTP.
- The code needs refactoring and automated testing.
