use hust_rust::translator::{Translator, TranspileOptions};
use std::time::Instant;

fn main() {
    for layers in [10, 50, 60, 100, 200, 255] {
        let code = generate_test_code(layers);
        let translator = Translator::new(TranspileOptions::default());
        
        let start = Instant::now();
        let result = translator.transpile(&code);
        let elapsed = start.elapsed();
        
        match result {
            Ok(_) => println!("Layers: {:3}, Time: {:?} ✅", layers, elapsed),
            Err(e) => println!("Layers: {:3}, Time: {:?} ❌ Error: {}", layers, elapsed, e),
        }
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
