use std::time::Instant;

fn main() {
    for layers in [10, 50, 60, 100] {
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
    code.push_str("interface IA {\n    fn base_action();\n}\n\n");
    for i in 0..layers {
        if i == 0 {
            code.push_str(&format!("class C{} implements IA {{\n    public void base_action() {{\n    }}\n}}\n\n", i));
        } else {
            code.push_str(&format!("class C{} extends C{} {{\n    public void method_{}() {{\n    }}\n}}\n\n", i, i-1, i));
        }
    }
    code
}
