use std::fs::File;
use std::io::{Read, Write, Cursor};
use std::collections::VecDeque;

use flate2::read::ZlibDecoder;
use zopfli::{Format, compress};
use chrono::Local;
use std::sync::mpsc;
use std::thread;

const NUM_THREAD : usize = 2;

fn ms_to_string(time : i64) -> String {
    if time < 1000 {
        format!("{}ms", time)
    } else if time < 10000 {
        format!("{}.{}s", time / 1000, time % 1000)
    } else {
        format!("{}s", time / 1000)
    }
}

fn main() {
    let init = Local::now();

    let mut source = File::open("old.hist").unwrap();
    let mut output = File::create("new.hist").unwrap();
    let mut lengths : Vec<u32> = Vec::new();
    let mut buf = [0u8; 4];
    loop {
        source.read(&mut buf).unwrap();
        let num = u32::from_le_bytes(buf.clone());
        if num == 0 {
            break;
        }
        lengths.push(num);
    }

    let orig_size : u32 = lengths.iter().sum::<u32>() + (1 + lengths.len() as u32) * 4;

    let num_lines = lengths.len();

    let mut process_line = |queue : &mut VecDeque<mpsc::Receiver<Vec<u8>>>, len : u32, c| {
        let mut decoder = ZlibDecoder::new(Read::by_ref(&mut source).take(len as u64));
        let mut inflated : Vec<u8> = Vec::new();
        decoder.read_to_end(&mut inflated).unwrap();

        let (sender, receiver) = mpsc::channel();
        thread::spawn(move || {
            let start = Local::now();

            let mut cursor = Cursor::new(Vec::new());
            compress(&Default::default(), &Format::Zlib, &inflated, &mut cursor).unwrap();

            let time_diff = (Local::now() - start).num_milliseconds();
            println!("Encoded {}/{} in {}", c, num_lines, ms_to_string(time_diff));
            sender.send(cursor.into_inner()).unwrap();
        });
        queue.push_back(receiver);
    };


    let mut outputs : Vec<Vec<u8>> = Vec::with_capacity(lengths.len());
    let mut res_queue : VecDeque<mpsc::Receiver<Vec<u8>>> = VecDeque::new();
    let mut counter = 1;
    for len in lengths.iter() {
        println!("{}/{}: {} bytes", counter, num_lines, len);

        process_line(&mut res_queue, *len, counter);

        if counter == NUM_THREAD {
            break;
        }
        counter += 1;
    }

    while let Some(sender) = res_queue.pop_front() {
        outputs.push(sender.recv().unwrap());
        if counter < lengths.len() {
            process_line(&mut res_queue, lengths[counter], counter + 1);
            counter += 1;
        }
    }

    let mut final_size = (1 + outputs.len() as u32) * 4;
    for line in outputs.iter() {
        final_size += line.len() as u32;
        output.write(&(line.len() as u32).to_le_bytes()).unwrap();
    }
    output.write(&0u32.to_le_bytes()).unwrap();
    for line in outputs {
        output.write(&line).unwrap();
    }

    println!("Original size: {}, New size: {}, Compression: {}%", orig_size, final_size, (100.0 * final_size as f64 / orig_size as f64).round());

    let time_diff = (Local::now() - init).num_milliseconds();
    println!("Total time: {}", ms_to_string(time_diff));
}
