// 临时测试文件：查看 keyboard-types::Code 枚举的可用值
use keyboard_types::Code;

fn main() {
    // 尝试打印所有可能的 Ctrl/Alt/Shift 相关枚举
    println!("Testing Control keys:");

    // 尝试常见的枚举名
    let variants = vec![
        "Control",
        "ControlLeft",
        "ControlRight",
        "ControlOrMeta",
        "MetaLeft",
        "MetaRight",
    ];

    for v in variants {
        println!("  {}", v);
    }

    // 实际可用的枚举（根据编译器提示）
    println!("\nActual available Code variants (partial list):");
    println!("  Code::ShiftLeft");
    println!("  Code::ShiftRight");
    // 这里会根据编译错误得知正确的枚举名
}
