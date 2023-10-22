mount:
	sshfs danya@10.0.0.2:/hugedata/lichess-data ./hugedata

retrieve_libdeps:
	mkdir -p extra_lib
	cp /home/danya/.local/lib/python3.11/site-packages/torch/lib/* ./extra_lib/

build:
	LIBTORCH_USE_PYTORCH=1 cargo build --release --package web_api
	docker buildx build --platform linux/amd64 . --tag registry.danya02.ru/unchessful/api:v1 --push