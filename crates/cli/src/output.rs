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

        properties.sort_by_key(|(k, _)| std::cmp::Reverse(k.len()));

        for (key, value) in properties.iter() {
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

#[cfg(test)]
mod tests {
    use crate::output::OutputBuilder;

    #[test]
    fn test_output_builder_build_header_only() {
        let builder = OutputBuilder::new("Header");
        assert_eq!(
            builder.build(),
            r#"● Header
"#
        );
    }

    #[test]
    fn test_output_builder_build_properties_only() {
        let mut builder = OutputBuilder::default();
        builder.property("key", "value");
        builder.property("longer_key", "another value");
        let expected = r#"
    longer_key: another value
           key: value
"#;
        assert_eq!(builder.build(), expected.trim_start_matches('\n'));
    }

    #[test]
    fn test_output_builder_build_header_and_properties() {
        let mut builder = OutputBuilder::new("Header");
        builder.property("key", "value");
        let expected = r#"
● Header
    key: value
"#;
        assert_eq!(builder.build(), expected.trim_start_matches('\n'));
    }

    #[test]
    fn test_output_builder_build_with_indentation() {
        let mut builder = OutputBuilder::new("Header");
        builder.indent(2);
        builder.property("key", "value");
        let expected = r#"
  ● Header
      key: value
"#;
        assert_eq!(builder.build(), expected.trim_start_matches('\n'));
    }

    #[test]
    fn test_output_builder_section() {
        let mut builder = OutputBuilder::new("Main Header");
        builder.property("main_key", "main_value");
        builder.section("Sub Section", |sub_builder| {
            sub_builder.property("sub_key", "sub_value");
        });
        let expected = r#"● Main Header
    main_key: main_value
● Sub Section
    sub_key: sub_value
"#;
        assert_eq!(builder.build(), expected.trim_start_matches('\n'));
    }

    #[test]
    fn test_output_builder_nested_sections_with_indent() {
        let mut builder = OutputBuilder::new("Level 0");
        builder.indent(0);
        builder.property("level0_prop", "val0");
        builder.section("Level 1", |b1| {
            b1.indent(2);
            b1.property("level1_prop", "val1");
            b1.section("Level 2", |b2| {
                b2.indent(2);
                b2.property("level2_prop", "val2");
            });
        });

        let expected = r#"
● Level 0
    level0_prop: val0
  ● Level 1
      level1_prop: val1
    ● Level 2
        level2_prop: val2
"#;
        assert_eq!(builder.build(), expected.trim_start_matches('\n'));
    }
}
