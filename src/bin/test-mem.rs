#![no_main]
#![no_std]
#![feature(custom_test_frameworks)]
#![test_runner(test_kartoffel::runner)]
#![feature(iter_next_chunk)]

use alloc::boxed::Box;
use async_kartoffel::println;

extern crate alloc;

#[unsafe(no_mangle)]
fn main() {
    // stack overflow error:
    // firmware crashed: invalid access on 0x000ffffc+4

    // largest possible stack array
    // let mut x = [255u8; 3833];
    // println!("{}", core::mem::size_of_val(&x));
    // loop {
    //     for i in &mut x {
    //         *i = i.wrapping_add(1);
    //     }
    //     println!("{}", x.first().unwrap());
    // }

    // panic
    //
    // largest possible heap array
    // let mut y = Box::new([255u8; 119180]);
    // println!("{}", core::mem::size_of_val(&y));
    // loop {
    //     for i in y.as_mut_slice() {
    //         *i = i.wrapping_add(1);
    //     }
    //     println!("{}", y.first().unwrap());
    // }

    // largest possible combination array
    let mut x = [255u8; 3828];
    let mut y = Box::new([255u8; 118808]);
    // let mut x = [255u8; 10];
    // let mut y = Box::new([255u8; 10]);
    println!("{}", core::mem::size_of_val(&x));
    println!("{}", core::mem::size_of_val(&y));
    loop {
        for i in &mut x {
            *i = i.wrapping_add(1);
        }
        for i in y.as_mut_slice() {
            *i = i.wrapping_add(1);
        }
        println!("{}", x.first().unwrap());
        println!("{}", y.first().unwrap());
    }
}
