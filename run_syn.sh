# numactl -N 0 -- ./apps/synthetic/target/release/synthetic 10.3.3.3:2333 --config client.config --threads 16 --mode runtime-client --protocol synthetic --transport udp --runtime 5000000000 \
# 	--rampup 0 \
# 	-d bimodal1 \
# 	--mean 1000 \
# 	--samples 30 --mpps 0.3

numactl -N 0 -- ./apps/synthetic/target/release/synthetic 10.3.3.3:2333 --config client.config --threads 16 --mode runtime-client --protocol synthetic --transport udp --runtime 5000000000 \
	-s \
	--rampup 0 \
	-d bimodal2 \
	--mean 1000 \
	--samples 10 --start_mpps 3 --mpps 4
	# --samples 40 --start_mpps 0 --mpps 4


