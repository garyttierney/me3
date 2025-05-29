pub fn print_section(indent: usize, header: &str, values: &[(&str, String)]) {
    let width = values.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

    println!("{:indent$}● {header}", "");

    for (key, value) in values {
        println!("  {:indent$} {key:>width$}: {value}", "");
    }

    if !values.is_empty() {
        println!();
    }
}

#[derive(Default)]
pub struct OutputBuilder {
    indent: usize,
    header: Option<String>,
    properties: Vec<(String, String)>,
    children: Vec<String>,
}

impl OutputBuilder {
    pub fn new<H: ToString>(header: H) -> Self {
        Self {
            header: Some(header.to_string()),
            ..Default::default()
        }
    }

    pub fn build(self) -> String {
        let OutputBuilder {
            indent,
            header,
            mut properties,
            children,
        } = self;

        let mut output = String::new();
        let width = properties.iter().map(|(k, _)| k.len()).max().unwrap_or(0);

        if let Some(header) = header {
            output.push_str(&format!("{:indent$}● {header}\n", ""));
        }

        properties.sort_by_key(|(k, _)| k.len());

        for (key, value) in properties.iter().rev() {
            output.push_str(&format!(
                "{:indent$}{key:>width$}: {value}\n",
                "",
                indent = indent + 4
            ));
        }

        for child in children {
            output.push_str(&child);
        }

        output
    }

    pub fn property<K: ToString, V: ToString>(&mut self, key: K, value: V) {
        self.properties.push((key.to_string(), value.to_string()));
    }

    pub fn indent(&mut self, width: usize) {
        self.indent += width;
    }

    pub fn section<H: Into<String>>(
        &mut self,
        header: H,
        builder: impl FnOnce(&mut OutputBuilder),
    ) {
        let mut section_builder = OutputBuilder {
            indent: self.indent,
            header: Some(header.into()),
            properties: vec![],
            children: vec![],
        };

        (builder)(&mut section_builder);

        self.children.push(section_builder.build());
    }
}
