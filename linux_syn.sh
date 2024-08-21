numactl -N 0 -- ./apps/synthetic/target/release/synthetic 10.3.3.3:2333 --config client.config --threads 32 --mode linux-client --protocol synthetic --transport udp --runtime 5000000000 \
	-d bimodal2 \
	--mean 1000 \
	--samples 20 --start_mpps 0 --mpps 0.6

