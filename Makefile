build-config:
	docker build --no-cache -t ${registry_url}/namada-config:${sha} -f config/Dockerfile --build-arg GIT_SHA=${sha} config

build-genesis:
	docker build --no-cache -t ${registry_url}/namada-genesis:${sha} -f genesis/Dockerfile --build-arg GIT_SHA=${sha} --build-arg GENESIS_TEMPLATE_VERSION=${genesis_template_version} genesis

build-namada:
	docker build --no-cache -t ${registry_url}/namada:${sha} -f namada/Dockerfile --build-arg GIT_SHA=${sha} namada

build-namada-inst:
	docker build --no-cache -t ${registry_url}/namada:${sha}-inst -f namada/Dockerfile.inst --build-arg GIT_SHA=${sha} namada

build-check:
	docker build --no-cache -t ${registry_url}/check:latest -f check/Dockerfile check