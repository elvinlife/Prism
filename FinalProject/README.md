# Final project
Please choose one project for each team. Each project only has a capacity of 3 teams and it is first-come-first-serve.


# 1. Prism 1.0
**Goal:** Improve throughput of the Bitcoin Client by implementing the Prism 1.0 protocol (just the transaction blocks part of the Prism protocol).

**Suggested steps** of this project, which should be used to create checkpoints. 
1. understand the exact prism 1.0 protocol, by reading the relevant literature. 
2. implement based on midterm project: need to define blocks, p2p protocols, state update rules according to prism.
3. Work to get as high throughput as possible, analyze the bottlenecks (disk I/O or network or consensus), compare to baseline (bitcoin) and to theory.
4. Plot performance (throughput) against some fraction of participants conducting active attacks. Identify good attacks to simulate. 

**Final Outcome:** Upgraded Bitcoin client with much higher throughput performance for same security. 

**References:** https://arxiv.org/abs/1909.11261 and https://arxiv.org/abs/1810.08092

# 2. Prism voting chains
**Goal:** Improve latency of the Bitcoin Client by implementing the voting chains of the Prism protocol. 

**Suggested steps** of this project, which should be used to create checkpoints. 
1. understand the exact prism voting chains protocol, by reading the relevant literature. Note that the confirmation rule is different between the two Prism papers (one system and the other theory). We recommend using the confirmation rule in Appendix A of the Prism systems paper.
2. implement based on midterm project: need to define blocks, p2p protocols, confirmation rules and state update rules according to prism.
3. Work to get as low latency as possible, analyze the bottlenecks (disk I/O or network or consensus), compare to baseline (bitcoin) and to theory. 
4. Plot performance (latency) against some fraction of participants conducting active attacks. Identify good attacks to simulate. 

**Final Outcome:** Upgraded Bitcoin client with much higher latency performance for same security. 

**References:** https://arxiv.org/abs/1909.11261 (this is Prism system paper) and https://arxiv.org/abs/1810.08092


# 3. Dandelion
**Goal:** Provide network level anonymity by avoiding linkage of transaction and the IP address where it is broadcast from. 

**Suggested steps** of this project, which should be used to create checkpoints. 
1. Simulate flooding, trickle and diffusion broadcast mechanisms on some caricature p2p network topologies. Implement basic IP-transaction deanonymization algorithms. **References:** https://arxiv.org/pdf/1703.08761.pdf
2. Read Dandelion and Dandelion++ protocols and implement them in the simulation framework above.  
3. Develop an adversarial model to detect sender, compare deanonymization performances of precision and recall. 

**Final Outcome:** Upgraded Bitcoin client with state of the art p2p anonymization protocols. 

**References:** https://arxiv.org/abs/1701.04439 and https://arxiv.org/abs/1805.11060

# 4. Compact blocks
**Goal:** Reduce network bandwidth usage by implementing compact blocks which reduces redundancy in transaction propagation.

**Suggested steps** of this project, which should be used to create check-points. 
1. Modify network and block module to include compact blocks
2. Develop a protocol to efficiently predict the transactions stored by the neighbouring block
3. Analyze the efficiency gain

**Final Outcome:** Upgraded network where a bitcoin node will only receive transaction data once(as compared to twice for normal blocks)

**References:** https://bitcoincore.org/en/2016/06/07/compact-blocks-faq/ , slides: Lecture 5 (slide 13)

# 5. Comparison of smart contract VMs
**Goal:** Comparison of the performance of EVM, MOVE VM and EOS VM. (Pick two out of three).

**Suggested steps** of this project, which should be used to create check-points.
1. Check and compare their structure: VM environment, stack language/instruction, storage. 
2. Learn their programming language. Create similar smart contracts on the platforms. (contracts should be computation-heavy, i/o-heavy, practical/usable)
3. Compare the throughput running on VM.
4. based on this result, guess the cost of instructions.
5. write raw instructions rather than smart contracts
6. Compare the throughput, find which instruction is unexpectedly slower.
7. Analyze the implementation to figure out why some instruction is slower. (Optional)

**Final Outcome:** Systemization of Knowledge on the performance of EVM, MOVE VM and EOS VM and perhaps some improvement proposals for their respective blockchain projects.

**References:** EOS, Ethereum, Move VM

# 6. PoS. longest chain and others
**Goals:** Implement PoS longest chain protocol, and develop a complete full node along the lines of the one created for the midterm project. Show performance under some well known attacks.

**Suggested steps** of this project, which should be used to create check-points.
1. Understand the PoS LC protocol, extract it from paper and its variants.
2. Implement a full node, you can use the codebase used from the Midterm project and change parts affected by the change in consensus protocol.
3. See whether resources are saved, and compare performance with PoW longest chain.
4. Implement some well known attacks like Nothing at Stake and see how it affects the computational load on a honest full node (for example, need to do a lot of state reversals)

**Final Outcome:** A network running PoS blockchain, and an analysis of how NaS attacks can lead to significant computational load.

**References:** https://arxiv.org/abs/1910.02218 and https://eprint.iacr.org/2017/656.pdf

# 7. SPV light clients
**Goals:** Develop light nodes which probes a set of full nodes for transaction verification. The light nodes have limited resources and hence cannot store and validate complete blockchain.

**Suggested steps** of this project, which should be used to create check-points. 
1. Implement Simple Payment verification(SPV) clients
2. Add light client handling in full node
3. Implement some optimizations like NiPoPoW

**Final Outcome:** A light node capable of verifying transaction inclusion(and implicit validity) using limited compute, storage and communication resources.

**References:** SPV, NiPoPoW_explained, NiPoPoW_paper

# 8. Stateless full node
**Goals:** Develop full nodes that can validate State transition without the need to store state, i.e. replace NewState=STF(State,Transaction) with NewState = STF’(Transaction,State Witness). Note that we still need the transaction generator to store witnesses.

**Suggested steps** of this project, which should be used to create check-points. 
1. Implement stateless full nodes
2. Implement archival nodes (without storage incentives)
3. Modify accumulator for efficient state witness
4. Compare efficiency of the modified accumulator w.r.t. Merkle tree based witness (optional)

**Final Outcome:** Stateless client that enables nodes with small storage to join the network, it also enables faster validator rotation in some sharding protocols.

**References:** https://ethresear.ch/t/the-stateless-client-concept/172, https://ethresear.ch/t/accumulators-scalability-of-utxo-blockchains-and-data-availability/176
