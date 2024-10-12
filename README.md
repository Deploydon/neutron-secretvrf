# Neutron Secret-VRF Example

This is a modified version of the Secret VRF sample contract to support Neutron min IBC fees.

Original Contract: https://github.com/writersblockchain/ibchooks-secretVRF/tree/master/consumer-side

Secret Documentation: https://docs.scrt.network/secret-network-documentation/confidential-computing-layer/ibc/usecases/secret-vrf-for-ibc-with-ibc-hooks

Given the current min IBC fees, it costs about 0.10~ $NTRN to request a random number. Ensure the contract holds some $NTRN before executing. 


Contract will generate a job ID automatically, query the job ID to get the generated number. The current range is from 1 to 1000, set your range with MIN_NUMBER and MAX_NUMBER.

