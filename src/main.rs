#![allow(unused_variables)]
use std::io;

fn main() -> io::Result<()> {
    // only support f32 for now
    let mut reader = hound::WavReader::open("data/ir_f32.wav").unwrap();
    println!("{}", reader.spec().channels);

    let mut iter = reader.samples::<f32>();

    let length = iter.len();
    println!("{}", length);

    let mut ir_l: Vec<f32> = Vec::with_capacity(iter.len() / 2);
    let mut ir_r: Vec<f32> = Vec::with_capacity(iter.len() / 2);

    for _ in 1..iter.len() {
        if let Some(Ok(s)) = iter.next() {
            ir_l.push(s);
        }
        if let Some(Ok(s)) = iter.next() {
            ir_r.push(s);
        }
    }

    let mut i = 0;
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
    let _sample_rate = client.sample_rate();
    let process = jack::ClosureProcessHandler::new(
        move |_: &jack::Client, ps: &jack::ProcessScope| -> jack::Control {
            // Get output buffer
            let out1 = out1_port.as_mut_slice(ps);
            let out2 = out2_port.as_mut_slice(ps);
            let in2 = in2_port.as_slice(ps);
            let in1 = in1_port.as_slice(ps);
            let stereo_buffer = out1.iter_mut().zip(out2.iter_mut());

            // DSP
            for (l, r) in stereo_buffer {
                if i < length / 2 {
                    *l = ir_l[i];
                    *r = ir_r[i];
                } else {
                    *l = 0.0;
                    *r = 0.0;
                }

                i += 1;

                if i as f32 > length as f32 * 0.55 {
                    i = 0;
                }
            }

            // out1.copy_from_slice(in1);
            // out2.copy_from_slice(in2);
            //
            // Continue as normal
            jack::Control::Continue
        },
    );

    // 4. activate the client
    let _active_client = client.activate_async((), process).unwrap();

    // event loop
    let mut buffer = String::new();
    let stdin = io::stdin();
    stdin.read_line(&mut buffer)?;

    // 6. Optional deactivate. Not required since active_client will deactivate on
    // drop, though explicit deactivate may help you identify errors in
    // deactivate.
    _active_client.deactivate().unwrap();

    Ok(())
}
