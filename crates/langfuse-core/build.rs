use std::{env, fs, path::Path};

fn main() {
    let src = "openapi.yml";
    println!("cargo:rerun-if-changed={src}");

    // Read YAML spec, pre-process, then parse into OpenAPI struct
    let yaml_content =
        fs::read_to_string(src).unwrap_or_else(|e| panic!("Failed to read {src}: {e}"));

    // Parse as generic Value first for pre-processing
    let mut value: serde_json::Value = serde_yaml::from_str(&yaml_content)
        .unwrap_or_else(|e| panic!("Failed to parse {src} as YAML: {e}"));

    // Pre-process: Remove empty schemas from error responses (4xx/5xx).
    // progenitor 0.9 panics when response types differ (e.g., 200 has a schema
    // but 400 has `schema: {}`) because it can't yet create enum response types.
    // Fix: strip the content from error responses so they become opaque errors.
    if let Some(paths) = value.get_mut("paths")
        && let Some(paths_obj) = paths.as_object_mut()
    {
        for (_path, methods) in paths_obj.iter_mut() {
            if let Some(methods_obj) = methods.as_object_mut() {
                for (_method, details) in methods_obj.iter_mut() {
                    if let Some(responses) = details.get_mut("responses")
                        && let Some(responses_obj) = responses.as_object_mut()
                    {
                        for (code, resp) in responses_obj.iter_mut() {
                            // Only strip non-2xx responses with empty schemas
                            if !code.starts_with('2')
                                && let Some(content) = resp.get("content")
                            {
                                let all_empty = content
                                    .as_object()
                                    .map(|co| {
                                        co.values().all(|ct| {
                                            ct.get("schema")
                                                .and_then(|s| s.as_object())
                                                .map(|o| o.is_empty())
                                                .unwrap_or(true)
                                        })
                                    })
                                    .unwrap_or(false);
                                if all_empty {
                                    resp.as_object_mut().unwrap().remove("content");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    sanitize_openapi_docs(&mut value);

    // Convert to OpenAPI struct
    let spec_str = serde_json::to_string(&value).unwrap();
    let spec: openapiv3::OpenAPI = serde_json::from_str(&spec_str)
        .unwrap_or_else(|e| panic!("Failed to parse pre-processed spec: {e}"));

    // Configure generator
    let mut settings = progenitor::GenerationSettings::default();
    settings
        .with_interface(progenitor::InterfaceStyle::Builder)
        .with_tag(progenitor::TagStyle::Merged);

    let mut generator = progenitor::Generator::new(&settings);

    // Generate the client code
    let tokens = generator
        .generate_tokens(&spec)
        .unwrap_or_else(|e| panic!("Failed to generate API client: {e}"));
    let ast = syn::parse2(tokens).unwrap_or_else(|e| panic!("Failed to parse generated code: {e}"));
    let content = prettyplease::unparse(&ast);

    // Write to OUT_DIR
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_file = Path::new(&out_dir).join("codegen.rs");
    fs::write(&out_file, content)
        .unwrap_or_else(|e| panic!("Failed to write {}: {e}", out_file.display()));
}

fn sanitize_openapi_docs(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if matches!(key.as_str(), "description" | "summary")
                    && let serde_json::Value::String(text) = child
                {
                    *text = sanitize_doc_text(text);
                }
                sanitize_openapi_docs(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                sanitize_openapi_docs(item);
            }
        }
        _ => {}
    }
}

fn sanitize_doc_text(text: &str) -> String {
    let escaped_brackets = escape_non_link_brackets(text);
    let escaped_placeholders = escape_angle_placeholders(&escaped_brackets);
    wrap_bare_urls(&escaped_placeholders)
}

fn escape_non_link_brackets(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '['
            && let Some(close_index) = chars[index + 1..].iter().position(|&ch| ch == ']')
        {
            let close_index = index + 1 + close_index;
            if chars.get(close_index + 1) != Some(&'(') {
                result.push_str("\\[");
                for ch in &chars[index + 1..close_index] {
                    result.push(*ch);
                }
                result.push_str("\\]");
                index = close_index + 1;
                continue;
            }
        }

        result.push(chars[index]);
        index += 1;
    }

    result
}

fn escape_angle_placeholders(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        if chars[index] == '<'
            && let Some(close_index) = chars[index + 1..].iter().position(|&ch| ch == '>')
        {
            let close_index = index + 1 + close_index;
            let inner: String = chars[index + 1..close_index].iter().collect();
            if !inner.is_empty()
                && !inner.contains(char::is_whitespace)
                && !inner.starts_with("http://")
                && !inner.starts_with("https://")
            {
                result.push_str("&lt;");
                result.push_str(&inner);
                result.push_str("&gt;");
                index = close_index + 1;
                continue;
            }
        }

        result.push(chars[index]);
        index += 1;
    }

    result
}

fn wrap_bare_urls(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut index = 0;

    while index < chars.len() {
        let remainder: String = chars[index..].iter().collect();
        if remainder.starts_with("http://") || remainder.starts_with("https://") {
            let previous = index
                .checked_sub(1)
                .and_then(|prev| chars.get(prev))
                .copied();
            let mut end = index;
            while let Some(ch) = chars.get(end) {
                if ch.is_whitespace() {
                    break;
                }
                end += 1;
            }

            let mut url_end = end;
            while url_end > index && matches!(chars[url_end - 1], '.' | ',' | ';' | ':') {
                url_end -= 1;
            }

            let url: String = chars[index..url_end].iter().collect();
            if matches!(previous, Some('(' | '<')) {
                result.push_str(&url);
            } else {
                result.push('<');
                result.push_str(&url);
                result.push('>');
            }

            for ch in &chars[url_end..end] {
                result.push(*ch);
            }
            index = end;
            continue;
        }

        result.push(chars[index]);
        index += 1;
    }

    result
}
