//! Local SVG instance expansion. No external references, scripts or entity resolution.
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone)]
struct Element {
    name: String,
    attrs: BTreeMap<String, String>,
    children: Vec<usize>,
}
pub(crate) fn expand(source: &str) -> Result<(String, Vec<String>), String> {
    use xml::reader::{ParserConfig, XmlEvent};
    let mut elements: Vec<Element> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    let mut ids = BTreeMap::new();
    for event in ParserConfig::new()
        .max_attributes(256)
        .max_attribute_length(16 * 1024 * 1024)
        .max_data_length(16 * 1024 * 1024)
        .max_entity_expansion_length(1024)
        .create_reader(source.as_bytes())
    {
        match event.map_err(|e| format!("Invalid SVG XML: {e}"))? {
            XmlEvent::StartElement {
                name, attributes, ..
            } => {
                if elements.len() >= 50000 || stack.len() >= 64 {
                    return Err("SVG exceeds complexity limits".into());
                }
                let attrs: BTreeMap<_, _> = attributes
                    .into_iter()
                    .map(|a| (a.name.local_name, a.value))
                    .collect();
                let index = elements.len();
                if let Some(id) = attrs.get("id") {
                    ids.entry(id.clone()).or_insert(index);
                }
                if let Some(parent) = stack.last() {
                    elements[*parent].children.push(index);
                }
                elements.push(Element {
                    name: name.local_name,
                    attrs,
                    children: Vec::new(),
                });
                stack.push(index);
            }
            XmlEvent::EndElement { .. } => {
                stack.pop().ok_or("Invalid SVG nesting")?;
            }
            _ => {}
        }
    }
    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('"', "&quot;")
    }
    struct Output<'a> {
        elements: &'a [Element],
        ids: &'a BTreeMap<String, usize>,
        xml: String,
        count: usize,
        active: BTreeSet<usize>,
        warnings: BTreeSet<String>,
    }
    impl Output<'_> {
        fn element(&mut self, index: usize, depth: usize, instance: bool) -> Result<(), String> {
            self.count += 1;
            if self.count > 50000 || depth > 64 || self.xml.len() > 16 * 1024 * 1024 {
                return Err("Expanded SVG exceeds complexity limits".into());
            }
            let node = &self.elements[index];
            if node.name == "use" {
                let Some(id) = node.attrs.get("href").and_then(|s| s.strip_prefix('#')) else {
                    self.warnings.insert("External SVG instances were skipped; embed or convert them to paths first.".into());
                    return Ok(());
                };
                let Some(&target) = self.ids.get(id) else {
                    self.warnings
                        .insert(format!("SVG instance #{id} has no local definition."));
                    return Ok(());
                };
                if !self.active.insert(target) {
                    return Err("Recursive SVG instance reference".into());
                }
                self.xml.push_str("<g");
                for (key, value) in &node.attrs {
                    if !matches!(key.as_str(), "href" | "x" | "y" | "width" | "height") {
                        self.xml.push_str(&format!(" {key}=\"{}\"", escape(value)));
                    }
                }
                self.xml.push_str("><g transform=\"translate(");
                let x = node
                    .attrs
                    .get("x")
                    .map_or("0", String::as_str)
                    .parse::<f32>()
                    .map_err(|_| "Invalid SVG instance x")?;
                let y = node
                    .attrs
                    .get("y")
                    .map_or("0", String::as_str)
                    .parse::<f32>()
                    .map_err(|_| "Invalid SVG instance y")?;
                if !x.is_finite() || !y.is_finite() {
                    return Err("Invalid SVG instance position".into());
                }
                self.xml.push_str(&format!("{x} {y})\">"));
                if self.elements[target].name == "symbol"
                    && self.elements[target].attrs.contains_key("viewBox")
                {
                    self.warnings.insert("SVG symbol viewport scaling was approximated; convert instances to paths for exact sizing.".into());
                }
                self.element(target, depth + 2, true)?;
                self.xml.push_str("</g></g>");
                self.active.remove(&target);
                return Ok(());
            }
            let name = if instance && node.name == "symbol" {
                "g"
            } else {
                node.name.as_str()
            };
            self.xml.push_str(&format!("<{name}"));
            for (key, value) in &node.attrs {
                self.xml.push_str(&format!(" {key}=\"{}\"", escape(value)));
            }
            self.xml.push('>');
            for &child in &node.children {
                self.element(child, depth + 1, false)?;
            }
            self.xml.push_str(&format!("</{name}>"));
            Ok(())
        }
    }
    if elements.is_empty() {
        return Err("Empty SVG".into());
    }
    let mut output = Output {
        elements: &elements,
        ids: &ids,
        xml: String::new(),
        count: 0,
        active: BTreeSet::new(),
        warnings: BTreeSet::new(),
    };
    output.element(0, 0, false)?;
    if output.xml.len() > 16 * 1024 * 1024 {
        return Err("Expanded SVG exceeds 16 MiB".into());
    }
    Ok((output.xml, output.warnings.into_iter().collect()))
}
#[cfg(test)]
mod tests {
    #[test]
    fn local_instances_keep_inherited_colors_and_transform_and_reject_cycles() {
        let imported = crate::svg_import::parse(r##"<svg><defs><path id="line" d="M0 0L20 0" fill="none"/></defs><use href="#line" x="10" y="15" stroke="red" transform="scale(2)"/></svg>"##).unwrap();
        assert_eq!(imported.marks.len(), 1);
        let bounds = crate::vector::control_bounds(&imported.marks[0]);
        assert!((bounds.left() - 20.).abs() < 0.01 && (bounds.top() - 30.).abs() < 0.01);
        assert_eq!(imported.marks[0].color, [255, 0, 0, 255]);
        assert!(
            crate::svg_import::parse(
                r##"<svg><defs><g id="a"><use href="#a"/></g></defs><use href="#a"/></svg>"##
            )
            .is_err()
        );
        let external =
            super::expand(r#"<svg><use href="https://invalid.example/a.svg#x"/></svg>"#).unwrap();
        assert_eq!(external.1.len(), 1);
    }
}
