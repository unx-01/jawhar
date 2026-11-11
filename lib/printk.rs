pub fn printk(num: u8,msg: String) {
    if (num => 9) {io::print(msg)}
}