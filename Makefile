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

build-workload:
	docker build --no-cache -t ${registry_url}/workload:latest -f workload/Dockerfile workload

build-masp-indexer:
	docker build --no-cache -t ${registry_url}/masp-indexer-webserver:${masp_sha} - < masp-indexer/webserver/Dockerfile --build-arg GIT_SHA=${masp_sha}
	docker build --no-cache -t ${registry_url}/masp-indexer-chain:${masp_sha} - < masp-indexer/chain/Dockerfile --build-arg GIT_SHA=${masp_sha}
	docker build --no-cache -t ${registry_url}/masp-indexer-block-filter:${masp_sha} - < masp-indexer/block-filter/Dockerfile --build-arg GIT_SHA=${masp_sha}