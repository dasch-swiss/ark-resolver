# Determine this makefile's path.
# Be sure to place this BEFORE `include` directives, if any.
# THIS_FILE := $(lastword $(MAKEFILE_LIST))
THIS_FILE := $(abspath $(lastword $(MAKEFILE_LIST)))
CURRENT_DIR := $(shell dirname $(realpath $(firstword $(MAKEFILE_LIST))))

include vars.mk

## ark-resolver
.PHONY: build
build: ## build and publish ark-resolver docker image locally
	docker build -t $(ARK_RESOLVER_IMAGE) -t $(REPO_PREFIX)/$(ARK_RESOLVER_REPO):latest .

.PHONY: test
test: build ## run ark-resolver tests inside docker image
	docker run $(ARK_RESOLVER_IMAGE) --test

.PHONY: run
run: build ## run ark-resolver inside docker image
	docker run $(ARK_RESOLVER_IMAGE)

.PHONY: publish
publish: build ## publish ark-resolver image to Dockerhub
	docker push $(ARK_RESOLVER_IMAGE)

.PHONY: help
help: ## this help
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST) | sort

.DEFAULT_GOAL := help