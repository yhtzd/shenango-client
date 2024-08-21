# numactl -N 0 -- ./apps/synthetic/target/release/synthetic 10.3.3.3:11211 --config client.config --threads 32 --mode runtime-client --protocol memcached --samples 10 --start_mpps 2 --mpps 3 --transport tcp --runtime 1000000000

numactl -N 0 -- ./apps/synthetic/target/release/synthetic 10.3.3.3:11211 --config client.config --threads 32 --mode runtime-client --protocol memcached --samples 30 --start_mpps 0 --mpps 3.0 --transport tcp --runtime 1000000000

