use std::time::Instant;

fn main() {
    for layers in [10, 50, 60, 100, 200] {
        let code = generate_test_code(layers);
        let start = Instant::now();
        
        let re = regex::Regex::new(r"(?m)^\s*class\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:extends\s+(\w+))?\s*(?:implements\s+([\w,\s]+))?\s*\{([\s\S]*?)^\}").unwrap();
        let _ = re.replace_all(&code, |caps: &regex::Captures| {
            format!("// matched: {}", &caps[1])
        });
        
        let elapsed = start.elapsed();
        println!("Layers: {:3}, Time: {:?}", layers, elapsed);
    }
}

fn generate_test_code(layers: usize) -> String {
    let mut code = String::new();
    // Multiple interfaces
    for i in 0..5 {
        code.push_str(&format!("interface I{} {{\n    fn method_{}();\n    fn method_{}_b();\n}}\n\n", i, i, i));
    }
    
    for i in 0..layers {
        if i == 0 {
            code.push_str(&format!("class C{} implements I0 {{\n    public void method_0() {{\n        println!(\"hello\");\n    }}\n    public void method_0_b() {{\n        println!(\"world\");\n    }}\n}}\n\n", i));
        } else {
            // Each class extends previous and implements more interfaces
            let iface = if i % 5 == 0 { format!(" implements I{}", i % 5) } else { String::new() };
            code.push_str(&format!("class C{} extends C{}{} {{\n    public void method_{}() {{\n        if true {{\n            println!(\"test\");\n        }}\n    }}\n    public void method_{}_b() {{\n        for i in 0..10 {{\n            println!(\"{{}}\", i);\n        }}\n    }}\n    public int field_{};\n}}\n\n", i, i-1, iface, i, i, i));
        }
    }
    code
}
