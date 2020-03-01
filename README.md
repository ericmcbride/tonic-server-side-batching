### Tonic Server Side Batching

Sometimes you need to batch some gRPC requests to do some computations somewhere.  Here is a vanilla example of how to do that.

### How to run examples
- `cargo run --bin batch-server`
- `cargo run --bin batch-client `

### Thought Process
I left the batch scheduler pretty empty on purpose.  Ideally you'd declare a vector, and you would push onto the vector until you hit a threshold, then do some action, flush, repeat.  The batch scheduler runs in the background, allowing the gRPC server to communicate to it.
