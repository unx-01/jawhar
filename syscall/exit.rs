#![no_std]

pub fn exit(status: u32){
        if status == 0 {return 0}
        if status == 5 {return 5}
        if status == 10 {return 10}
        if status == 20 {return 20}        
}