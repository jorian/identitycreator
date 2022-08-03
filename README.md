# Identity Creator

This app lets you create Verus identities in an automated fashion.

## How to use

See the `src/main.rs` file for an example of how to use this.

## Dependencies

This library depends on the `vrsc-rpc` library, which is a RPC wrapper for the Verus daemon. It is required to have a native Verus instance running.

## Todo

- make use of zmq to wait for new block, instead of polling
- make async
