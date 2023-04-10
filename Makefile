# IMG ?= apecloud/ape-dts
BUILD_IMG ?= caiqynb/ape-dts-env
RELASE_IMG ?= caiqynb/ape-dts
VERSION ?= 0.1
CONFIG_PATH ?= ./images/example/mysql_snapshot_sample.yaml
MODULE_NAME ?= ape-dts

# make release-build GIT_TOKEN=xxx
.PHONY: release-build
release-build:
	cd images && ./build.sh "$(BUILD_IMG):$(VERSION)" "$(GIT_TOKEN)"
# make docker-build RELASE_IMG=apecloud/ape-dts MODULE_NAME=ape-dts CONFIG_PATH=xxx
.PHONY: docker-build
docker-build:
	docker build -t $(RELASE_IMG):$(VERSION) --build-arg LOCAL_CONFIG_PATH=$(CONFIG_PATH) --build-arg MODULE_NAME=$(MODULE_NAME) -f Dockerfile_release . 


