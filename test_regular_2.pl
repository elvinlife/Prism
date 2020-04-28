#!/usr/bin/perl
use warnings;
use strict;
use threads;

my $cmd1 = "cargo run -- -vvv --p2p 127.0.0.1:6000 --api 127.0.0.1:7000 2> ./log/reg2_64.log";
my $cmd2 = "cargo run -- -vvv --p2p 127.0.0.1:6001 --api 127.0.0.1:7001 -c 127.0.0.1:6000";
my $cmd3 = "cargo run -- -vvv --p2p 127.0.0.1:6002 --api 127.0.0.1:7002 -c 127.0.0.1:6001";
my $cmd4 = "cargo run -- -vvv --p2p 127.0.0.1:6003 --api 127.0.0.1:7003 -c 127.0.0.1:6002";
my $cmd5 = "cargo run -- -vvv --p2p 127.0.0.1:6004 --api 127.0.0.1:7004 -c 127.0.0.1:6003";
my $cmd6 = "cargo run -- -vvv --p2p 127.0.0.1:6005 --api 127.0.0.1:7005 -c 127.0.0.1:6004";
my $cmd7 = "cargo run -- -vvv --p2p 127.0.0.1:6006 --api 127.0.0.1:7006 -c 127.0.0.1:6005";
my $cmd8 = "cargo run -- -vvv --p2p 127.0.0.1:6007 --api 127.0.0.1:7007 -c 127.0.0.1:6006 127.0.0.1:7000";
my $cmd_url = join "curl -L http://127.0.0.1:7000/miner/start?lambda=10000 &",
    "curl -L http://127.0.0.1:7001/miner/start?lambda=10000 & \n",
    "curl -L http://127.0.0.1:7002/miner/start?lambda=10000 & \n",
    "curl -L http://127.0.0.1:7003/miner/start?lambda=10000 & \n",
    "curl -L http://127.0.0.1:7004/miner/start?lambda=10000 & \n",
    "curl -L http://127.0.0.1:7005/miner/start?lambda=10000 & \n",
    "curl -L http://127.0.0.1:7006/miner/start?lambda=10000 & \n",
    "curl -L http://127.0.0.1:7007/miner/start?lambda=10000 & ";
my @cmd_array = ($cmd1, $cmd2, $cmd3, $cmd4, $cmd5, $cmd6, $cmd7, $cmd8);
my @threads;

foreach my $cmd (@cmd_array) {
    push @threads, threads->create(sub {system($cmd)});
}

sleep(1);

my $t_control = threads->create(sub {system($cmd_url)});
$t_control->join();

sleep(60);

system("killall bitcoin");
