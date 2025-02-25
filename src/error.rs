use std::borrow::Cow;

use naga_oil::compose::{Composer, ComposerError, ComposerErrorInner};
use regex::Regex;

lazy_static::lazy_static! {
    static ref UNDECORATE_REGEX: Regex = Regex::new("(.+)X_naga_oil_mod_X([A-Z0-9]*)X").unwrap();
}

pub(crate) fn decompose_mangled_name(source: &str) -> Option<(String, &str)> {
    let captures = UNDECORATE_REGEX.captures(source)?;
    let name = captures.get(1).unwrap().as_str();
    let module = captures.get(2).unwrap();
    let module = String::from_utf8(
        data_encoding::BASE32_NOPAD
            .decode(module.as_str().as_bytes())
            .unwrap(),
    )
    .unwrap();
    Some((module, name))
}

pub(crate) fn demangle_mod_names(source: &str, pad: bool) -> Cow<'_, str> {
    let Some((module, name)) = decompose_mangled_name(source) else {
        return std::borrow::Cow::Borrowed(source);
    };

    Cow::Owned(if pad {
        let original_len = source.len();
        format!(
            "{module:>len$}::{name}",
            len = original_len - 2 - name.len()
        )
    } else {
        format!("{module}::{name}")
    })
}

pub(crate) fn format_compose_error(e: ComposerError, composer: &Composer) -> String {
    let (source_name, source, offset) = match &e.source {
        naga_oil::compose::ErrSource::Module {
            name,
            offset,
            defs: _,
        } => {
            let source = composer
                .module_sets
                .get(name)
                .unwrap_or_else(|| {
                    panic!(
                        "while handling error could not find module {}: {:?}",
                        name, e
                    )
                })
                .sanitized_source
                .clone();
            (name, source, *offset)
        }
        naga_oil::compose::ErrSource::Constructing {
            source,
            path,
            offset,
        } => (path, source.clone(), *offset),
    };

    let source = " ".repeat(offset) + &source;

    match e.inner {
        ComposerErrorInner::WgslParseError(e) => {
            let wgsl_error = e.emit_to_string_with_path(&source, source_name);

            // Demangle first line that probably contains type but not in context, so no padding required
            let (first_line, other_lines) = wgsl_error.split_once('\n').unwrap();
            let first_line = demangle_mod_names(first_line, false);

            // Demangle anything else
            let other_lines = demangle_mod_names(other_lines, true);

            format!("wgsl parsing error: {}\n{}", first_line, other_lines)
        }
        ComposerErrorInner::GlslParseError(e) => format!("glsl parsing error(s): {:?}", e),
        ComposerErrorInner::ShaderValidationError(e) => format!(
            "failed to build a valid final module: {0}",
            e.emit_to_string(&source)
        ),
        _ => format!("{}", e),
    }
}
