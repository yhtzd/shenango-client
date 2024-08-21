numactl -N 0 -- ./apps/synthetic/target/release/synthetic 10.3.3.3:2333 --config client.config --threads 16 --mode runtime-client --protocol synthetic --transport udp --runtime 1000000000 \
	--rampup 0 \
	-d bimodal2 \
	--mean 1000 \
	--samples 12 --start_mpps 0 --mpps 1.2


