use anyhow::Result;
use tch::{nn, nn::Module, nn::OptimizerConfig, Device};

const IMAGE_DIM: i64 = 784;
const HIDDEN_NODES: i64 = 128;
const LABELS: i64 = 10;

fn net(vs: &nn::Path) -> impl Module {
    nn::seq()
        .add(nn::linear(
            vs / "layer1",
            IMAGE_DIM,
            HIDDEN_NODES,
            Default::default(),
        ))
        .add_fn(|xs| xs.relu())
        .add(nn::linear(vs, HIDDEN_NODES, LABELS, Default::default()))
}

pub fn run() -> Result<()> {
    let m = tch::vision::mnist::load_dir("mnist-data")?;
    let vs = nn::VarStore::new(Device::Cpu);
    let net = net(&vs.root());
    let mut opt = nn::Adam::default().build(&vs, 1e-3)?;
    for epoch in 1..200 {
        dbg!(m.train_images.size());
        let prediction = net.forward(&m.train_images);
        dbg!(prediction.size());
        let loss = prediction.cross_entropy_for_logits(&m.train_labels);
        opt.backward_step(&loss);
        let test_accuracy = net
            .forward(&m.test_images)
            .accuracy_for_logits(&m.test_labels);
        dbg!(test_accuracy.size());
        println!(
            "epoch: {:4} train loss: {:8.5} test acc: {:5.2}%",
            epoch,
            f64::try_from(&loss)?,
            100. * f64::try_from(&test_accuracy)?,
        );
    }
    Ok(())
}
