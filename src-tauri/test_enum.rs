// 测试 keyboard_types::Code 枚举的可用变体
extern crate keyboard_types;
use keyboard_types::Code;

fn main() {
    // 尝试使用不同的枚举名称
    let _test1 = Code::ShiftLeft;
    let _test2 = Code::ShiftRight;
    
    // 尝试 Ctrl 的不同变体
    // let _test3 = Code::ControlLeft;   // 可能不存在
    // let _test4 = Code::Control;        // 尝试这个
    // let _test5 = Code::ControlOrMeta;  // 或者这个
    
    println!("Test compilation successful");
}
