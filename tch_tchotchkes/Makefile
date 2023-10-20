run:
	LD_LIBRARY_PATH=/home/danya/.local/lib/python3.11/site-packages/torch/lib cargo run --release

test:
	LD_LIBRARY_PATH=/home/danya/.local/lib/python3.11/site-packages/torch/lib cargo test


get_deps:
	wget https://download.pytorch.org/libtorch/cpu/libtorch-shared-with-deps-2.1.0%2Bcpu.zip
	unzip libtorch-shared-with-deps-2.1.0+cpu.zip

get_mnist:
	mkdir -p mnist-data
	cd mnist-data && wget https://storage.googleapis.com/cvdf-datasets/mnist/train-images-idx3-ubyte.gz
	cd mnist-data && wget https://storage.googleapis.com/cvdf-datasets/mnist/train-labels-idx1-ubyte.gz
	cd mnist-data && wget https://storage.googleapis.com/cvdf-datasets/mnist/t10k-images-idx3-ubyte.gz
	cd mnist-data && wget https://storage.googleapis.com/cvdf-datasets/mnist/t10k-labels-idx1-ubyte.gz
	cd mnist-data && gunzip *