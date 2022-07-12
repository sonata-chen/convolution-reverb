mod convolution;
mod fft;
mod plugin;
mod ui;
mod app;

// #![allow(unused_variables)]
// #![allow(unused_imports)]
// #![allow(dead_code)]

use std::io;
use crate::app::app;

use crate::plugin::AudioPlugin;


fn main() -> io::Result<()> {

    // 1. open a client
    let (client, _status) =
        jack::Client::new("convolution", jack::ClientOptions::NO_START_SERVER).unwrap();

    // 2. register port
    let mut out1_port = client
        .register_port("convolution_out1", jack::AudioOut::default())
        .unwrap();
    let mut out2_port = client
        .register_port("convolution_out2", jack::AudioOut::default())
        .unwrap();
    let in1_port = client
        .register_port("convolution_in1", jack::AudioIn::default())
        .unwrap();
    let in2_port = client
        .register_port("convolution_in2", jack::AudioIn::default())
        .unwrap();

    // 3. define process callback handler
    let sample_rate = client.sample_rate();
    println!("jack sample rate: {}", sample_rate);

    let (tx, mut plugin) = AudioPlugin::new();
    plugin.prepare_to_play(sample_rate, client.buffer_size() as usize);

    let process = jack::ClosureProcessHandler::new(
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            // Get output buffer
            let out1 = out1_port.as_mut_slice(ps);
            let out2 = out2_port.as_mut_slice(ps);
            let in2 = in2_port.as_slice(ps);
            let in1 = in1_port.as_slice(ps);
            let input = [in1, in2];
            let mut output = [out1, out2];

            plugin.process(&input, &mut output);

            // Continue as normal
            jack::Control::Continue
        },
    );

    // 4. activate the client
    let _active_client = client.activate_async((), process).unwrap();

    // open GUI
    app(tx);

    // 6. Optional deactivate. Not required since active_client will deactivate on
    // drop, though explicit deactivate may help you identify errors in
    // deactivate.
    _active_client.deactivate().unwrap();

    Ok(())
}
