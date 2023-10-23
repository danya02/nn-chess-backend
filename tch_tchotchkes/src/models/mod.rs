pub mod eval_narrow;
pub mod eval_superwide;
pub mod eval_wide;
pub mod move_rnn;

use tch::nn::{self, Module};

fn net(vs: &nn::Path, input_shape: i64, hidden_shapes: &[i64], output_shape: i64) -> impl Module {
    let net = nn::seq();

    // Input layer -> hidden layer 1
    let mut net = net.add(nn::linear(
        vs / "layer1",
        input_shape,
        hidden_shapes[0],
        Default::default(),
    ));

    for n in 0..hidden_shapes.len() - 1 {
        net = net.add_fn(|x| x.relu());
        net = net.add(nn::linear(
            vs.sub(&format!("hidden{n}")),
            hidden_shapes[n],
            hidden_shapes[n + 1],
            Default::default(),
        ))
    }
    // Now add the final hidden layer, which outputs to the output shape
    let net = net.add(nn::linear(
        vs.sub(&format!("hidden{}", hidden_shapes.len() - 1)),
        *hidden_shapes.last().unwrap(),
        output_shape,
        Default::default(),
    ));
    // Do not apply any function on last layer
    net
}
