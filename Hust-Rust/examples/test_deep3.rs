use std::time::Instant;

fn main() {
    for layers in [10, 50, 60, 100] {
        let code = generate_test_code(layers);
        let start = Instant::now();
        
        // Simulate the parent chain walking logic
        let re = regex::Regex::new(r"(?m)^\s*class\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*(?:extends\s+(\w+))?\s*(?:implements\s+([\w,\s]+))?\s*\{([\s\S]*?)^\}").unwrap();
        let _ = re.replace_all(&code, |caps: &regex::Captures| {
            let class_name = &caps[1];
            let parent_class = caps.get(2).map(|m| m.as_str());
            
            // Walk up parent chain - this is the bottleneck!
            let mut current_parent = parent_class.map(|s| s.to_string());
            let mut parent_chain: Vec<String> = Vec::new();
            while let Some(ref parent) = current_parent {
                parent_chain.push(parent.clone());
                // NEW REGEX CREATED HERE for each parent!
                let parent_re = regex::Regex::new(&format!(
                    r"(?m)^\s*class\s+{}\s*(?:extends\s+(\w+))?\s*(?:implements\s+([\w,\s]+))?\s*\{{",
                    parent
                )).unwrap();
                
                if let Some(pcaps) = parent_re.captures(&code) {
                    current_parent = pcaps.get(1).map(|m| m.as_str().to_string());
                } else {
                    current_parent = None;
                }
            }
            
            format!("// {} has {} parents\n", class_name, parent_chain.len())
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
