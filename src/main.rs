#![feature(mpsc_select, get_type_id)]
#![allow(dead_code, unused_imports)]

use std::sync::mpsc::{SyncSender, Receiver, SendError, RecvError, Handle, Select, sync_channel};
use std::collections::HashMap;
use std::iter::FromIterator;
use std::thread;
use std::any::{Any, TypeId};
use std::marker::Sized;
use std::rc::Rc;

fn main() {
    channel_test();
    typing_test();
}

fn print_type_id(x: Box<Any>) {
    println!("{:?}", x.get_type_id());
    match x.downcast_ref::<SyncSender<String>>() {
        Some(s) => {
            println!("it's a string channel!");
            for i in 0..3 {
                println!("sending ping {}", i);
                s.send(format!("ping {}", i)).unwrap();
            }
        },
        None => println!("it's not a string :(")
    }
    println!("sender closing channel");
}

fn typing_test() {
    println!("type test");
    let (tx, rx) = sync_channel::<String>(0);
    let t = thread::spawn(move|| {
        for i in rx {
            println!("received {}", i);
        }
        println!("receiver sees channel closed");
    });
    print_type_id(Box::new(tx));
    t.join().unwrap();
}

const N: i32 = 3;
const M: i32 = 3;

fn channel_test() {
    if N == 0 {
        return;
    }

    println!("making {} channels", N);
    let (txs, mut rxs): (Vec<SyncSender<String>>, Vec<Receiver<String>>) =
        (0..N).map(|_| {sync_channel(0)}).unzip();

    println!("launching threads");
    let threads: Vec<thread::JoinHandle<()>> =
        txs.into_iter().enumerate().map(|p| {
            thread::spawn(move || {
                (0..M).map(|i| {
                    println!("sending '{}'", format!("{} {}", p.0, i));
                    match p.1.send(format!("{} {}", p.0, i)) {
                        Result::Ok(()) => (),
                        Result::Err(SendError(e)) => {
                            println!("{} ERROR {}", p.0, e);
                        }
                    }
                }).count();
            })
        }).collect();

    loop {
        let todelete: usize;
        {
            let sel = Select::new();
            let mut handles: HashMap<usize, (Box<Handle<String>>, usize)> =
                HashMap::from_iter(rxs.iter().enumerate().map(|p| {
                    let mut h = Box::new(sel.handle(p.1));
                    unsafe{h.add();}
                    (h.id(), (h, p.0))
                }));
            loop {
                let id = sel.wait();
                match handles.get_mut(&sel.wait()) {
                    Some(p) => {
                        match p.0.recv() {
                            Result::Ok(v) => {
                                println!("received '{}'", v);
                            },
                            Result::Err(RecvError) => {
                                todelete = p.1;
                                break;
                            }
                        }
                    },
                    None => println!("UNKNOWN HANDLE {}", id)
                }
            }
        }
        rxs.swap_remove(todelete);
        if rxs.len() <= 0 {break}
    }

    println!("joining threads");
    threads.into_iter().map(|thread| {thread.join().unwrap();}).count();
}
