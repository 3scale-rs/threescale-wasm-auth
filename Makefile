MKFILE_PATH := $(abspath $(lastword $(MAKEFILE_LIST)))
PROJECT_PATH := $(patsubst %/,%,$(dir $(MKFILE_PATH)))
COMPOSEFILE := $(PROJECT_PATH)/compose/docker-compose.yaml
DOCKER_COMPOSE := docker-compose -f $(COMPOSEFILE)
OPEN_APP ?= xdg-open

.PHONY: build
build: export TARGET?=wasm32-unknown-unknown
build: export BUILD?=debug
build: ## Build WASM filter
	if test "x$(BUILD)" = "xrelease"; then \
	  cargo build --target=$(TARGET) --release $(CARGO_EXTRA_ARGS) ; \
	else \
	  cargo build --target=$(TARGET) $(CARGO_EXTRA_ARGS) ; \
	fi
	mkdir -p $(PROJECT_PATH)/compose/wasm
	ln -sf ../../target/$(TARGET)/$(BUILD)/threescale_wasm_auth.wasm $(PROJECT_PATH)/compose/wasm/

clean: ## Clean WASM filter
	cargo clean
	rm -f $(PROJECT_PATH)/compose/wasm/threescale_wasm_auth.wasm

.PHONY: doc
doc: ## Open project documentation
	cargo doc --open

.PHONY: up
up: ## Start docker-compose containers
	$(DOCKER_COMPOSE) up

.PHONY: stop
stop: ## Stop docker-compose containers
	$(DOCKER_COMPOSE) stop

.PHONY: status
status: ## Status of docker-compose containers
	$(DOCKER_COMPOSE) ps

.PHONY: top
top: ## Show runtime information about docker-compose containers
	$(DOCKER_COMPOSE) top

kill: ## Force-stop docker-compose containers
	$(DOCKER_COMPOSE) kill

.PHONY: down
down: ## Stop and remove containers and other docker-compose resources
	$(DOCKER_COMPOSE) down

.PHONY: proxy-info
proxy-info: export INDEX?=1
proxy-info: ## Obtain the local host address and port for a service (use SERVICE, PORT and optionally INDEX)
	$(DOCKER_COMPOSE) port --index $(INDEX) $(SERVICE) $(PORT)

.PHONY: proxy-url
proxy-url: export INDEX?=1
proxy-url: export SCHEME?=http
proxy-url: ## Obtain a URL for the given service (use SERVICE, PORT and optionally INDEX)
	$(DOCKER_COMPOSE) port --index $(INDEX) $(SERVICE) $(PORT)

.PHONY: proxy
proxy: export INDEX?=1
proxy: export SCHEME?=http
proxy: LOCALURL=$(shell $(DOCKER_COMPOSE) port --index $(INDEX) $(SERVICE) $(PORT))
proxy: ## Open service and port in a browser (same as proxy-info, but optionally define SCHEME and OPEN_APP)
	$(OPEN_APP) $(SCHEME)://$(LOCALURL)

.PHONY: ingress-helper
ingress-helper: export SERVICE?=ingress
ingress-helper: export PORT?=80
ingress-helper: export TARGET?=proxy-url
ingress-helper:
	$(MAKE) $(TARGET)

.PHONY: ingress-url
ingress-url: ## Show the ingress URL
	$(MAKE) ingress-helper

.PHONY: ingress-open
ingress-open: export TARGET?=proxy
ingress-open: ## Open the ingress URL
	$(MAKE) ingress-helper

.PHONY: ingress-admin-url
ingress-admin-url: export PORT?=8001
ingress-admin-url: ## Show the ingress admin URL
	$(MAKE) ingress-helper

.PHONY: ingress-admin-open
ingress-admin-open: export PORT?=8001
ingress-admin-open: export TARGET?=proxy
ingress-admin-open: ## Open the ingress admin URL
	$(MAKE) ingress-helper

.PHONY: curl
curl: export SCHEME?=http
curl: export SERVICE?=ingress
curl: export INDEX?=1
curl: export PORT?=80
curl: export HOST?=web.app
curl: export USER_KEY?=invalid-key
curl: export TARGET?=$$($(DOCKER_COMPOSE) port --index $(INDEX) $(SERVICE) $(PORT))
curl: ## Perform a request to a specific service (default ingress:80 with Host: web.app, please set USER_KEY)
	curl -vvv -H "Host: $(HOST)" -H "X-API-Key: $(USER_KEY)" "$(SCHEME)://$(TARGET)/$(SVC_PATH)"

.PHONY: curl-web.app
curl-web.app: export USER_KEY?=$(WEB_KEY)
curl-web.app: ## Perform a curl call to web.app (make sure to export secrets)
	$(MAKE) curl

# Check http://marmelab.com/blog/2016/02/29/auto-documented-makefile.html
.PHONY: help
help: ## Print this help
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)
