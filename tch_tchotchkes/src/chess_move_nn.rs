use anyhow::Result;
use tch::{nn, nn::Module, nn::OptimizerConfig, vision::dataset::Dataset, Device};

const BOARD_SIZE: i64 = 64;
const BOARD_SQUARE_NUM_OPTS: i64 = 2 * 6;
const INPUT_SHAPE: i64 = BOARD_SIZE * BOARD_SQUARE_NUM_OPTS;
const HIDDEN_SHAPE: &[i64] = &[1536, 768, 256];
const OUTPUT_SHAPE: i64 = 2 * BOARD_SIZE + 1;

/// Defines the shape for a neural network whose output is a chess move
fn net(vs: &nn::Path) -> impl Module {
    let net = nn::seq();

    // Input layer -> hidden layer 1
    let mut net = net.add(nn::linear(
        vs / "chessboard",
        INPUT_SHAPE,
        HIDDEN_SHAPE[0],
        Default::default(),
    ));

    for n in 0..HIDDEN_SHAPE.len() - 1 {
        net = net.add_fn(|x| x.relu());
        net = net.add(nn::linear(
            vs.sub(&format!("hidden{n}")),
            HIDDEN_SHAPE[n],
            HIDDEN_SHAPE[n + 1],
            Default::default(),
        ))
    }
    // Now add the final hidden layer, which outputs to the output shape
    let net = net.add(nn::linear(
        vs.sub(&format!("hidden{}", HIDDEN_SHAPE.len() - 1)),
        *HIDDEN_SHAPE.last().unwrap(),
        OUTPUT_SHAPE,
        Default::default(),
    ));
    // Do not apply any function on last layer
    net
}

pub fn run_training() -> Result<()> {
    let data: Dataset = todo!();
    let vs = nn::VarStore::new(Device::Cpu);
    let net = net(&vs.root());
    let mut opt = nn::Adam::default().build(&vs, 0.001)?;
    for epoch in 1..200 {
        for (input, output) in data.train_iter(64).shuffle().to_device(vs.device()) {
            let loss = net.forward(&input).cross_entropy_for_logits(&output);
            opt.backward_step(&loss);
        }
        for (input, output) in data.test_iter(64).shuffle().to_device(vs.device()) {
            let test_accuracy = net.forward(&input).accuracy_for_logits(&output);
            println!(
                "epoch: {:4} train loss: {:8.5} test acc: {:5.2}%",
                epoch,
                0,
                100. * f64::try_from(&test_accuracy)?,
            );
        }
    }

    Ok(())
}
