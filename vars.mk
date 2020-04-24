REPO_PREFIX := daschswiss
ARK_RESOLVER_REPO := ark-resolver

ifeq ($(BUILD_TAG),)
  BUILD_TAG := $(shell git describe --tag --dirty --abbrev=7)
endif
ifeq ($(BUILD_TAG),)
  BUILD_TAG := $(shell git rev-parse --verify HEAD)
endif

ifeq ($(ARK_RESOLVER_IMAGE),)
  ARK_RESOLVER_IMAGE := $(REPO_PREFIX)/$(ARK_RESOLVER_REPO):$(BUILD_TAG)
endif
