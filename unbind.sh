sudo dpdk-devbind --bind=ixgbe 02:00.1
sudo ip link set dev ens2f1 up
sudo ip a add 10.3.3.1/24 dev ens2f1

