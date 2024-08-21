numactl -N 0 -- ./apps/synthetic/target/release/synthetic 10.3.3.3:2333 --config client.config --threads 16 --mode runtime-client --protocol synthetic --transport udp --runtime 5000000000 \
	--slowdown \
	--rampup 0 \
	-d rocksdb \
	--mean 1000 \
	--samples 20 --start_mpps 0 --mpps 0.05
	# --samples 1 --start_mpps 0.002 --mpps 0.002
	# --samples 10 --start_mpps 0 --mpps 0.05

